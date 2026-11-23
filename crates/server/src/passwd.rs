//! The passwd database, asked about the one account anything here cares about:
//! the user this server runs as.
//!
//! **Two fields are read off it**, and each is a fact about the machine rather
//! than anything Verkstead was told. Which shell a Conversation's **Terminal**
//! comes up in is the account's login shell — the shell the human would have
//! got had they sat down at the machine (see [`crate::terminals::shell`], which
//! is where that answer is judged before it is believed). And where a path
//! field's browse opens on an **Annex** is the account's home: the box logs in
//! as the service's own user, so the home passwd gives it is the directory the
//! human's own clones land in, while the `HOME` the service runs with is a state
//! directory holding none of them (ADR-0024, *Open repo asks where the
//! repository is*).
//!
//! **The database rather than the file.** `getpwuid_r` asks whichever store
//! this machine keeps its accounts in — `/etc/passwd` on NixOS, the directory
//! service on a Mac — and reading the file directly would find nothing on a Mac
//! for every account a human actually has.
//!
//! **And on Windows there is no such database.** An account there has neither a
//! login shell nor a passwd home recorded against it, so both readings answer
//! nothing and each caller falls back to what that platform does instead.

use std::path::PathBuf;

/// The login shell the database gives the user this server runs as, or `None`
/// where it holds no entry for it.
///
/// Believed by nobody as it stands: [`crate::terminals::shell`] is what decides
/// whether a terminal can be opened on the answer.
#[cfg(unix)]
pub(crate) fn login_shell() -> Option<String> {
    read(|entry| said(entry.pw_shell))
}

/// And the home it gives that same user, which is not the `HOME` the service
/// runs with — see this module's own documentation for the one device where the
/// two part company.
#[cfg(unix)]
pub(crate) fn login_home() -> Option<PathBuf> {
    read(|entry| said(entry.pw_dir).map(PathBuf::from))
}

/// Nothing, on the platform that keeps no such database — the other arm of each
/// reading above, for the reason this module's documentation gives.
#[cfg(not(unix))]
pub(crate) fn login_shell() -> Option<String> {
    None
}

#[cfg(not(unix))]
pub(crate) fn login_home() -> Option<PathBuf> {
    None
}

/// One reading of the entry, with `of` picking a field out of it.
///
/// The reentrant call, with a buffer that grows until the entry fits, because
/// the server is threaded and the plain `getpwuid` hands back a pointer into
/// storage the next caller reuses. `of` runs while that buffer is still alive
/// and copies whatever it takes out of it, which is why it is a closure rather
/// than the entry being handed back.
#[cfg(unix)]
fn read<T>(of: impl Fn(&libc::passwd) -> Option<T>) -> Option<T> {
    // Where the buffer starts and how far it is allowed to grow. glibc suggests
    // a size of its own and this is comfortably past it; the ceiling is there so
    // that a database answering `ERANGE` forever is nothing read rather than a
    // server eating memory.
    const ROOM: usize = 1024;
    const MOST: usize = 64 * 1024;

    let mut room = ROOM;

    loop {
        let mut buffer = vec![0u8; room];
        let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
        let mut found: *mut libc::passwd = std::ptr::null_mut();

        // SAFETY: `entry` and `found` are ours to write into, and the buffer the
        // strings are written into is the one whose length is passed beside it.
        // Nothing read out of them outlives this iteration: whatever `of` takes
        // is copied before the buffer goes.
        let answered = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                &mut entry,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut found,
            )
        };

        if answered == libc::ERANGE && room < MOST {
            room *= 2;
            continue;
        }

        // A non-zero answer is the lookup failing and a null `found` is the
        // database holding no such user — both are a machine with nothing to say
        // about the account the server runs as.
        if answered != 0 || found.is_null() {
            tracing::debug!(
                answered,
                "the passwd database holds no entry for this server's own user"
            );

            return None;
        }

        return of(&entry);
    }
}

/// What one of the entry's fields says, or `None` where it says nothing at all —
/// a null field, and an empty one, being two ways of saying the same nothing.
#[cfg(unix)]
fn said(field: *const libc::c_char) -> Option<String> {
    if field.is_null() {
        return None;
    }

    // SAFETY: the entry was filled in by `read` and its strings point into the
    // buffer, which is alive for as long as the closure this runs inside is.
    let said = unsafe { std::ffi::CStr::from_ptr(field) };

    said.to_str()
        .ok()
        .filter(|said| !said.is_empty())
        .map(str::to_owned)
}
