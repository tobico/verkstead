//! What is left to do once the thing a rendering started has gone.
//!
//! A rendering is what a session is started from, and this is what the same
//! call hands back beside it: a value held for as long as that process runs and
//! asked to close once it has gone. Both callers hold one — a session's relay
//! and a Conversation Terminal's follow loop — because a terminal is a shell in
//! the same profile under the same account, and what is true of one session's
//! ending is true of the other's.
//!
//! **It began as the platform whose links are hard ones** — see
//! [`super::open`], which joins a file into a session's profile that way
//! because a file symbolic link there wants a privilege a per-user install has
//! not got. A bind and a symbolic link are names that follow whatever happens
//! at the far end of them, so on the two Unix platforms most of what a session
//! was given leaves nothing to see to.
//!
//! **Most, and not Claude's credentials file.** A rename over a symbolic link
//! replaces the link rather than writing through it, which is what Claude does
//! to its login on a Mac; and on Linux an account with no login has nothing to
//! bind, so the file a session logs in and writes is a file of the root's own.
//! Both are handed back here beside the Windows links — see
//! [`super::Sandbox::command`].
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
//! **A file given as a copy is merged back rather than written back.** Claude's
//! `.claude.json` is copied into a session's profile on every platform, so the
//! trust seeded into it is never written straight into the account. A copy is
//! never one file with the account's, so what the session changed in it is
//! merged into the account's own instead — see [`super::root::merged_back`].
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

use super::sharing::{Baseline, Share};

/// How a file that was written back is made one with the account's again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rejoin {
    /// By a hard link, which is how the Windows rendering joined it.
    Hard,

    /// By a symbolic link, which is how a Mac joined it.
    Symbolic,

    /// Not at all. Linux binds the file where the account has one, and a bind
    /// is not a thing to make outside a namespace — so a file of the root's own
    /// stays the session's, and the next session is bound to the account's.
    Not,
}

/// What a rendering left to be seen to once what it started has gone.
///
/// Made by the renderer, which knows how its platform joined an account into a
/// profile, and added to by [`super::Sandbox::command`], which knows which file
/// is a login that a rename can take away.
#[derive(Debug)]
pub struct Closing {
    /// The files a session was given that it may replace rather than write:
    /// the account's own path, the name the session found it under, and how
    /// the two are made one file again once it has been written back.
    linked: Vec<(PathBuf, PathBuf, Rejoin)>,

    /// And the files a session was given a copy of rather than a link to: the
    /// account's own path, where the copy is on the host, and the copy as the
    /// account last had it. What the session changed in it is merged into the
    /// account's — see [`super::root::merged_back`].
    ///
    /// **Apart from the linked ones**, because a copy is never one file with the
    /// account's. Asked the identity question, it would be written back whole at
    /// every ending and linked afterwards, which is neither what a copy is for.
    copied: Vec<(PathBuf, PathBuf, Baseline)>,

    /// And the root this launch shares with whatever else of its Conversation
    /// is running in it, on Linux — held for as long as it runs, so the next
    /// launch does not build the root again from under it. See
    /// [`super::sharing`].
    share: Option<Share>,

    /// And the Conversation's entries the session is running behind, held for
    /// as long as it runs.
    ///
    /// **Held rather than seen to**, and held here as well as by the module
    /// that owns them. Their life is the Conversation's — written at the first
    /// session, taken back with the Worktree, swept for at startup (see
    /// [`super::entries`] and [`crate::boundaries`]) — so what this adds is the
    /// one thing that lifetime cannot say on its own: a session is still
    /// running. A close that arrives while one is takes the entries out of the
    /// module's hands and finds this one still holding them, and they come off
    /// the human's directories when the session ends rather than out from under
    /// it.
    #[cfg(windows)]
    behind: Option<std::sync::Arc<super::entries::Entries>>,
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
        Closing {
            linked: Vec::new(),
            copied: Vec::new(),
            share: None,
            #[cfg(windows)]
            behind: None,
        }
    }

    /// And the files a rendering joined in by hard link, each as the account's
    /// own path and the name a session found it under.
    pub(crate) fn of_links(linked: Vec<(PathBuf, PathBuf)>) -> Closing {
        Closing {
            linked: linked
                .into_iter()
                .map(|(host, inside)| (host, inside, Rejoin::Hard))
                .collect(),
            copied: Vec::new(),
            share: None,
            #[cfg(windows)]
            behind: None,
        }
    }

    /// The same, with one more file to write back — see [`Rejoin`] for what is
    /// made of the two names afterwards.
    pub(crate) fn and(mut self, host: PathBuf, inside: PathBuf, rejoin: Rejoin) -> Closing {
        self.linked.push((host, inside, rejoin));

        self
    }

    /// The same, with a file whose copy `copy` is merged against `baseline` —
    /// see the field for what is done with it.
    pub(crate) fn merging(mut self, host: PathBuf, copy: PathBuf, baseline: Baseline) -> Closing {
        self.copied.push((host, copy, baseline));

        self
    }

    /// The same, holding the root this launch shares — see the field.
    pub(crate) fn sharing(mut self, share: Share) -> Closing {
        self.share = Some(share);

        self
    }

    /// The same, holding the entries the session runs behind — see the field,
    /// which is where the whole of what holding them means is.
    #[cfg(windows)]
    pub(crate) fn behind(mut self, entries: std::sync::Arc<super::entries::Entries>) -> Closing {
        self.behind = Some(entries);

        self
    }

    /// The names inside the profile this has anything left to do about — none
    /// at all where the rendering joined nothing in by hand.
    pub fn linked(&self) -> impl Iterator<Item = &Path> {
        self.linked.iter().map(|(_, inside, _)| inside.as_path())
    }

    /// And the copies on the host whose changes are merged back, apart from
    /// those names — see [`Closing::merging`].
    pub fn copied(&self) -> impl Iterator<Item = &Path> {
        self.copied.iter().map(|(_, copy, _)| copy.as_path())
    }

    /// The session has gone: whatever it wrote to its account that the account
    /// has not got is written back over it, and the link is made fresh.
    ///
    /// Blocks — it is a file copy at worst and two questions of the filesystem
    /// at best — so it is called off the runtime by whoever holds it.
    pub fn close(self) {
        for (host, inside, rejoin) in self.linked {
            match written_back(&host, &inside, rejoin) {
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

        for (host, copy, baseline) in self.copied {
            // Under the baseline's lock, because a copy shared by two launches
            // is merged by whichever ends first and then by the other, and the
            // second is to carry only what changed after the first.
            let Ok(mut baseline) = baseline.lock() else {
                continue;
            };

            match super::root::merged_back(&host, &copy, &mut baseline) {
                Ok(true) => tracing::debug!(
                    account = %host.display(),
                    copy = %copy.display(),
                    "what a session changed in its copy of a file was merged into the account's own"
                ),
                Ok(false) => {}
                Err(error) => tracing::error!(
                    error = ?error,
                    account = %host.display(),
                    copy = %copy.display(),
                    "what a session changed in its copy of a file could not be merged into the \
                     account's own, so it is only in the session's own profile"
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
fn written_back(host: &Path, inside: &Path, rejoin: Rejoin) -> io::Result<bool> {
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
    match rejoin {
        Rejoin::Hard => super::open::joined(host, inside)?,
        #[cfg(unix)]
        Rejoin::Symbolic => {
            std::fs::remove_file(inside)?;
            std::os::unix::fs::symlink(host, inside)?;
        }
        #[cfg(not(unix))]
        Rejoin::Symbolic => {}
        Rejoin::Not => {}
    }

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
            written_back(&host, &inside, Rejoin::Hard).unwrap(),
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
            !written_back(&host, &inside, Rejoin::Hard).unwrap(),
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

        assert!(!written_back(&host, &inside, Rejoin::Hard).unwrap());
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

        assert!(written_back(&host, &inside, Rejoin::Hard).unwrap());
        assert_eq!(std::fs::read_to_string(&host).unwrap(), "logged in\n");
    }

    /// A login joined in by a symbolic link, as a Mac joins one, and replaced
    /// by the rename Claude saves it with: written back, and linked again.
    #[cfg(unix)]
    #[test]
    fn a_file_replaced_over_a_symbolic_link_is_written_back_and_linked_again() {
        let account = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let (host, inside) = (
            account.path().join(".credentials.json"),
            root.path().join(".credentials.json"),
        );

        std::fs::write(&host, "the account's own\n").unwrap();
        std::os::unix::fs::symlink(&host, &inside).unwrap();

        Closing::nothing()
            .and(host.clone(), inside.clone(), Rejoin::Symbolic)
            .close();

        assert_eq!(
            std::fs::read_to_string(&host).unwrap(),
            "the account's own\n",
            "a link nothing replaced is the account's file, and nothing is copied"
        );

        replaced(&inside, "refreshed\n");

        Closing::nothing()
            .and(host.clone(), inside.clone(), Rejoin::Symbolic)
            .close();

        assert_eq!(std::fs::read_to_string(&host).unwrap(), "refreshed\n");
        assert_eq!(
            std::fs::read_link(&inside).unwrap(),
            host,
            "and the root links the account's file again"
        );
    }

    /// And a file of the root's own on Linux, where there was no login to bind:
    /// written back, and left where it is.
    #[test]
    fn a_file_the_root_made_is_written_back_and_left_as_it_is() {
        let account = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let (host, inside) = (
            account.path().join(".credentials.json"),
            root.path().join(".credentials.json"),
        );

        std::fs::write(&inside, "logged in\n").unwrap();

        assert!(written_back(&host, &inside, Rejoin::Not).unwrap());
        assert_eq!(std::fs::read_to_string(&host).unwrap(), "logged in\n");
        assert!(
            !std::fs::symlink_metadata(&inside).unwrap().is_symlink(),
            "a file of its own, still"
        );
    }
}
