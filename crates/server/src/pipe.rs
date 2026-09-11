//! The named pipe the server listens on beside its socket, and the whole of
//! what a sandboxed Windows session will have to ask through.
//!
//! **Why a pipe at all** ([ADR-0014](../../../docs/adr/0014-windows-sessions.md)).
//! It was an AppContainer that could not dial `127.0.0.1`: a session inside one
//! is refused the loopback interface, and the exemption is an elevated command
//! per machine that an unsigned per-user install cannot ask for — so what a
//! session asked through had to be something an identity could be *granted*
//! rather than routed to, and a named pipe is the one such thing this platform
//! has. A session runs as a local account now and can dial the loopback, so the
//! pipe is no longer the only way in. It stays because it is landed, harmless,
//! and the one transport no firewall on the human's machine has to agree with.

//!
//! **It is a listener like any other.** `axum::serve` takes anything
//! implementing its own [`axum::serve::Listener`] — an `Io` that reads and
//! writes, an `Addr`, an `accept` and a `local_addr` — so the pipe half is that
//! trait over [`tokio::net::windows::named_pipe`], and the router is served over
//! it exactly as it is served over the socket. One router, one database and one
//! Conversation-scoped namespace, reached two ways.
//!
//! **A pipe instance is the connection.** There is no listening socket here
//! that accepts and hands back another: an instance is created, waited on for a
//! client, and *is* what that client is speaking to once one arrives. So a
//! [`Listener`] always holds one created and waiting — [`Listener::accept`]
//! waits on the one it holds, makes the next before handing the connected one
//! over, and never leaves the name with no instance behind it, which a client
//! dialling in that window would be refused for with nothing wrong.
//!
//! **The name is the Data Directory's.** Two Verksteads on one machine against
//! two Data Directories open two pipes and neither disturbs the other. Two
//! against one Data Directory is what the TCP bind already refuses, and the
//! first instance is created as the *first* instance so that the pipe refuses
//! it the same way rather than quietly shadowing the server already there. It
//! is derived rather than configured: nothing outside reads the name except
//! through what a session is handed.
//!
//! **The descriptor is settled when the pipe is opened.** It grants the account
//! the server runs as, and beside it the one identity a session of this
//! installation runs as: the local account of Verkstead's own, which is the
//! same account for every Conversation (ADR-0014, *Amended: the Sandbox is an
//! account*). So [`Listener::open`] is handed that identity and every instance
//! it creates grants it.
//!
//! It used to be a set that was added to, and only because the identity was
//! not there yet: an AppContainer was made per Conversation as that
//! Conversation's first session started, an hour after the pipe was opened. One
//! account for the installation takes that whole mechanism away, which is a
//! simplification rather than a retargeting.
//!
//! **A machine with no account opens a pipe granting nobody.** Which is a
//! machine that starts no session either — a session with no account to run as
//! is refused before anything is started, see [`crate::sandbox::account`] — so
//! there is never anybody inside to be refused the pipe.
//!
//! **And the one thing that makes an account makes it while the server is
//! up.** The onboarding wizard's install run is where an account comes from now
//! (ADR-0016), and it runs an hour after this pipe was opened granting nobody.
//! So the grant is a value the listener re-reads rather than one it was built
//! with: [`hold_the_grant`] leaves the handle where that run can reach it, and
//! [`granted`] moves it.
//!
//! **Which really re-opens the pipe.** The descriptor belongs to the pipe
//! *object* rather than to an instance of it — it is the one the instance that
//! created the object was given, and every instance after that is handed the
//! object that is already there — so a wider descriptor on one more instance
//! would change nothing at all. What moves the grant is the object going and
//! coming back: the last instance closed and a first one created again. See
//! [`Listener::regranted`], which [`Listener::accept`] reaches the moment the
//! grant moves under it. A session started after that asks over the pipe with
//! nothing restarted.

use std::ffi::{OsStr, c_void};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::time::Duration;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::watch;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// How long an accept that failed waits before going round again.
///
/// The same second axum waits after an accept error on a socket, and for the
/// same reason: what fails here fails for a reason nothing in this process can
/// mend — a machine out of handles — and retrying it flat out is a runtime
/// spent on nothing.
const AGAIN: Duration = Duration::from_secs(1);

/// What Win32 puts in front of every pipe name. The API's, rather than
/// anybody's to type — see [`Listener::asked_through`].
const PREFIX: &str = r"\\.\pipe\";

/// What a listener without its waiting instance would be, said where the
/// invariant is read — see [`Listener::waiting`], which is empty for the one
/// instant a re-grant is closing the pipe and opening it again.
const HELD: &str = "a listener holds an instance except while it is being re-opened";

/// The named pipe a server keeping its Data Directory at `data_dir` listens on,
/// as Win32 names one: the spelling `CreateNamedPipeW` takes and `CreateFileW`
/// opens.
///
/// Every character after the prefix comes off the Data Directory, so this is
/// the same name every time for one directory and a different one for the next.
pub fn named(data_dir: &Path) -> String {
    format!("{PREFIX}{}", bare(data_dir))
}

/// What the pipe is called, with neither spelling's prefix on it.
///
/// Reachable from the rest of the crate for one other thing this machine has to
/// name after a Data Directory: what a Conversation's entries are held under
/// while its work lasts — see [`crate::sandbox::entries`]. Two Verksteads on
/// one machine keep those apart the way they keep their pipes apart, and this
/// is that fingerprint said once rather than twice.
pub(crate) fn bare(data_dir: &Path) -> String {
    // Through the resolved path rather than the one that was typed: `.` and the
    // absolute name of the same directory are one Data Directory, and two
    // servers pointed at it by those two spellings have to collide. Windows
    // hands back the on-disk casing, so two spellings differing only in case
    // resolve to one name without anything here lowering it. A directory that
    // will not resolve is one nothing has made yet — the caller makes it before
    // asking — and the name it was asked by is the honest answer.
    let settled = data_dir.canonicalize();
    let settled = settled.as_deref().unwrap_or(data_dir);

    format!("verkstead-{:016x}", fingerprint(settled))
}

/// `path` as one number, by FNV-1a over the way Windows itself spells it.
///
/// Written out rather than taken from a hashing crate or from the standard
/// library's own hasher: what this decides is the name of an object two
/// processes have to agree on, so it has to be the same number in a year's time
/// as it is today, and `DefaultHasher` promises exactly the opposite.
fn fingerprint(path: &Path) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET;

    for unit in path.as_os_str().encode_wide() {
        for byte in unit.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
    }

    hash
}

/// The pipe half of the server's listening: the name, who it is created
/// granting, and the instance waiting for the next client.
///
/// Handed to `axum::serve` beside the socket's own listener — see this module's
/// own documentation for why that is all it takes.
pub struct Listener {
    /// What the pipe is called, kept because every further instance is created
    /// by it and because it is what [`Listener::local_addr`] answers.
    name: String,

    /// The same name in the spelling a client is told it in — see
    /// [`Listener::asked_through`], which is the whole of what it is for.
    asked_through: String,

    /// The one identity each instance is created granting beside the account
    /// the server runs as, and nothing where there is none to grant.
    ///
    /// A **receiver** rather than a value because the identity can arrive
    /// inside a run: the install run makes the account the wizard's first step
    /// is missing, and says so here — which is what wakes an accept to open the
    /// pipe again. See [`granted`], and this module's own documentation.
    granting: watch::Receiver<Option<String>>,

    /// And the end that moves it, kept so that a caller can be handed one after
    /// the pipe is open — see [`Listener::regranting`].
    regranting: Regranting,

    /// The instance created and waiting for a client. There is always one — see
    /// this module's own documentation.
    ///
    /// **An `Option` for one instant and no other.** A pipe object's descriptor
    /// is the one the instance that *created* it was given, so moving the grant
    /// means closing the last instance and creating a first one again: this is
    /// empty between those two, and nowhere else — see [`Listener::regranted`].
    waiting: Option<NamedPipeServer>,
}

/// Who the pipe grants, as the thing that makes an account holds it.
///
/// A handle rather than a call into the listener, because the listener is
/// handed to `axum::serve` the moment it is open and nothing can reach it
/// again: what crosses is this, and what it does is say the identity. Re-opening
/// the pipe with it is the listener's own business — see
/// [`Listener::regranted`].
#[derive(Debug, Clone)]
pub struct Regranting(watch::Sender<Option<String>>);

impl Regranting {
    /// Grant `identity` from here on.
    ///
    /// Nothing is re-opened here. What this does is say so; the listener does
    /// the re-opening the moment it is next between clients, which is
    /// immediately — an accept that is waiting on one wakes for this.
    pub fn to(&self, identity: &str) {
        // A receiver that has gone is a server that has stopped serving, which
        // is nothing to report from here: what was going to ask over the pipe
        // was going to ask a process that is not there.
        let _ = self.0.send(Some(identity.to_owned()));
    }
}

/// The grant of the one pipe this process opened, left where whatever makes an
/// account while the server is up can reach it.
///
/// **Held rather than threaded**, the way `session_path` is — see
/// [`crate::sandbox::hold_session_path`]. The pipe is opened as the server
/// comes up and the install run is a press an hour later, and every router
/// between them would otherwise carry a parameter about a platform most of them
/// are not on.
static GRANTING: std::sync::RwLock<Option<Regranting>> = std::sync::RwLock::new(None);

/// Hold `regranting` for the rest of this run.
///
/// Called once as the server comes up, with the pipe it just opened — see
/// [`crate::run_on_keyed`].
pub fn hold_the_grant(regranting: Regranting) {
    *GRANTING.write().unwrap_or_else(|held| held.into_inner()) = Some(regranting);
}

/// And the account this Data Directory's sessions run as has just been made:
/// the pipe is opened again granting `identity`.
///
/// Nothing at all where no pipe was opened, which is every platform but this
/// one and a server whose listener is not this process's.
pub(crate) fn granted(identity: &str) {
    if let Some(regranting) = GRANTING
        .read()
        .unwrap_or_else(|held| held.into_inner())
        .as_ref()
    {
        regranting.to(identity);
    }
}

impl Listener {
    /// Open the pipe a server against `data_dir` listens on, granting
    /// `granting` beside the account the server runs as.
    ///
    /// `granting` is the identity every session of this installation runs as —
    /// the SID of the local account, resolved before the server comes up — and
    /// `None` on a machine that has not got one, which is every machine that is
    /// not a Windows one and every Windows one where the elevated verb has
    /// never been run. A pipe granting nobody is not a case to handle here: a
    /// session with no account is refused before anything is started.
    ///
    /// Refused where the name is already taken, which is a second server
    /// against one Data Directory: the first instance is created as the first
    /// instance, so the pipe answers that the way the socket answers a taken
    /// address.
    pub fn open(data_dir: &Path, granting: Option<&str>) -> io::Result<Listener> {
        let bare = bare(data_dir);
        let name = format!("{PREFIX}{bare}");
        let waiting = instance(&name, &Descriptor::granting(granting)?, true)?;

        // The sender goes to whoever is holding it and the receiver stays here:
        // what moves the grant is an account made inside this run — see
        // [`Listener::regranting`].
        let (moving, granting) = watch::channel(granting.map(str::to_owned));

        Ok(Listener {
            name,
            asked_through: format!("pipe://{bare}"),
            granting,
            regranting: Regranting(moving),
            waiting: Some(waiting),
        })
    }

    /// The handle that moves who this pipe grants, for whatever makes an
    /// account while the server is up — see [`hold_the_grant`], which is where
    /// the one caller leaves it.
    pub fn regranting(&self) -> Regranting {
        self.regranting.clone()
    }

    /// Open the pipe again, granting whoever it grants now.
    ///
    /// **A pipe object's descriptor is the one the instance that made it was
    /// given**, and every instance after that is handed the object that is
    /// already there: a further instance created with a wider descriptor is a
    /// handle on the old grant. So moving the grant is the object going and
    /// coming back — the last instance closed, and a *first* instance created
    /// again with the new descriptor. What a session dials next is that one.
    ///
    /// **Which wants nothing else holding the name.** A connection open at this
    /// moment is a pipe object that outlives the close, and the first-instance
    /// flag is refused for exactly that — so what is made then is an ordinary
    /// instance and the pipe goes on serving on the grant it had, said in the
    /// log because it is the one thing a caller cannot see. In practice there
    /// is nothing to hold it: the account being made is what a session is
    /// waiting for, so there is no session inside yet.
    ///
    /// Reached where the grant has moved and nowhere else, which is once in the
    /// life of a server that came up without an account and never on one that
    /// did.
    fn regranted(&mut self) -> io::Result<()> {
        let granting = self.granting.borrow_and_update().clone();
        let descriptor = Descriptor::granting(granting.as_deref())?;

        // Closed before the next is made, which is the whole of the mechanism —
        // and is why the field is an `Option`: for this instant the name has
        // nothing behind it, and a client dialling inside it is refused as it
        // would be a moment before the server came up.
        self.waiting = None;

        match instance(&self.name, &descriptor, true) {
            Ok(opened) => {
                self.waiting = Some(opened);
                Ok(())
            }

            Err(refused) => {
                tracing::warn!(
                    error = %refused,
                    "the named pipe could not be opened again granting the account that was \
                     just made — something is still connected to it, so it goes on granting \
                     what it did until Verkstead is started again",
                );

                self.waiting = Some(instance(&self.name, &descriptor, false)?);

                Ok(())
            }
        }
    }

    /// What the pipe is called as Win32 names one: the spelling every instance
    /// is created under, and what [`Listener::local_addr`] answers.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// And in the spelling a client is *told* it in: `pipe://<name>`, which is
    /// what the CLI's `--server` and `VERKSTEAD_SERVER` take, and what the
    /// startup line says.
    ///
    /// Windows' `\\.\pipe\` belongs to the API rather than to a human: this
    /// goes in a terminal and in an environment value, where backslashes are
    /// the shell's. So what travels is the name alone, and the end that dials
    /// puts the prefix back on — `crates/cli/src/pipe.rs`.
    pub fn asked_through(&self) -> &str {
        &self.asked_through
    }
}

impl axum::serve::Listener for Listener {
    type Io = NamedPipeServer;

    /// The pipe's name. There is no address at the far end of one — a client is
    /// a process on this machine and nothing else — so what a connection is
    /// labelled with is the name it came in on.
    type Addr = String;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            // Two things can happen to an instance that is waiting: a client
            // dials it, or the account it should have been granting turns up
            // and the pipe is opened again — see [`Listener::regranted`].
            // `connect` is cancel-safe, so the branch that loses has lost
            // nothing.
            //
            // And the client wins where both are ready, because the re-open
            // closes the instance it would have been answered on: a grant that
            // waits for the next accept is a moment later, and a connection
            // dropped for it is a request nobody answered.
            tokio::select! {
                biased;

                connected = self.waiting.as_ref().expect(HELD).connect() => {
                    if let Err(what) = connected {
                        went_wrong(&what).await;
                        continue;
                    }
                }

                moved = self.granting.changed() => {
                    // A sender that has gone is nobody left to move the grant,
                    // which is a listener that goes on granting what it has.
                    //
                    // And round again until there is an instance, rather than
                    // once: a re-open closes the one there was before it makes
                    // the next, so a failure here is a name with nothing behind
                    // it — which is the accept error the trait's own
                    // documentation describes, and is waited out the same way.
                    if moved.is_ok() {
                        while let Err(what) = self.regranted() {
                            went_wrong(&what).await;
                        }
                    }

                    continue;
                }
            }

            // The next instance before the connected one is handed over, so
            // that the name never stands with nothing listening behind it. An
            // instance that will not create is the accept error the trait's own
            // documentation describes: said, waited on, and gone round again
            // rather than an end to the server.
            //
            // Created granting whoever the pipe grants now, which is the
            // descriptor the object already has: a grant that moved moved it by
            // closing the object and making it again, so by here there is
            // nothing left for an instance to differ about — see
            // [`Listener::regranted`].
            let granting = self.granting.borrow_and_update().clone();

            let next = loop {
                match Descriptor::granting(granting.as_deref())
                    .and_then(|granting| instance(&self.name, &granting, false))
                {
                    Ok(next) => break next,
                    Err(what) => went_wrong(&what).await,
                }
            };

            return (self.waiting.replace(next).expect(HELD), self.name.clone());
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        Ok(self.name.clone())
    }
}

/// One instance of the pipe called `name`, created granting `granting`.
///
/// `first` is what refuses a second server against one Data Directory — see
/// [`Listener::open`] — so it is the opening instance's alone: every instance
/// behind it is one more on a name this process already holds.
fn instance(name: &str, granting: &Descriptor, first: bool) -> io::Result<NamedPipeServer> {
    let mut attributes = granting.attributes();

    // SAFETY: `attributes` is a valid `SECURITY_ATTRIBUTES` for the length of
    // the call, and the descriptor it points at is `granting`, which outlives
    // it. Everything else about the pipe is the default: duplex, byte mode, and
    // remote clients refused.
    //
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .create_with_security_attributes_raw(name, ptr::from_mut(&mut attributes).cast())
    }
}

/// An accept that failed: say so, wait, and let the caller go round again.
async fn went_wrong(what: &io::Error) {
    tracing::error!(error = %what, "accepting on the named pipe failed");

    tokio::time::sleep(AGAIN).await;
}

/// The security descriptor every instance of the pipe is created with.
///
/// Owned, because what `ConvertStringSecurityDescriptorToSecurityDescriptorW`
/// hands back is a block this process is to free — and it is read again at
/// every instance, so it is freed when the listener goes rather than after the
/// first one.
pub(crate) struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    /// A descriptor granting the account the server runs as, and `also` beside
    /// it — the one identity this installation's sessions run as, where there
    /// is one to name.
    ///
    /// Written as SDDL, which is Windows' own spelling of a descriptor and the
    /// one a person can read: `D:P` is a DACL and nothing inherited into it,
    /// `A` is an entry that allows, `GA` is everything — the server's own
    /// account needs it, because creating each further instance of the pipe is
    /// an access the descriptor either allows or refuses — and `GRGW` is what a
    /// client needs and no more.
    pub(crate) fn granting(also: Option<&str>) -> io::Result<Descriptor> {
        let mut sddl = format!("D:P(A;;GA;;;{})", the_server_runs_as()?);

        if let Some(identity) = also {
            sddl.push_str(&format!("(A;;GRGW;;;{identity})"));
        }

        let sddl = wide(&sddl);
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

        // SAFETY: the string is NUL-terminated and lives across the call, and
        // the descriptor is written only where the call reports success.
        let read = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                ptr::null_mut(),
            )
        };

        if read == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Descriptor(descriptor))
    }

    /// The descriptor as `CreateNamedPipeW` takes one.
    ///
    /// Made per call rather than held: it is a pointer to `self` and two
    /// numbers, and a struct holding a pointer to itself is not a thing to keep
    /// around.
    pub(crate) fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0,
            // Nothing this process starts is to inherit the pipe: a session is
            // handed the *name*, and opens it as itself.
            bInheritHandle: 0,
        }
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: what is freed is what the conversion above allocated, and
        // this is the only owner of it.
        unsafe { LocalFree(self.0) };
    }
}

// SAFETY: a descriptor is a block of memory rather than anything with an
// affinity to the thread that made it; the pipe reads it while an instance is
// being created and never afterwards. Said because the listener holding one is
// handed to a runtime that moves it between threads.
unsafe impl Send for Descriptor {}
unsafe impl Sync for Descriptor {}

/// The account the server runs as, as a security descriptor names one: the
/// string form of this process's token user.
fn the_server_runs_as() -> io::Result<String> {
    let mut token: HANDLE = ptr::null_mut();

    // SAFETY: the handle is written only where the call reports success.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let who = user_of(token);

    // SAFETY: the token was opened above and is not used again.
    unsafe { CloseHandle(token) };

    who
}

/// The user of `token`, in the string form a descriptor spells an identity in.
fn user_of(token: HANDLE) -> io::Result<String> {
    let mut needed = 0;

    // How much room the answer wants. It fails for being given none, which is
    // the question rather than a failure, so what is read is the length it
    // asked for rather than what it returned.
    //
    // SAFETY: a null buffer of length zero is what asking the length is.
    unsafe { GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed) };

    if needed == 0 {
        return Err(io::Error::last_os_error());
    }

    // In words rather than bytes, because what lands here is a struct holding a
    // pointer and a `Vec<u8>` is not aligned for one.
    let mut buffer = vec![0u64; (needed as usize).div_ceil(size_of::<u64>())];
    let into = buffer.as_mut_ptr().cast::<c_void>();

    // SAFETY: the buffer is the length the call just asked for.
    if unsafe { GetTokenInformation(token, TokenUser, into, needed, &mut needed) } == 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: what a `TokenUser` question writes is a `TOKEN_USER`, and the SID
    // it points at is inside the same buffer, which outlives the read below.
    let user = unsafe { &*into.cast::<TOKEN_USER>() };

    named_sid(user.User.Sid)
}

/// `sid` in the string form a security descriptor spells an identity in —
/// `S-1-5-21-…`.
fn named_sid(sid: PSID) -> io::Result<String> {
    let mut written = ptr::null_mut();

    // SAFETY: the SID is one the caller read out of a token, and the string is
    // written only where the call reports success.
    if unsafe { ConvertSidToStringSidW(sid, &mut written) } == 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: what the call wrote is a NUL-terminated string this process is to
    // free, and it is freed the moment it has been read.
    let named = unsafe { from_wide(written) };
    unsafe { LocalFree(written.cast()) };

    Ok(named)
}

/// `text` as Windows takes a string: UTF-16, and NUL-terminated.
fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

/// And back: what Windows wrote at `from`, up to its NUL.
///
/// # Safety
///
/// `from` points at a NUL-terminated UTF-16 string that is not written to while
/// this reads it.
unsafe fn from_wide(from: *const u16) -> String {
    let mut length = 0;

    // SAFETY: the caller's own — the string is terminated, so this stops.
    while unsafe { *from.add(length) } != 0 {
        length += 1;
    }

    // SAFETY: the length was just measured off the same string.
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(from, length) })
}

#[cfg(test)]
mod tests {
    use std::os::windows::io::AsRawHandle;

    use tokio::net::windows::named_pipe::ClientOptions;
    use windows_sys::Win32::Foundation::HANDLE;

    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertStringSidToSidW,
        SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{DACL_SECURITY_INFORMATION, GetKernelObjectSecurity};

    use super::*;

    /// An identity of the shape a session account's is: a local account of this
    /// machine's, which is the one thing beside the server's own that a pipe
    /// grants.
    ///
    /// Nobody's, and it does not have to be: a descriptor names identities and
    /// never asks the machine whether it has heard of them.
    const THE_SESSION_ACCOUNT: &str = "S-1-5-21-1001-1002-1003-1004";

    /// Two Data Directories are two pipes, and one Data Directory is one pipe
    /// however many times it is asked about.
    ///
    /// Which is the whole of what the name is for: two Verksteads on one machine
    /// must not land on one name, and the two servers that *are* one Verkstead's
    /// Data Directory twice over must.
    #[test]
    fn a_pipe_is_named_after_its_data_directory() {
        let one = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();

        assert_ne!(named(one.path()), named(other.path()));
        assert_eq!(named(one.path()), named(one.path()));
    }

    /// And the two spellings of one directory are one name, because they are one
    /// Data Directory.
    #[test]
    fn a_data_directory_asked_for_two_ways_is_one_pipe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("under")).unwrap();
        let roundabout = dir.path().join("under").join("..");

        assert_eq!(named(&roundabout), named(dir.path()));
    }

    /// The two spellings are one pipe: what a client is told is the name Win32
    /// was given with its prefix taken off, and the end that dials puts it
    /// back on.
    #[tokio::test]
    async fn what_a_client_is_told_is_the_name_without_the_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let listener = Listener::open(dir.path(), None).unwrap();

        let bare = listener
            .name()
            .strip_prefix(PREFIX)
            .expect("a Win32 pipe name starts with the prefix");

        assert_eq!(listener.asked_through(), format!("pipe://{bare}"));
        assert_eq!(listener.name(), named(dir.path()));
    }

    /// A second server against one Data Directory is refused by the pipe, the
    /// way a second server on one address is refused by the socket.
    #[tokio::test]
    async fn a_second_server_on_one_data_directory_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let _first = Listener::open(dir.path(), None).expect("nothing holds this name yet");

        let second = Listener::open(dir.path(), None);

        assert!(
            second.is_err(),
            "a second listener on one Data Directory should be refused the name"
        );
    }

    /// What a pipe opened granting nobody is created granting: the account the
    /// server runs as, and nothing else at all.
    ///
    /// Read off the pipe itself rather than proved by connecting as somebody
    /// else — a runner has one account, so the descriptor is what there is to
    /// ask.
    #[tokio::test]
    async fn the_pipe_grants_the_account_the_server_runs_as_and_nothing_wider() {
        let dir = tempfile::tempdir().unwrap();
        let listener = Listener::open(dir.path(), None).unwrap();

        let granted = granted_by(&listener);

        assert_eq!(granted.len(), 1, "one entry and no other");
        assert_eq!(granted[0], the_server_runs_as().unwrap());
    }

    /// And the session account beside it, where the server was opened knowing
    /// one — which is every Windows machine the elevated verb has been run on.
    #[tokio::test]
    async fn the_session_account_is_granted_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let listener = Listener::open(dir.path(), Some(THE_SESSION_ACCOUNT)).unwrap();

        assert_eq!(
            granted_by(&listener),
            vec![
                the_server_runs_as().unwrap(),
                THE_SESSION_ACCOUNT.to_owned()
            ],
            "the account the server runs as, and the one its sessions run as"
        );
    }

    /// And so is every instance made behind a connection, rather than only the
    /// one the server opened with.
    ///
    /// Which is what makes the grant the *pipe's* rather than the first
    /// client's: a session asking an hour in dials the instance made in place
    /// of whichever one was taken by the ask before it.
    #[tokio::test]
    async fn the_instance_made_behind_a_connection_grants_it_too() {
        use axum::serve::Listener as _;

        let dir = tempfile::tempdir().unwrap();
        let mut listener = Listener::open(dir.path(), Some(THE_SESSION_ACCOUNT)).unwrap();

        let dialled = ClientOptions::new()
            .open(listener.name())
            .expect("the account this test runs as to be granted its own pipe");
        let (connected, _) = listener.accept().await;

        assert_eq!(
            granted_by(&listener),
            vec![
                the_server_runs_as().unwrap(),
                THE_SESSION_ACCOUNT.to_owned()
            ],
            "the instance made behind that connection should grant it too"
        );

        drop(dialled);
        drop(connected);
    }

    /// And a pipe opened granting nobody grants the account the moment there is
    /// one, which is the wizard's install run having made it.
    ///
    /// **Which has to be a re-open rather than one more instance.** A pipe
    /// object's descriptor is the one the instance that made it was given, so a
    /// further instance created with the account named in it would read back
    /// exactly as this reads before the re-grant — which is what this asserts
    /// against. See [`Listener::regranted`], and this module's own
    /// documentation.
    #[tokio::test]
    async fn a_pipe_granting_nobody_grants_an_account_made_since() {
        let dir = tempfile::tempdir().unwrap();
        let mut listener = Listener::open(dir.path(), None).unwrap();

        assert_eq!(
            granted_by(&listener),
            vec![the_server_runs_as().unwrap()],
            "a server that came up before the verb was run grants nobody else",
        );

        listener.regranting().to(THE_SESSION_ACCOUNT);
        listener.regranted().expect("the pipe to be opened again");

        assert_eq!(
            granted_by(&listener),
            vec![
                the_server_runs_as().unwrap(),
                THE_SESSION_ACCOUNT.to_owned()
            ],
            "the instance a session would dial should grant the account just made",
        );
    }

    /// And every instance behind it grants it too, rather than only the one the
    /// re-grant made.
    #[tokio::test]
    async fn an_instance_made_after_a_regrant_grants_it_too() {
        use axum::serve::Listener as _;

        let dir = tempfile::tempdir().unwrap();
        let mut listener = Listener::open(dir.path(), None).unwrap();

        listener.regranting().to(THE_SESSION_ACCOUNT);
        listener.regranted().expect("the pipe to be opened again");

        let dialled = ClientOptions::new()
            .open(listener.name())
            .expect("the account this test runs as to be granted its own pipe");
        let (connected, _) = listener.accept().await;

        assert_eq!(
            granted_by(&listener),
            vec![
                the_server_runs_as().unwrap(),
                THE_SESSION_ACCOUNT.to_owned()
            ],
            "the instance made behind that connection is made with the grant that moved",
        );

        drop(dialled);
        drop(connected);
    }

    /// Who `listener`'s pipe lets through, in the order its descriptor says it,
    /// asked of the pipe itself.
    fn granted_by(listener: &Listener) -> Vec<String> {
        let dacl = dacl_of(listener.waiting.as_ref().expect(HELD).as_raw_handle() as HANDLE);

        dacl.split('(')
            .skip(1)
            .map(|entry| {
                let entry = entry.trim_end_matches(')');
                assert!(
                    entry.starts_with("A;"),
                    "an entry that allows, got ({entry})"
                );

                identity(entry.rsplit(';').next().unwrap())
            })
            .collect()
    }

    /// The DACL on `handle`, as SDDL — the spelling a descriptor is written
    /// down in, which is the one this reads back.
    fn dacl_of(handle: HANDLE) -> String {
        let mut needed = 0;

        // The room the answer wants, which is what asking for none is.
        //
        // SAFETY: the handle is the caller's, held across this whole function.
        unsafe {
            GetKernelObjectSecurity(
                handle,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                0,
                &mut needed,
            )
        };
        assert_ne!(needed, 0, "{}", io::Error::last_os_error());

        // In words rather than bytes: a descriptor holds pointers.
        let mut buffer = vec![0u64; (needed as usize).div_ceil(size_of::<u64>())];
        let into = buffer.as_mut_ptr().cast::<c_void>();

        // SAFETY: the buffer is the length the call just asked for.
        let read = unsafe {
            GetKernelObjectSecurity(handle, DACL_SECURITY_INFORMATION, into, needed, &mut needed)
        };
        assert_ne!(read, 0, "{}", io::Error::last_os_error());

        let mut written = ptr::null_mut();

        // SAFETY: the descriptor is the one just read, and the string is
        // written only where the call reports success.
        let said = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                into,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut written,
                ptr::null_mut(),
            )
        };
        assert_ne!(said, 0, "{}", io::Error::last_os_error());

        // SAFETY: what the call wrote is a NUL-terminated string this process
        // is to free, and it is freed the moment it has been read.
        let dacl = unsafe { from_wide(written) };
        unsafe { LocalFree(written.cast()) };

        dacl
    }

    /// Whom an entry names, as `S-1-…`.
    ///
    /// Written back rather than compared as it stands: a descriptor read out of
    /// Windows spells a well-known account as an alias — `LA` for the machine's
    /// own administrator — so what came back goes through the same conversion
    /// the descriptor was written from before anything is compared.
    fn identity(said: &str) -> String {
        let said = wide(said);
        let mut sid = ptr::null_mut();

        // SAFETY: the string is NUL-terminated and lives across the call, and
        // the SID is written only where the call reports success.
        let read = unsafe { ConvertStringSidToSidW(said.as_ptr(), &mut sid) };
        assert_ne!(read, 0, "{}", io::Error::last_os_error());

        let identity = named_sid(sid).unwrap();

        // SAFETY: what the call wrote is this process's to free, and it is
        // freed the moment it has been read.
        unsafe { LocalFree(sid.cast()) };

        identity
    }
}
