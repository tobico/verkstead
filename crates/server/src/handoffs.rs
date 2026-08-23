//! The handoff document: where a grilling session writes it, and how it gets
//! from there onto the Timeline.
//!
//! A grilling and the implementation that follows it run under different Agent
//! Profiles, and a session cannot change the account it is running as — so the
//! implementation is a fresh session rather than the grilling one carrying on,
//! and everything the grilling knows has to be written down before it ends. That
//! document is the handoff, and this is the directory it is written in.
//!
//! Outside the Worktree, deliberately. The handoff is Verkstead's artifact
//! rather than the project's: a document written into the checkout would be a
//! file every `git add -A` after it swept into the human's repository, and no
//! amount of instructing an agent not to commit something is as good as the file
//! not being there. So each Conversation gets a directory of Verkstead's own,
//! under the State Directory beside the worktrees, bound into every one of its
//! sandboxes at [`INSIDE`] — which is under `/tmp` because that is where a
//! session already expects to find what is neither the checkout nor its home.
//!
//! Taking it is a move rather than a read: the copy in the directory goes as the
//! Timeline gets one, so the Event is the handoff from then on and there is
//! never a second one to go stale.

use std::path::{Path, PathBuf};

/// Where a Conversation's directory is mounted inside its sandbox.
pub(crate) const INSIDE: &str = "/tmp/verkstead";

/// What the handoff document is called inside it.
const HANDOFF: &str = "handoff.md";

/// The two of those together, which is the path the grilling skill names and the
/// session writes.
///
/// Nothing in the server says it: the skill is markdown and says it in prose,
/// and this module reads the file from the outside where it is somewhere else
/// entirely. It is here so that the tests can hold the skill's sentence and the
/// mount to each other — the one place the two could drift apart is a session
/// writing a document nothing ever reads.
#[cfg(test)]
pub(crate) const HANDOFF_INSIDE: &str = "/tmp/verkstead/handoff.md";

/// The directories Conversations' sessions are given outside their worktrees.
///
/// A root under the State Directory and nothing else: which directory belongs to
/// which Conversation is its id, so there is nothing to keep in a table.
#[derive(Debug, Clone)]
pub struct Handoffs {
    root: PathBuf,
}

impl Handoffs {
    /// The root under `state_dir`, beside the worktrees — both are things
    /// Verkstead made rather than things the human pointed it at.
    pub fn under(state_dir: &Path) -> Handoffs {
        Handoffs {
            root: state_dir.join("handoffs"),
        }
    }

    /// One Conversation's directory, made if it is not there yet.
    ///
    /// `None` where it could not be made, which is a Conversation with nowhere
    /// to put a handoff: the sandbox binds this directory, so a bind source that
    /// is not there would be a session that refuses to start with the reason
    /// buried in bwrap's complaint.
    ///
    /// Made here rather than when the worktree is, so that it is true of every
    /// session whatever state the Conversation was in when this landed.
    pub fn directory(&self, conversation_id: i64) -> Option<PathBuf> {
        let path = self.root.join(conversation_id.to_string());

        match std::fs::create_dir_all(&path) {
            Ok(()) => Some(path),
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    conversation_id,
                    path = %path.display(),
                    "a Conversation's handoff directory could not be made"
                );
                None
            }
        }
    }

    /// Where a Conversation's handoff document is written, seen from outside its
    /// sandbox.
    ///
    /// The path rather than the document, because the inline tail is watched for
    /// this file arriving the way a backlog is watched for on the branch — see
    /// [`crate::runner`]. The directory is not made here: [`Self::directory`]
    /// does that as the sandbox is built, and a watcher only ever reads.
    pub(crate) fn document(&self, conversation_id: i64) -> PathBuf {
        self.root.join(conversation_id.to_string()).join(HANDOFF)
    }

    /// The handoff a session wrote, taken out of the directory.
    ///
    /// `None` where there is none, which is an ordinary thing rather than a
    /// failure: a grilling that ended without writing one is a grilling whose
    /// agent skipped its closing move, and what follows is primed with the Brief
    /// alone.
    ///
    /// Removed as it is read, so the Timeline holds the only copy. An empty file
    /// counts as none: a document of nothing says nothing, and an Event holding
    /// it would be a blank card on the Timeline for good.
    pub(crate) fn take(&self, conversation_id: i64) -> Option<String> {
        let path = self.document(conversation_id);

        let written = read(&path)?;

        if let Err(error) = std::fs::remove_file(&path) {
            // Kept rather than refused: the document is in hand, and a copy left
            // behind is worth less than the handoff reaching the Timeline.
            tracing::error!(
                error = ?error,
                conversation_id,
                path = %path.display(),
                "a handoff document could not be taken out of its directory"
            );
        }

        Some(written)
    }

    /// Give a Conversation's directory back, with whatever is in it.
    ///
    /// Called where the worktree is removed, because it is the same thing: what
    /// a Conversation was given to work in, given back when it stops. A
    /// directory that was never made is nothing to remove.
    pub(crate) fn remove(&self, conversation_id: i64) {
        let path = self.root.join(conversation_id.to_string());

        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::error!(
                error = ?error,
                conversation_id,
                path = %path.display(),
                "a Conversation's handoff directory could not be removed"
            ),
        }
    }
}

/// Whether a handoff has been written at `path` — which is what says an inline
/// grilling's tail has landed.
///
/// The same reading [`Handoffs::take`] gives, asked without taking anything: a
/// file that is not there and a file with nothing in it are both a handoff that
/// has not been written yet. One rule in one place, because the watcher and the
/// taker disagreeing would be a session ended over a document that then read as
/// none.
pub(crate) fn written(path: &Path) -> bool {
    read(path).is_some()
}

/// The document at `path`, or `None` where there is nothing there to hand over.
///
/// A file that will not read reads as none, which is the right way round for
/// what turns on it: a session is ended on the strength of this, and a
/// filesystem that was briefly busy is no reason to end one.
fn read(path: &Path) -> Option<String> {
    let written = match std::fs::read_to_string(path) {
        Ok(written) => written,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::error!(
                error = ?error,
                path = %path.display(),
                "a handoff document could not be read"
            );
            return None;
        }
    };

    (!written.trim().is_empty()).then_some(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The skill names one path and the sandbox mounts another half of it, and
    /// the two have to be the same file.
    #[test]
    fn the_path_the_skill_names_is_the_directory_that_is_mounted() {
        assert_eq!(Path::new(INSIDE).join(HANDOFF), Path::new(HANDOFF_INSIDE));
    }

    #[test]
    fn a_conversations_directory_is_made_where_it_is_asked_for() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());

        let path = handoffs.directory(7).expect("a directory to be made");

        assert_eq!(path, state.path().join("handoffs/7"));
        assert!(path.is_dir());
    }

    /// Taken rather than read: the Timeline holds the only copy afterwards, so
    /// nothing can come back to a document that has already been put on it.
    #[test]
    fn a_handoff_is_taken_out_of_the_directory_it_was_written_in() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());

        let path = handoffs.directory(7).unwrap();
        std::fs::write(path.join(HANDOFF), "# What we settled\n").unwrap();

        assert_eq!(handoffs.take(7).as_deref(), Some("# What we settled\n"));
        assert_eq!(handoffs.take(7), None, "the second read finds nothing");
    }

    /// A grilling that never wrote one, and one that wrote nothing: the same
    /// answer, because a document of nothing hands nothing over.
    #[test]
    fn nothing_written_is_no_handoff() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());

        assert_eq!(handoffs.take(7), None);

        let path = handoffs.directory(7).unwrap();
        std::fs::write(path.join(HANDOFF), "  \n\n").unwrap();

        assert_eq!(handoffs.take(7), None);
    }

    /// The watcher's reading and the taker's are one reading. An inline session
    /// is ended on the first and its document is read by the second, and the two
    /// disagreeing would be a grilling ended over a handoff that then read as
    /// none.
    #[test]
    fn a_handoff_reads_as_written_exactly_where_it_would_be_taken() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());
        let document = handoffs.document(7);

        assert!(
            !written(&document),
            "there is no directory yet, let alone one"
        );

        let path = handoffs.directory(7).unwrap();
        assert!(!written(&document), "nor a document in it");

        std::fs::write(path.join(HANDOFF), "  \n\n").unwrap();
        assert!(!written(&document), "a document of nothing is not one");

        std::fs::write(path.join(HANDOFF), "# What we settled\n").unwrap();
        assert!(written(&document));

        assert_eq!(handoffs.take(7).as_deref(), Some("# What we settled\n"));
        assert!(!written(&document), "and taking it leaves none");
    }

    #[test]
    fn removing_gives_the_whole_directory_back() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());

        let path = handoffs.directory(7).unwrap();
        std::fs::write(path.join(HANDOFF), "# What we settled\n").unwrap();

        handoffs.remove(7);
        assert!(!path.exists());

        // A Conversation aborted twice, which is not an error anywhere else
        // either.
        handoffs.remove(7);
    }
}
