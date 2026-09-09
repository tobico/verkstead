//! The named pipe the server listens on beside its socket, and the whole of
//! what a sandboxed Windows session will have to ask through.
//!
//! **Why a pipe at all** ([ADR-0014](../../../docs/adr/0014-windows-sessions.md)).
//! An AppContainer is refused the loopback interface, and the exemption is an
//! elevated command per machine that an unsigned per-user install cannot ask
//! for. So a session inside one cannot dial `127.0.0.1`, and what it asks
//! through has to be something the container's own identity can be *granted*
//! instead of routed to. A named pipe is that, and it is the one such thing
//! this platform has.
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
//! **The descriptor is a set that is added to.** The pipe is created granting
//! the account the server runs as and nothing wider, and every identity beside
//! that one is an AppContainer that was not on the machine when the pipe was
//! opened: the server opens the pipe at startup, and a container is made per
//! Conversation as that Conversation's first session starts. So what a pipe is
//! opened against is a [`Grants`] rather than a list — a set each container
//! puts its identity into as it is made and takes it out of as it goes — and
//! putting one in writes the new descriptor onto the instance standing under
//! the name there and then, as well as on to every instance made after it.
//! That is the whole of how a session started an hour after the server is able
//! to open this at all.

use std::ffi::{OsStr, c_void};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetTokenInformation, PSECURITY_DESCRIPTOR, PSID,
    SECURITY_ATTRIBUTES, SetKernelObjectSecurity, TOKEN_QUERY, TOKEN_USER, TokenUser,
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
/// name after a Data Directory: the AppContainer a Conversation's sessions run
/// inside — see [`crate::sandbox::container::Container::for_conversation`].
/// Two Verksteads on one machine keep their containers apart the way they keep
/// their pipes apart, and this is that fingerprint said once rather than twice.
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

/// The identities a pipe grants beside the account the server runs as, and the
/// whole of how one that was not on the machine when the pipe was opened comes
/// to be granted on it.
///
/// **Because the pipe is opened before there is anything to grant.** The server
/// opens it at startup; the identity a session runs under is an AppContainer of
/// its Conversation's own, made as that Conversation's first session starts.
/// So the identities are a set that is added to rather than an argument, and
/// what puts one in is the container itself — see
/// [`crate::sandbox::container::Container`], which does it as it is made and
/// undoes it as it goes.
///
/// **An identity put in reaches the pipe that is already open.** A `Grants`
/// holds the instance every pipe open on it is standing behind, so adding one
/// writes the new descriptor onto those instances there and then — and every
/// instance made after it is created from the set as it stands. Both, rather
/// than either: which of the two Windows checks a client's open against is not
/// a thing to depend on, and a session refused the pipe it was given is a
/// session that cannot ask at all.
///
/// Cheap to clone and shared by every clone. The server's own is
/// [`Grants::of_this_process`], which is the one every container puts itself
/// into; [`Grants::none`] is one of a caller's own, which is what a test opens
/// a pipe against so that what it asks about is its own identities and nobody
/// else's.
#[derive(Clone)]
pub struct Grants(Arc<Mutex<Granted>>);

/// What a [`Grants`] holds: who is granted, and what to write that onto.
///
/// One lock over both halves, because the two are written together — an
/// identity put in is written onto the instance standing in the same breath,
/// and an instance is created from the identities as they stand. Apart, the two
/// would race into a pipe standing under the name granting a set nobody holds.
#[derive(Default)]
struct Granted {
    /// The identities themselves, as a security descriptor spells one: the
    /// string form of a SID.
    identities: Vec<String>,

    /// The instance each pipe open on these grants is waiting for a client on
    /// right now — and nothing at all where none is, which is every container
    /// made with no server behind it.
    ///
    /// A list rather than the one a running Verkstead has, because a set of
    /// grants is a set of identities rather than a pipe's own property: the
    /// suite stands two servers up against one process's containers, and an
    /// identity has to reach both of their pipes or the second one stood up
    /// would quietly take the first one's sessions away.
    ///
    /// Borrowed rather than owned: what holds an instance is a [`Listener`],
    /// which takes its own out of here as it replaces it and as it goes — see
    /// [`Grants::standing`] and the listener's [`Drop`] — so a handle that is
    /// here is a handle that is open.
    ///
    /// As numbers rather than as `HANDLE`, which the bindings spell as a raw
    /// pointer: what these are is the operating system's own handles rather
    /// than addresses of anything, and a pointer here would make the whole of
    /// [`Listener::accept`] un-`Send` — which the trait it implements requires
    /// — for the sake of a spelling.
    waiting: Vec<usize>,
}

/// The grants of this process's own pipe: the set every AppContainer made here
/// puts its identity into, and the one the server opens its pipe against.
///
/// A static for the reason the profiles themselves are one — see
/// [`crate::sandbox::container`]: there is one pipe per process and one set of
/// containers per process, and a container made deep inside a session start has
/// no other way of reaching either.
static CONTAINERS: LazyLock<Grants> = LazyLock::new(Grants::none);

impl Grants {
    /// The grants of this process's own pipe — see [`CONTAINERS`].
    pub fn of_this_process() -> Grants {
        CONTAINERS.clone()
    }

    /// And a set of a caller's own with nothing in it, which is a pipe granting
    /// the account the server runs as and nobody else.
    pub fn none() -> Grants {
        Grants(Arc::new(Mutex::new(Granted::default())))
    }

    /// One more identity granted: written onto the pipe standing under the name
    /// right now, and remembered for every instance made after it.
    ///
    /// **It can refuse**, and a caller that cannot grant an identity has made a
    /// container nothing inside can ask through — which is a session refused
    /// rather than a session started behind a transport it cannot open
    /// (ADR-0014, Q18). What did not go on is not remembered either: a set
    /// holding an identity the pipe never got would be a lie told to every
    /// instance after it.
    pub fn to(&self, identity: &str) -> io::Result<()> {
        let mut granted = self.held();

        if granted.identities.iter().any(|held| held == identity) {
            return Ok(());
        }

        granted.identities.push(identity.to_owned());

        written(&granted).inspect_err(|_| {
            granted.identities.pop();
        })
    }

    /// And one no longer granted, which is a container that has gone.
    ///
    /// Nothing is reported and nothing can be done about it: what lets go of a
    /// container is a `Drop`, which has nowhere to hand a refusal to. What a
    /// pipe that would not take it back goes on granting is a SID whose profile
    /// has been deleted, which resolves to nobody.
    pub fn no_longer(&self, identity: &str) {
        let mut granted = self.held();

        granted.identities.retain(|held| held != identity);

        if let Err(what) = written(&granted) {
            tracing::warn!(
                identity,
                error = %what,
                "the pipe would not take back the identity of a container that has gone"
            );
        }
    }

    /// The next instance of the pipe called `name`, created granting what this
    /// holds — and taken as the instance standing under that name, in place of
    /// `instead_of` where the caller had one already, so that an identity
    /// arriving after it is written onto this one.
    ///
    /// Under the lock for the whole of it, which is what stops an identity
    /// arriving mid-way from landing in neither the instance being created nor
    /// the one it replaces.
    fn standing(
        &self,
        name: &str,
        first: bool,
        instead_of: Option<usize>,
    ) -> io::Result<NamedPipeServer> {
        let mut granted = self.held();
        let made = instance(name, &Descriptor::granting(&granted.identities)?, first)?;

        // Taken out only where the new one was made, so that a listener whose
        // next instance would not create goes on holding the one it has and
        // goes round again — see [`Listener::accept`].
        if let Some(gone) = instead_of {
            granted.waiting.retain(|standing| *standing != gone);
        }

        granted.waiting.push(handle_of(&made));

        Ok(made)
    }

    /// And `was` standing no longer, which is what a listener leaves behind as
    /// it goes: a handle here is one that is open, and a closed one is a handle
    /// Windows is free to hand to something else entirely.
    fn nothing_standing(&self, was: usize) {
        self.held().waiting.retain(|standing| *standing != was);
    }

    /// What is held, through a lock that a panicking holder does not take the
    /// grants away with.
    fn held(&self) -> MutexGuard<'_, Granted> {
        self.0.lock().unwrap_or_else(|held| held.into_inner())
    }
}

/// What `granted` says, written onto the instance each pipe open on it is
/// standing behind — and nothing at all where none is open.
///
/// **Onto the instance rather than into a new one.** Replacing what is standing
/// would leave the name momentarily behind two instances, and a client dialling
/// in that window could land on the one that was on its way out and be refused
/// for nothing at all. A descriptor written onto the handle is the same change
/// with no such window.
fn written(granted: &Granted) -> io::Result<()> {
    if granted.waiting.is_empty() {
        return Ok(());
    }

    let descriptor = Descriptor::granting(&granted.identities)?;

    for waiting in &granted.waiting {
        // SAFETY: the handle is an instance a listener is holding — see
        // [`Granted::waiting`] — and the descriptor is read for the length of
        // the call and freed after the loop.
        let written = unsafe {
            SetKernelObjectSecurity(*waiting as HANDLE, DACL_SECURITY_INFORMATION, descriptor.0)
        };

        if written == 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

/// The handle `pipe` is, as [`Granted::waiting`] keeps one.
fn handle_of(pipe: &NamedPipeServer) -> usize {
    pipe.as_raw_handle() as usize
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

    /// Who each instance is created granting. Held for the listener's whole
    /// life: an instance is made per connection, and each one is made granting
    /// the set as it stands by then — see [`Grants`].
    granting: Grants,

    /// The instance created and waiting for a client. There is always one — see
    /// this module's own documentation.
    waiting: NamedPipeServer,
}

impl Listener {
    /// Open the pipe a server against `data_dir` listens on, granting
    /// `granting` beside the account the server runs as.
    ///
    /// The set rather than the identities in it: what a session runs under is a
    /// container made long after this, and a [`Grants`] is what carries one of
    /// those to a pipe that is already open. The server's own is
    /// [`Grants::of_this_process`].
    ///
    /// Refused where the name is already taken, which is a second server
    /// against one Data Directory: the first instance is created as the first
    /// instance, so the pipe answers that the way the socket answers a taken
    /// address.
    pub fn open(data_dir: &Path, granting: &Grants) -> io::Result<Listener> {
        let bare = bare(data_dir);
        let name = format!("{PREFIX}{bare}");
        let waiting = granting.standing(&name, true, None)?;

        Ok(Listener {
            name,
            asked_through: format!("pipe://{bare}"),
            granting: granting.clone(),
            waiting,
        })
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

impl Drop for Listener {
    /// The grants stop pointing at an instance nothing holds any more.
    ///
    /// A handle in [`Granted::waiting`] is one a descriptor may be written onto
    /// at any moment, and this is the last moment at which the instance it
    /// names is still open: the field is dropped after this runs, and a handle
    /// Windows has taken back is one it is free to hand to something else
    /// entirely.
    fn drop(&mut self) {
        self.granting.nothing_standing(handle_of(&self.waiting));
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
            if let Err(what) = self.waiting.connect().await {
                went_wrong(&what).await;
                continue;
            }

            // The next instance before the connected one is handed over, so
            // that the name never stands with nothing listening behind it. An
            // instance that will not create is the accept error the trait's own
            // documentation describes: said, waited on, and gone round again
            // rather than an end to the server.
            //
            // Created granting whoever is in the set by now rather than
            // whoever was in it when the server started, which is what makes a
            // Conversation that began an hour ago able to open this — see
            // [`Grants`].
            let standing = handle_of(&self.waiting);

            let next = loop {
                match self.granting.standing(&self.name, false, Some(standing)) {
                    Ok(next) => break next,
                    Err(what) => went_wrong(&what).await,
                }
            };

            return (
                std::mem::replace(&mut self.waiting, next),
                self.name.clone(),
            );
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
    // `write_dac` is the one thing asked for beyond that, and it is what lets
    // an identity granted later be written onto this instance rather than only
    // onto the next one — see [`written`]. A handle's access is settled when it
    // is opened, so it has to be asked for here; the descriptor grants the
    // account the server runs as everything, which includes it.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .write_dac(true)
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
    /// it — every identity a container has put into the pipe's [`Grants`], in
    /// the order they were put there.
    ///
    /// Written as SDDL, which is Windows' own spelling of a descriptor and the
    /// one a person can read: `D:P` is a DACL and nothing inherited into it,
    /// `A` is an entry that allows, `GA` is everything — the server's own
    /// account needs it, because creating each further instance of the pipe and
    /// writing this onto one are both accesses the descriptor either allows or
    /// refuses — and `GRGW` is what a client needs and no more.
    pub(crate) fn granting(also: &[String]) -> io::Result<Descriptor> {
        let mut sddl = format!("D:P(A;;GA;;;{})", the_server_runs_as()?);

        for identity in also {
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
    use tokio::net::windows::named_pipe::ClientOptions;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertStringSidToSidW,
        SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{DACL_SECURITY_INFORMATION, GetKernelObjectSecurity};

    use super::*;

    /// An identity of the shape the container stage will pass: an AppContainer
    /// SID, which is the one thing that would let a session inside one open the
    /// pipe at all.
    ///
    /// Nobody's, and it does not have to be: a descriptor names identities and
    /// never asks the machine whether it has heard of them.
    const A_CONTAINER: &str = "S-1-15-2-1001-1002-1003-1004-1005-1006-1007-1008";

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
        let listener = Listener::open(dir.path(), &Grants::none()).unwrap();

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
        let _first =
            Listener::open(dir.path(), &Grants::none()).expect("nothing holds this name yet");

        let second = Listener::open(dir.path(), &Grants::none());

        assert!(
            second.is_err(),
            "a second listener on one Data Directory should be refused the name"
        );
    }

    /// What the pipe is created granting: the account the server runs as, and
    /// nothing else at all.
    ///
    /// Read off the pipe itself rather than proved by connecting as somebody
    /// else — a runner has one account, so the descriptor is what there is to
    /// ask.
    #[tokio::test]
    async fn the_pipe_grants_the_account_the_server_runs_as_and_nothing_wider() {
        let dir = tempfile::tempdir().unwrap();
        let listener = Listener::open(dir.path(), &Grants::none()).unwrap();

        let granted = granted_by(&listener);

        assert_eq!(granted.len(), 1, "one entry and no other");
        assert_eq!(granted[0], the_server_runs_as().unwrap());
    }

    /// And an identity already in the grants when the pipe is opened.
    #[tokio::test]
    async fn a_further_identity_is_granted_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let grants = Grants::none();
        grants
            .to(A_CONTAINER)
            .expect("no pipe is open on these yet");

        let listener = Listener::open(dir.path(), &grants).unwrap();

        assert_eq!(
            granted_by(&listener),
            vec![the_server_runs_as().unwrap(), A_CONTAINER.to_owned()],
            "the account the server runs as, and the identity it was given"
        );
    }

    /// And one that arrives *after* the pipe is open, which is every container
    /// there will ever be: the server opens its pipe at startup and a
    /// Conversation's profile is made as its first session starts.
    ///
    /// Asked of the instance standing under the name rather than of the next
    /// one, because that is the instance a client dialling this second would
    /// land on.
    #[tokio::test]
    async fn an_identity_granted_after_the_pipe_was_opened_reaches_it() {
        let dir = tempfile::tempdir().unwrap();
        let grants = Grants::none();
        let listener = Listener::open(dir.path(), &grants).unwrap();

        assert_eq!(
            granted_by(&listener).len(),
            1,
            "nothing is granted beside the account until something asks for it"
        );

        grants
            .to(A_CONTAINER)
            .expect("an identity to be grantable on a pipe that is already open");

        assert_eq!(
            granted_by(&listener),
            vec![the_server_runs_as().unwrap(), A_CONTAINER.to_owned()],
            "the identity should have been written onto the instance standing"
        );
    }

    /// And it goes again with the container that put it there — a pipe granting
    /// a profile that has been deleted grants a SID that resolves to nobody.
    #[tokio::test]
    async fn an_identity_taken_back_is_no_longer_granted() {
        let dir = tempfile::tempdir().unwrap();
        let grants = Grants::none();
        let listener = Listener::open(dir.path(), &grants).unwrap();

        grants.to(A_CONTAINER).unwrap();
        grants.no_longer(A_CONTAINER);

        assert_eq!(
            granted_by(&listener),
            vec![the_server_runs_as().unwrap()],
            "the account the server runs as, and nobody else again"
        );
    }

    /// And it reaches every pipe open on the set rather than the last one
    /// opened, which is what the suite stands two servers up on.
    #[tokio::test]
    async fn an_identity_reaches_every_pipe_open_on_the_grants() {
        let one = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let grants = Grants::none();

        let first = Listener::open(one.path(), &grants).unwrap();
        let second = Listener::open(other.path(), &grants).unwrap();

        grants.to(A_CONTAINER).unwrap();

        for listener in [&first, &second] {
            assert_eq!(
                granted_by(listener),
                vec![the_server_runs_as().unwrap(), A_CONTAINER.to_owned()],
                "both pipes should grant it, and {} did not",
                listener.name()
            );
        }
    }

    /// And a pipe that has gone is not one anything is written onto — which is
    /// the whole reason a listener takes its instance back out of the set.
    #[tokio::test]
    async fn a_pipe_that_has_gone_is_no_longer_written_onto() {
        let one = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let grants = Grants::none();

        drop(Listener::open(one.path(), &grants).unwrap());
        let standing = Listener::open(other.path(), &grants).unwrap();

        grants
            .to(A_CONTAINER)
            .expect("a listener that has gone should not be written onto at all");

        assert_eq!(granted_by(&standing).len(), 2);
    }

    /// And the instance made behind a connection is made granting the set as it
    /// stands, rather than as it stood when the server started.
    ///
    /// Which is the other half of the same fact: a client that connects takes
    /// the standing instance with it, and what a session asking a moment later
    /// dials is the one made in its place.
    #[tokio::test]
    async fn the_instance_made_behind_a_connection_grants_what_the_set_holds() {
        use axum::serve::Listener as _;

        let dir = tempfile::tempdir().unwrap();
        let grants = Grants::none();
        let mut listener = Listener::open(dir.path(), &grants).unwrap();

        grants.to(A_CONTAINER).unwrap();

        let dialled = ClientOptions::new()
            .open(listener.name())
            .expect("the account this test runs as to be granted its own pipe");
        let (connected, _) = listener.accept().await;

        assert_eq!(
            granted_by(&listener),
            vec![the_server_runs_as().unwrap(), A_CONTAINER.to_owned()],
            "the instance made behind that connection should grant it too"
        );

        drop(dialled);
        drop(connected);
    }

    /// Who `listener`'s pipe lets through, in the order its descriptor says it,
    /// asked of the pipe itself.
    fn granted_by(listener: &Listener) -> Vec<String> {
        let dacl = dacl_of(handle_of(&listener.waiting) as HANDLE);

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
