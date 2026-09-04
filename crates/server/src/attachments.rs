//! Where a Conversation's attached files are kept, and what a file is called
//! once it is there.
//!
//! One flat directory per Conversation under the Data Directory,
//! `attachments/<id>/`, shaped on the handoff directories beside it: a root
//! Verkstead made, one directory named by the Conversation's own id, and
//! nothing to keep in a table about which is whose. The record of what is in
//! one is the `attachments` rows — see the store's own module — and this is the
//! half of it that is bytes.
//!
//! Outside the Worktree for the handoffs' reason and one of its own. A file
//! written into the checkout would be swept into the human's repository by the
//! first `git add -A` after it; and a Conversation is given a Worktree when its
//! work starts, while a file is attached to a draft that has none.
//!
//! **A file keeps its own base name**, because the path is one an agent has to
//! be able to type out of a listing in its prompt. So what may be attached is a
//! plain base name and nothing else — no separator, no leading dot, nothing
//! empty — and a name already taken is not overwritten: the newcomer counts up
//! over its own extension-less stem, `notes-2.md` and then `notes-3.md`, with
//! no spaces or brackets in it. Both files stay, and both are records.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::platform::Platform;

/// How large one attached file may be.
///
/// Generous, because what the human has to hand is a screenshot or an export
/// rather than a paragraph — and finite, because the body is read into memory
/// to be written out. The route carries this as its own body limit over the
/// router's default, the way the Question Set route raises its own.
pub const MAX_BYTES: usize = 32 * 1024 * 1024;

/// Where a Conversation's attached files are read inside its sandbox, on the
/// platform whose sandbox can mount one there.
///
/// Beside the skills, in the directory of Verkstead's own — see
/// [`crate::sandbox::own_directory`]. What is under that directory is what
/// Verkstead put there rather than whatever the machine happened to have, and
/// the path is nobody's: nothing the human attached lands where a backend goes
/// looking for something of its own.
pub(crate) const INSIDE: &str = "/verkstead/attachments";

/// Where a session whose Conversation's files are at `directory` reads them.
///
/// [`INSIDE`] where a bind makes that path, and the directory's own real path
/// where none can. On a Mac `/verkstead` is the Data Directory itself, so there
/// is no bind to put one Conversation's directory at a name every Conversation
/// would otherwise share: what the policy reaches is that Conversation's own
/// subdirectory of the attachments root and no other, and what the prompt names
/// is where the files really are.
///
/// The skills' own arrangement, one level deeper — see
/// [`crate::skills::Skills::inside`]. Theirs is one directory for the whole
/// installation, so one path serves every session; this is one per
/// Conversation, so the path a session is told is that Conversation's.
///
/// A `Platform` rather than a `cfg`, for the reason [`Platform`] is a value at
/// all: the arm this machine will never run is still an arm a test on it can
/// ask for, and this one decides a path a session is told about in prose.
pub(crate) fn inside(platform: Platform, directory: &Path) -> PathBuf {
    match platform {
        Platform::MacOs => directory.to_owned(),
        Platform::Linux | Platform::Windows => PathBuf::from(INSIDE),
    }
}

/// One Conversation's attached files as its sandbox is given them: the
/// directory on the host, and where the session inside reads it.
///
/// Both of them, because the two are the same path on only one platform — see
/// [`inside`]. Which way a session may reach it is not carried here at all: it
/// is read-only whatever is in it, the copy being the record, and an agent that
/// wants to work on a file copies it into the Worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Bound {
    host: PathBuf,
    inside: PathBuf,
}

impl Bound {
    /// The directory on the host, which is what the bind reaches for.
    pub(crate) fn host(&self) -> &Path {
        &self.host
    }

    /// And where a session finds it.
    pub(crate) fn inside(&self) -> &Path {
        &self.inside
    }
}

/// The directories a Conversation's attached files are kept in.
///
/// A root under the Data Directory and nothing else, exactly as
/// [`crate::handoffs::Handoffs`] is: which directory belongs to which
/// Conversation is its id.
#[derive(Debug, Clone)]
pub struct Attachments {
    root: PathBuf,
}

impl Attachments {
    /// The root under `data_dir`, beside the worktrees and the handoffs — all
    /// three are things Verkstead made rather than things the human pointed it
    /// at.
    pub fn under(data_dir: &Path) -> Attachments {
        Attachments {
            root: data_dir.join("attachments"),
        }
    }

    /// One Conversation's directory, whether or not anything has been attached
    /// yet.
    ///
    /// The path rather than the making of it: a sandbox binds this directory
    /// and a listing reads it, and neither of those should be creating
    /// anything. [`Self::keep`] makes it on the way past, which is the one
    /// moment there is something to put in it.
    pub(crate) fn directory(&self, conversation_id: i64) -> PathBuf {
        self.root.join(conversation_id.to_string())
    }

    /// What a Conversation's sandbox binds, or `None` where there is nothing to
    /// bind — no bind, and nothing at that path inside.
    ///
    /// Asked of the directory rather than of the record, because what a bind
    /// needs is a directory: a source that is not there would be a session
    /// refusing to start with the reason buried in bwrap's complaint. And asked
    /// for something *in* it rather than for the directory alone, because one
    /// holding nothing is the Conversation whose files were all removed again —
    /// the directory is left where it is — and a path an agent is told about and
    /// finds empty is worse than a path that is not there.
    pub(crate) fn bound(&self, platform: Platform, conversation_id: i64) -> Option<Bound> {
        let host = self.directory(conversation_id);

        // A directory that is not there and one holding nothing are the same
        // answer here, which is what makes this one reading rather than two.
        std::fs::read_dir(&host).ok()?.next()?.ok()?;

        Some(Bound {
            inside: inside(platform, &host),
            host,
        })
    }

    /// And where a session reads them, asked without asking whether there is
    /// anything there: what the prompt's listing names each file under.
    ///
    /// One answer for the bind and the listing both, so that a session cannot be
    /// told about a path other than the one it was given.
    pub(crate) fn inside(&self, platform: Platform, conversation_id: i64) -> PathBuf {
        inside(platform, &self.directory(conversation_id))
    }

    /// Put a file in a Conversation's directory, answering with the name it
    /// ended up under.
    ///
    /// Which is `name` where the directory did not already have one, and
    /// `name-2.ext` — then `-3`, and up — where it did. The counting is done by
    /// creating the file rather than by looking first: two uploads landing
    /// together would both find the same name free and the second would write
    /// over the first, so each candidate is created exclusively and a name
    /// somebody else has just taken sends this on to the next one.
    ///
    /// The name is a plain base name by the time this is called — see
    /// [`plain`], which is what the endpoint refuses on. Nothing here re-asks:
    /// a caller that skipped it would be writing wherever the name pointed, and
    /// the check belongs where the refusal has somewhere to be said.
    pub(crate) fn keep(&self, conversation_id: i64, name: &str, body: &[u8]) -> Result<String> {
        let directory = self.directory(conversation_id);

        std::fs::create_dir_all(&directory).with_context(|| {
            format!(
                "making the attachments directory of Conversation {conversation_id} at {}",
                directory.display(),
            )
        })?;

        let mut counting = 1;

        loop {
            let called = counted(name, counting);

            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(&called))
            {
                Ok(mut file) => {
                    file.write_all(body).with_context(|| {
                        format!("writing {called:?} into the directory of Conversation {conversation_id}")
                    })?;

                    return Ok(called);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => counting += 1,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "making {called:?} in the directory of Conversation {conversation_id}"
                        )
                    });
                }
            }
        }
    }

    /// Take one file back out again.
    ///
    /// A file that is not there is nothing to remove: the row and the file are
    /// two writes rather than one act, and a removal that ran twice — or one
    /// that follows a directory somebody tidied by hand — has nothing left to
    /// do and no reason to say so.
    pub(crate) fn drop_file(&self, conversation_id: i64, name: &str) -> Result<()> {
        let path = self.directory(conversation_id).join(name);

        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error)
                .with_context(|| format!("removing the attached file at {}", path.display())),
        }
    }
}

/// Whether a name is one a file may be attached under: a plain base name, and
/// nothing else.
///
/// Three refusals, and each of them is a path rather than a name. A separator
/// of either kind would put the file somewhere other than the Conversation's
/// own directory — and a backslash reaching a Linux server is a name rather
/// than a separator there, which is worse: it is a file an agent could not type
/// out of its own prompt. A leading dot is `.`, `..`, and every hidden file a
/// listing would not show. An empty name is not a file at all.
///
/// And the whole of it asked of the platform as well, which is what catches
/// what this build has not thought of: a name is plain exactly where the path
/// it makes has that name as its one and only component.
pub(crate) fn plain(name: &str) -> bool {
    if name.is_empty() || name.starts_with('.') {
        return false;
    }

    if name.contains(['/', '\\']) || name.contains(char::is_control) {
        return false;
    }

    Path::new(name).file_name().is_some_and(|only| only == name)
}

/// `name` where `counting` is one, and the name counted up over its
/// extension-less stem where it is not — `notes.md` becoming `notes-2.md`.
///
/// The stem is everything before the last dot, so `logs.tar.gz` counts up as
/// `logs.tar-2.gz`: what the counter goes before is the extension a machine
/// opens the file by, and the rest of the name is the human's.
///
/// A hyphen and a number, with no spaces and no brackets, because the result is
/// a path an agent reads out of its prompt and types into a command.
fn counted(name: &str, counting: u32) -> String {
    if counting == 1 {
        return name.to_owned();
    }

    let path = Path::new(name);

    match (path.file_stem(), path.extension()) {
        (Some(stem), Some(extension)) => format!(
            "{}-{counting}.{}",
            stem.to_string_lossy(),
            extension.to_string_lossy(),
        ),
        _ => format!("{name}-{counting}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conversations_directory_is_under_the_root() {
        let attachments = Attachments::under(Path::new("/data"));

        assert_eq!(
            attachments.directory(7),
            Path::new("/data/attachments/7"),
            "beside the handoffs, and named by the Conversation's own id",
        );
    }

    #[test]
    fn a_file_lands_under_its_own_name() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        let called = attachments.keep(7, "wireframe.png", b"PNG").unwrap();

        assert_eq!(called, "wireframe.png");
        assert_eq!(
            std::fs::read(state.path().join("attachments/7/wireframe.png")).unwrap(),
            b"PNG",
        );
    }

    /// The second of a name is renamed rather than replacing the first: both
    /// files were handed over, and both are records.
    #[test]
    fn a_name_already_there_counts_up_instead_of_replacing_it() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        assert_eq!(
            attachments.keep(7, "notes.md", b"first").unwrap(),
            "notes.md"
        );
        assert_eq!(
            attachments.keep(7, "notes.md", b"second").unwrap(),
            "notes-2.md",
        );
        assert_eq!(
            attachments.keep(7, "notes.md", b"third").unwrap(),
            "notes-3.md",
        );

        let directory = state.path().join("attachments/7");
        assert_eq!(std::fs::read(directory.join("notes.md")).unwrap(), b"first");
        assert_eq!(
            std::fs::read(directory.join("notes-2.md")).unwrap(),
            b"second",
        );
    }

    /// The counter goes before the extension, whatever the rest of the name is
    /// doing — and after the whole name where there is no extension to go
    /// before.
    #[test]
    fn the_count_goes_over_the_extension_less_stem() {
        assert_eq!(counted("notes.md", 1), "notes.md");
        assert_eq!(counted("notes.md", 2), "notes-2.md");
        assert_eq!(counted("logs.tar.gz", 2), "logs.tar-2.gz");
        assert_eq!(counted("README", 4), "README-4");
    }

    /// One Conversation's directory is not another's, which is the whole of what
    /// keeps two of them out of each other's files.
    #[test]
    fn one_conversations_files_are_not_anothers() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        assert_eq!(
            attachments.keep(7, "notes.md", b"mine").unwrap(),
            "notes.md"
        );
        assert_eq!(
            attachments.keep(8, "notes.md", b"theirs").unwrap(),
            "notes.md",
            "the name is free in a directory of its own",
        );
    }

    /// Where a bind can make one path serve every Conversation it is that path,
    /// and where none can it is the directory the files are really in — which is
    /// what keeps one Conversation's listing out of another's on the platform
    /// that has nothing to mount.
    #[test]
    fn what_a_session_is_told_is_the_one_directory_it_was_given() {
        let attachments = Attachments::under(Path::new("/data"));

        assert_eq!(
            attachments.inside(Platform::Linux, 7),
            Path::new(INSIDE),
            "beside the skills, in the directory of Verkstead's own",
        );

        assert_eq!(
            attachments.inside(Platform::MacOs, 7),
            Path::new("/data/attachments/7"),
            "and where nothing can be mounted, the real directory",
        );
        assert_ne!(
            attachments.inside(Platform::MacOs, 7),
            attachments.inside(Platform::MacOs, 8),
            "which is a different one per Conversation, as the bind's is",
        );
    }

    /// A Conversation with nothing attached gets no bind: there would be nothing
    /// at the path, and a listing of nothing is worse than no path at all.
    #[test]
    fn there_is_nothing_to_bind_until_something_is_attached() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        assert_eq!(
            attachments.bound(Platform::Linux, 7),
            None,
            "nothing has ever been attached, so there is no directory either",
        );

        attachments.keep(7, "notes.md", b"first").unwrap();

        let bound = attachments
            .bound(Platform::Linux, 7)
            .expect("a Conversation with a file attached is bound");

        assert_eq!(bound.host(), state.path().join("attachments/7"));
        assert_eq!(bound.inside(), Path::new(INSIDE));

        attachments.drop_file(7, "notes.md").unwrap();

        assert_eq!(
            attachments.bound(Platform::Linux, 7),
            None,
            "and the directory the removal left behind is not something to bind",
        );
    }

    #[test]
    fn a_file_is_taken_back_out_again() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        attachments.keep(7, "notes.md", b"first").unwrap();
        attachments.drop_file(7, "notes.md").unwrap();

        assert!(!state.path().join("attachments/7/notes.md").exists());

        // Removed twice, which is a removal that ran again over a row that had
        // already gone: nothing to do, and no reason to say so.
        attachments.drop_file(7, "notes.md").unwrap();
    }

    #[test]
    fn a_plain_base_name_is_the_only_thing_that_may_be_attached() {
        for name in [
            "notes.md",
            "wireframe.png",
            "logs.tar.gz",
            "README",
            "a b.csv",
        ] {
            assert!(plain(name), "{name:?} is a plain base name");
        }

        for name in [
            "",
            ".",
            "..",
            ".gitignore",
            "../escape.md",
            "notes/../escape.md",
            "/etc/passwd",
            "sub/notes.md",
            "sub\\notes.md",
            "line\nbreak.md",
        ] {
            assert!(!plain(name), "{name:?} is not one");
        }
    }
}
