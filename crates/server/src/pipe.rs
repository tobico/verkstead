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
//! **The descriptor is an argument.** The pipe is created granting the account
//! the server runs as and nothing wider, and it takes a further identity beside
//! that one — the seam the container stage fills, and the whole reason the
//! descriptor is decided here rather than left to whatever the platform would
//! have put on the object.

use std::ffi::{OsStr, c_void};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::time::Duration;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
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
fn bare(data_dir: &Path) -> String {
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

/// The pipe half of the server's listening: the name, what it is created
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

    /// What each instance is created with. Held for the listener's whole life:
    /// an instance is made per connection, and each one is made granting this.
    granting: Descriptor,

    /// The instance created and waiting for a client. There is always one — see
    /// this module's own documentation.
    waiting: NamedPipeServer,
}

impl Listener {
    /// Open the pipe a server against `data_dir` listens on.
    ///
    /// `also` is one further identity granted beside the account the server runs
    /// as, as Windows writes an identity in a security descriptor: the string
    /// form of a SID. Nothing passes one yet — it is the seam the container
    /// stage fills with the identity its sessions run under, which is the only
    /// way a process in an AppContainer could open this at all.
    ///
    /// Refused where the name is already taken, which is a second server
    /// against one Data Directory: the first instance is created as the first
    /// instance, so the pipe answers that the way the socket answers a taken
    /// address.
    pub fn open(data_dir: &Path, also: Option<&str>) -> io::Result<Listener> {
        let bare = bare(data_dir);
        let name = format!("{PREFIX}{bare}");
        let granting = Descriptor::granting(also)?;
        let waiting = instance(&name, &granting, true)?;

        Ok(Listener {
            name,
            asked_through: format!("pipe://{bare}"),
            granting,
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
            let next = loop {
                match instance(&self.name, &self.granting, false) {
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
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    /// A descriptor granting the account the server runs as, and `also` beside
    /// it where there is one.
    ///
    /// Written as SDDL, which is Windows' own spelling of a descriptor and the
    /// one a person can read: `D:P` is a DACL and nothing inherited into it,
    /// `A` is an entry that allows, `GA` is everything — the server's own
    /// account needs it, because creating each further instance of the pipe is
    /// itself an access the descriptor either allows or refuses — and `GRGW` is
    /// what a client needs and no more.
    fn granting(also: Option<&str>) -> io::Result<Descriptor> {
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
    fn attributes(&self) -> SECURITY_ATTRIBUTES {
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

    /// What the pipe is created granting: the account the server runs as, and
    /// nothing else at all.
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

    /// And the further identity the caller may name, which is the seam the
    /// container stage fills.
    #[tokio::test]
    async fn a_further_identity_is_granted_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let listener = Listener::open(dir.path(), Some(A_CONTAINER)).unwrap();

        let granted = granted_by(&listener);

        assert_eq!(
            granted,
            vec![the_server_runs_as().unwrap(), A_CONTAINER.to_owned()],
            "the account the server runs as, and the identity it was given"
        );
    }

    /// Who `listener`'s pipe lets through, in the order its descriptor says it,
    /// asked of the pipe itself.
    fn granted_by(listener: &Listener) -> Vec<String> {
        let dacl = dacl_of(listener.waiting.as_raw_handle() as HANDLE);

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
