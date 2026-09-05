//! What is left to do once the thing a rendering started has gone.
//!
//! A rendering is what a session is started from, and this is what the same
//! call hands back beside it: a value held for as long as that process runs and
//! asked to close once it has gone. Both callers hold one — a session's relay
//! and a Conversation Terminal's follow loop — because a terminal is a shell in
//! the same profile under the same account, and what is true of one session's
//! ending is true of the other's.
//!
//! **It is nothing at all on the two Unix platforms.** A bind and a symbolic
//! link are names that follow whatever happens at the far end of them, so a
//! session that ends leaves nothing to see to. What this exists for is the
//! platform whose links are hard ones — see [`super::open`], which joins a
//! file into a session's profile that way because a file symbolic link there
//! wants a privilege a per-user install has not got.
//!
//! **And a hard link is one file only while everything writes in place.** An
//! agent that saves its config by writing a temporary file and renaming it over
//! the top leaves the session writing to a file of its own, with the account's
//! copy seeing none of it and nothing saying so — and an agent's config is
//! exactly that kind of file. What is decided is the outcome rather than the
//! mechanism: nothing a session wrote to its account is lost. So a linked file
//! that is no longer the account's own is written back over it, and the link is
//! made fresh for the session after (ADR-0014). The ordinary case is one file
//! and costs nothing; the replacing case costs a copy rather than the session's
//! work.
//!
//! Directories are no part of it. A junction is a path rather than a file,
//! nothing replaces one, and what is behind it is the account itself.
//!
//! **Whether a file is still the account's own is asked of the file** rather
//! than remembered from when the link was made. Two names for one file is a
//! fact the filesystem holds, and a rename is precisely the thing that changes
//! it without telling anybody.
//!
//! **Nothing here refuses anything.** A session that has ended is long past
//! being refused, so a file that could not be written back is named in the log
//! with what went wrong, and the rest go on.

use std::io;
use std::path::{Path, PathBuf};

/// What a rendering left to be seen to once what it started has gone.
///
/// Made by the renderer rather than composed by a caller: which paths are in it
/// is a fact about how that platform joined an account into a profile, and the
/// two renderings that join nothing in by hand hand back [`Closing::nothing`].
#[derive(Debug)]
pub struct Closing {
    /// The files joined into a session's profile by a hard link, the account's
    /// own path first and the name inside the profile second.
    linked: Vec<(PathBuf, PathBuf)>,
}

impl Closing {
    /// Nothing to see to, which is what a rendering whose links follow their
    /// own target leaves behind.
    ///
    /// Said out here rather than kept inside the module, because on a Windows
    /// build the two renderings that hand one back are not compiled at all —
    /// and a constructor that is the answer on two platforms is not a thing to
    /// hide on the third.
    pub fn nothing() -> Closing {
        Closing { linked: Vec::new() }
    }

    /// And the files a rendering joined in by hard link, each as the account's
    /// own path and the name a session found it under.
    pub(crate) fn of_links(linked: Vec<(PathBuf, PathBuf)>) -> Closing {
        Closing { linked }
    }

    /// The names inside the profile this has anything left to do about — none
    /// at all where the rendering joined nothing in by hand.
    pub fn linked(&self) -> impl Iterator<Item = &Path> {
        self.linked.iter().map(|(_, inside)| inside.as_path())
    }

    /// The session has gone: whatever it wrote to its account that the account
    /// has not got is written back over it, and the link is made fresh.
    ///
    /// Blocks — it is a file copy at worst and two questions of the filesystem
    /// at best — so it is called off the runtime by whoever holds it.
    pub fn close(self) {
        for (host, inside) in self.linked {
            match written_back(&host, &inside) {
                Ok(true) => tracing::debug!(
                    account = %host.display(),
                    inside = %inside.display(),
                    "a file a session was given was replaced rather than written in place, so \
                     what the session wrote was written back over the account's own"
                ),
                Ok(false) => {}
                Err(error) => tracing::error!(
                    error = ?error,
                    account = %host.display(),
                    inside = %inside.display(),
                    "a file a session wrote could not be written back to the account, so what \
                     it wrote there is only inside the session's own profile"
                ),
            }
        }
    }
}

/// Write `inside` back over `host` where the two have stopped being one file,
/// and say whether anything had to be.
///
/// Two of the cases leave everything as it is. A file still one with the
/// account's is the ordinary session, which wrote in place and has nothing left
/// to give; and a name with nothing at it *inside* is a session that took away
/// the file it was given, which is not a session asking for the account's to
/// go.
///
/// The account having no such file is a write-back rather than a refusal, and
/// it is the case a fresh Profile starts in: the link could not be made because
/// there was nothing to link, and the first thing the session does is log in
/// and write one. That file is the account's.
fn written_back(host: &Path, inside: &Path) -> io::Result<bool> {
    let Some(ours) = identity(inside)? else {
        return Ok(false);
    };

    if identity(host)? == Some(ours) {
        return Ok(false);
    }

    // Over the account's own file rather than beside it: whatever else is a
    // name for that file — the human's own login is one — is a name for what
    // the session wrote, which is the whole point of the account being linked
    // in rather than copied.
    std::fs::copy(inside, host)?;

    // And one file again, so that the session after this reads and writes the
    // account rather than a copy of it. A rendering makes the link afresh
    // anyway; making it here is what keeps the profile true in between.
    super::open::joined(host, inside)?;

    Ok(true)
}

/// Which file `path` is, or `None` where there is nothing at that name.
///
/// Two numbers, and the only question ever asked of two of them is whether they
/// are the same: what is being settled is whether two names are one file, which
/// is what a hard link makes and what a rename over one takes away.
#[cfg(windows)]
fn identity(path: &Path) -> io::Result<Option<(u32, u64)>> {
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    // Safety: the structure is what the call is documented to fill in, and it
    // is a plain record of numbers with no invalid bit pattern.
    let mut about: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };

    // Safety: the handle is the open file's own and outlives the call, and the
    // pointer is to the structure above.
    let asked = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &raw mut about) };

    if asked == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(Some((
        about.dwVolumeSerialNumber,
        u64::from(about.nFileIndexHigh) << 32 | u64::from(about.nFileIndexLow),
    )))
}

/// And the same question on a Unix, which asks it of two numbers of its own.
///
/// **Nothing in production reaches this.** What it belongs to is the Windows
/// rendering, and [`crate::platform::Platform::HERE`] is never Windows on a
/// Unix. What it is for is the suite, the same as [`super::junction`]'s Unix
/// arm: a file is joined into a profile by a hard link on either kind of
/// machine, so what a session's ending comes to can be asked wherever the tests
/// are run.
#[cfg(unix)]
fn identity(path: &Path) -> io::Result<Option<(u64, u64)>> {
    use std::os::unix::fs::MetadataExt;

    match std::fs::metadata(path) {
        Ok(about) => Ok(Some((about.dev(), about.ino()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The account's own file and the name a session found it under, joined in
    /// the way a rendering joins one — see [`super::super::open::joined`].
    fn linked(account: &Path, profile: &Path) -> (PathBuf, PathBuf) {
        let (host, inside) = (account.join("config.json"), profile.join("config.json"));

        std::fs::write(&host, "the account's own\n").unwrap();
        super::super::open::joined(&host, &inside).unwrap();

        (host, inside)
    }

    /// What an agent that saves a file by writing a temporary one and renaming
    /// it over the top leaves behind: a name inside the profile that has
    /// stopped being the account's file at all.
    fn replaced(inside: &Path, said: &str) {
        let written = inside.with_extension("json.tmp");

        std::fs::write(&written, said).unwrap();
        std::fs::rename(&written, inside).unwrap();
    }

    /// The case this exists for: what the session wrote is on the account once
    /// the session has ended.
    #[test]
    fn a_file_replaced_inside_the_profile_is_written_back_over_the_account() {
        let account = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();
        let (host, inside) = linked(account.path(), profile.path());

        replaced(&inside, "what the session wrote\n");

        assert!(
            written_back(&host, &inside).unwrap(),
            "a file that is no longer the account's own has to be written back"
        );
        assert_eq!(
            std::fs::read_to_string(&host).unwrap(),
            "what the session wrote\n"
        );
        assert_eq!(
            identity(&host).unwrap(),
            identity(&inside).unwrap(),
            "and the link is made fresh, so the session after this one finds one \
             file rather than two"
        );
    }

    /// And the ordinary case, which is every session that wrote its config in
    /// place: the account already has what the session wrote, and nothing is
    /// copied anywhere.
    #[test]
    fn a_file_written_in_place_is_not_copied_back() {
        let account = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();
        let (host, inside) = linked(account.path(), profile.path());

        std::fs::write(&inside, "written in place\n").unwrap();

        assert!(
            !written_back(&host, &inside).unwrap(),
            "the two are one file, so there is nothing to write back"
        );
        assert_eq!(
            std::fs::read_to_string(&host).unwrap(),
            "written in place\n",
            "which the account has anyway, that being what one file means"
        );
    }

    /// A session that took away the file it was given is not a session asking
    /// for the account's to go.
    #[test]
    fn a_file_the_session_took_away_leaves_the_accounts_own_where_it_is() {
        let account = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();
        let (host, inside) = linked(account.path(), profile.path());

        std::fs::remove_file(&inside).unwrap();

        assert!(!written_back(&host, &inside).unwrap());
        assert_eq!(
            std::fs::read_to_string(&host).unwrap(),
            "the account's own\n"
        );
    }

    /// And the file the account never had, which is what a fresh Profile's
    /// first session writes: there was nothing to link, so what the session
    /// wrote is the account's own from here.
    #[test]
    fn a_file_the_account_never_had_is_the_accounts_once_the_session_wrote_it() {
        let account = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();

        let (host, inside) = (
            account.path().join("config.json"),
            profile.path().join("config.json"),
        );

        std::fs::write(&inside, "logged in\n").unwrap();

        assert!(written_back(&host, &inside).unwrap());
        assert_eq!(std::fs::read_to_string(&host).unwrap(), "logged in\n");
    }
}
