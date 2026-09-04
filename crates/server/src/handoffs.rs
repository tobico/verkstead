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
//! under the Data Directory beside the worktrees, reached inside every one of
//! its sandboxes at [`INSIDE`] — which is under `/tmp` because that is where a
//! session already expects to find what is neither the checkout nor its home.
//!
//! **Except where every Conversation would be reaching the same `/tmp`.** That
//! path is one directory per session on Linux, a tmpfs of the session's own
//! standing in a namespace nothing else is in; on a platform whose sandbox has
//! no mounts to make it with, `/tmp` is the machine's one real `/tmp` and a link
//! written there is a link every Conversation on the box shares. The second
//! session to start would repoint it and the first would write its handoff into
//! somebody else's directory — which its own policy refuses, so the document
//! would simply never be written. So there the directory is reached under the
//! session's own HOME instead: see [`inside`], and [`crate::sandbox::Homes`] for
//! why a HOME is a different directory per Conversation there and the same one
//! here.
//!
//! Windows is that second kind twice over: nothing is bound there either, and
//! `/tmp` is not a path that machine has at all. So it is the HOME answer, and
//! for the same reason the Mac's is.
//!
//! Taking it is a move rather than a read: the copy in the directory goes as the
//! Timeline gets one, so the Event is the handoff from then on and there is
//! never a second one to go stale.

use std::path::{Path, PathBuf};

use crate::platform::Platform;

/// Where a Conversation's directory is reached inside its sandbox, on the
/// platform whose sandbox can mount one there.
pub(crate) const INSIDE: &str = "/tmp/verkstead";

/// And what it is called under HOME where none can — see [`inside`], and this
/// module's own documentation for why the two are not one path.
const INSIDE_HOME: &str = "verkstead";

/// How a skill names the second of those, which is the one thing about this
/// path that cannot be an absolute name.
///
/// A skill that sends a session to a path has to name one it can open, and this
/// one is a different directory per Conversation — so it is named through the
/// variable that already differs per Conversation rather than spelled out. The
/// skills are written out once for the whole installation and a Conversation's
/// id is not known then, so there is nothing else a fixed string could say. See
/// [`crate::skills::said_at`], which is what puts this in the text.
pub(crate) const SAID_INSIDE_HOME: &str = "$HOME/verkstead";

/// Where a session whose HOME is `home` reaches its handoff directory.
///
/// A `Platform` rather than a `cfg`, for the reason [`Platform`] is a value at
/// all: the arm this machine will never run is still an arm a test on it can
/// ask for, and this one decides a path a session is told about in prose.
pub(crate) fn inside(platform: Platform, home: &Path) -> PathBuf {
    match platform {
        Platform::MacOs | Platform::Windows => crate::sandbox::under(home, INSIDE_HOME),
        Platform::Linux => PathBuf::from(INSIDE),
    }
}

/// And how a skill names it on the installation this platform is running: the
/// path itself where a mount makes one directory per session of it, and the
/// variable it has to be said through where none does — see
/// [`SAID_INSIDE_HOME`].
pub(crate) fn said(platform: Platform) -> &'static str {
    match platform {
        Platform::MacOs | Platform::Windows => SAID_INSIDE_HOME,
        Platform::Linux => INSIDE,
    }
}

/// What the handoff document is called inside it.
const HANDOFF: &str = "handoff.md";

/// And what a session's own prompt is called, where it is written down rather
/// than handed over as an argument — see [`Handoffs::wrote_prompt`].
///
/// In this directory rather than anywhere else for the reason the handoff is in
/// it: it is the one writable place a Conversation has that git will never see,
/// and every session of the Conversation reaches it.
///
/// One name per Conversation rather than one per session. A Conversation runs
/// one session at a time and the document beside this is one per Conversation
/// too, so a second name would be a file nothing ever went back to.
const PROMPT: &str = "prompt.md";

/// What a session is started on where its prompt is in a file: one line, naming
/// the file by the path the session will open it at.
///
/// **One line, because it is one argument.** The whole point of writing the
/// prompt down is that the command line has a length a long handoff exceeds —
/// see [`Handoffs::wrote_prompt`] — so what goes on it instead has to be short
/// whatever the prompt was.
///
/// The path is in backticks, which is how the rest of the prompts here name a
/// path and is also what lets something reading the line rather than
/// understanding it find the file: a stand-in agent in the Windows suite reads
/// the name out from between them.
fn started_on(prompt_file: &Path) -> String {
    format!(
        "Your instructions for this session are in the file `{}`. Read the whole \
         of that file and follow it: it is the message you would otherwise have \
         been started on, and nothing else here repeats what it says.",
        prompt_file.display(),
    )
}

/// The two of those together, which is the path the grilling skill names and the
/// session writes.
///
/// Nothing in the server says it: the skill is markdown and says it in prose,
/// and this module reads the file from the outside where it is somewhere else
/// entirely. It is here so that the tests can hold the skill's sentence and what
/// a sandbox makes to each other — the one place the two could drift apart is a
/// session writing a document nothing ever reads.
#[cfg(test)]
pub(crate) const HANDOFF_INSIDE: &str = "/tmp/verkstead/handoff.md";

/// The directories Conversations' sessions are given outside their worktrees.
///
/// A root under the Data Directory and nothing else: which directory belongs to
/// which Conversation is its id, so there is nothing to keep in a table.
#[derive(Debug, Clone)]
pub struct Handoffs {
    root: PathBuf,
}

impl Handoffs {
    /// The root under `data_dir`, beside the worktrees — both are things
    /// Verkstead made rather than things the human pointed it at.
    pub fn under(data_dir: &Path) -> Handoffs {
        Handoffs {
            root: data_dir.join("handoffs"),
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

    /// Write `prompt` into `conversation_id`'s directory, and answer the one
    /// line a session is started on in its place.
    ///
    /// What this is for is a platform whose command line is too short to carry
    /// a prompt: Windows caps one at 32,767 characters, and an implementing
    /// session's prompt carries the whole handoff document inlined. So the
    /// prompt goes to a file there and the argument is a sentence naming it —
    /// see [`started_on`], and [`crate::sessions::Agents::argv`], which is
    /// where the platform decides between the two.
    ///
    /// `inside` is where this directory is reached from within the session —
    /// see [`inside`] — because the line has to name the file by the path the
    /// *session* will open it at rather than the one the server wrote it at.
    /// The two are one directory on the far side of a junction.
    ///
    /// `None` where the directory could not be made or the file could not be
    /// written, which is a session that has nothing to be started on: the
    /// caller starts none, the way it starts none for a sandbox it could not
    /// build.
    ///
    /// **This blocks**, and is called from where the sandbox is built for that
    /// reason.
    pub(crate) fn wrote_prompt(
        &self,
        conversation_id: i64,
        inside: &Path,
        prompt: &str,
    ) -> Option<String> {
        let path = self.directory(conversation_id)?.join(PROMPT);

        if let Err(error) = std::fs::write(&path, prompt) {
            tracing::error!(
                error = ?error,
                conversation_id,
                path = %path.display(),
                "a session's prompt could not be written down, so it has nothing to be \
                 started on"
            );
            return None;
        }

        Some(started_on(&inside.join(PROMPT)))
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

    /// And where no mount can make that path, it is under the session's own
    /// HOME — which is what keeps two Conversations running at once out of each
    /// other's document, their HOMEs being different directories.
    #[test]
    fn a_platform_with_no_mounts_reaches_it_under_the_home_it_was_given() {
        let sevens = Path::new("/data/homes/7");
        let eights = Path::new("/data/homes/8");

        assert_eq!(
            inside(Platform::MacOs, sevens),
            Path::new("/data/homes/7/verkstead"),
            "one Conversation's directory is under its own HOME",
        );
        assert_ne!(
            inside(Platform::MacOs, sevens),
            inside(Platform::MacOs, eights),
            "and so it is not the one the Conversation beside it is writing into",
        );

        assert_eq!(
            inside(Platform::Windows, sevens),
            Path::new("/data/homes/7/verkstead"),
            "and Windows the same, having no mounts either and no `/tmp` at all",
        );

        assert_eq!(
            inside(Platform::Linux, sevens),
            Path::new(INSIDE),
            "where a tmpfs of the session's own makes that path one directory \
             per session already",
        );

        assert_eq!(
            said(Platform::Windows),
            said(Platform::MacOs),
            "and a skill names it the same way on both: through the variable \
             that already differs per Conversation",
        );
    }

    /// The variable a skill names it through is that same path, said the one way
    /// a string written before any Conversation exists can say it.
    #[test]
    fn what_a_skill_says_is_what_a_sessions_home_makes() {
        assert_eq!(
            SAID_INSIDE_HOME.replace("$HOME", "/data/homes/7"),
            inside(Platform::MacOs, Path::new("/data/homes/7"))
                .display()
                .to_string(),
        );
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

    /// A prompt written down goes in the directory beside the handoff, and what
    /// comes back names it by the path the session opens it at rather than the
    /// one the server wrote it at. The two are one directory on the far side of
    /// a junction, and only one of them is a path the session has.
    #[test]
    fn a_prompt_written_down_is_named_by_the_path_the_session_opens_it_at() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());
        let inside = inside(Platform::Windows, Path::new("C:\\verkstead\\homes\\7"));

        let started_on = handoffs
            .wrote_prompt(7, &inside, "# Rate limiting\n")
            .expect("a prompt to be written down");

        assert_eq!(
            std::fs::read_to_string(state.path().join("handoffs/7/prompt.md")).unwrap(),
            "# Rate limiting\n",
        );
        assert!(
            started_on.contains(&inside.join(PROMPT).display().to_string()),
            "{started_on:?} does not name the file the session will open",
        );
        assert!(
            !started_on.contains("Rate limiting"),
            "and says nothing of the prompt itself: {started_on:?}",
        );
    }

    /// And it is written over as each session of the Conversation starts, so
    /// what the file holds is what the session running was started on.
    #[test]
    fn the_next_sessions_prompt_is_the_one_in_the_directory() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());
        let inside = Path::new("/home/verkstead/verkstead");

        handoffs
            .wrote_prompt(7, inside, "# Rate limiting\n")
            .unwrap();
        handoffs
            .wrote_prompt(7, inside, "# What we settled\n")
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(state.path().join("handoffs/7/prompt.md")).unwrap(),
            "# What we settled\n",
        );
    }

    #[test]
    fn removing_gives_the_whole_directory_back() {
        let state = tempfile::tempdir().unwrap();
        let handoffs = Handoffs::under(state.path());

        let path = handoffs.directory(7).unwrap();
        std::fs::write(path.join(HANDOFF), "# What we settled\n").unwrap();

        handoffs.remove(7);
        assert!(!path.exists());

        // A Conversation closed twice, which is not an error anywhere else
        // either.
        handoffs.remove(7);
    }
}
