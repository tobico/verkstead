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
//!
//! **And they outlive everything but the delete.** Closing a Conversation takes
//! its Worktree and its handoff directory and leaves this one alone — a Steer
//! can bring a Closed Conversation back to life, and a file cannot be made
//! again the way a checkout can — and a Trim leaves it for a reason of its own:
//! what a Trim takes is the bulk a session produced, and these are the human's
//! own input. The Cleanup's delete is the one thing that removes a directory,
//! and [`sweeping`] at startup is the backstop under it.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::platform::Platform;
use crate::store;

/// How large one attached file may be.
///
/// Generous, because what the human has to hand is a screenshot or an export
/// rather than a paragraph — and finite, because the body is read into memory
/// to be written out. The route carries this as its own body limit over the
/// router's default, the way the Question Set route raises its own.
///
/// **Decimal**, so that the cap is the number the composer names in its
/// refusal and the number a pill would have said of the file that was
/// refused — see `sized` in `crate::skills`, which is what every size in this
/// product is said in. A binary cap and a decimal sentence would be a human
/// told *larger than 32 MB* about a 33 MB file that was taken.
pub const MAX_BYTES: usize = 32 * 1000 * 1000;

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

    /// Give a Conversation's whole directory back, with everything in it.
    ///
    /// The Cleanup's delete and nothing else — see [`crate::cleanup`], the one
    /// point Verkstead forgets a Conversation for good. Closing leaves the
    /// directory exactly where it is: a Steer can bring a Closed Conversation
    /// back to life, and a file cannot be made again the way a Worktree can.
    ///
    /// Nothing to refuse with, which is [`crate::handoffs::Handoffs::remove`]'s
    /// shape and is a decision rather than a copy of it: what this follows is a
    /// delete that has already emptied the record, so a directory that will not
    /// go is a line in the log rather than a Conversation half-forgotten. What
    /// is left behind is taken at the next start — see [`sweeping`], the
    /// backstop written for exactly this.
    ///
    /// A directory that was never made is nothing to remove, and that is most
    /// Conversations: one nothing was ever attached to has no directory at all.
    pub(crate) fn remove(&self, conversation_id: i64) {
        let path = self.directory(conversation_id);

        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::error!(
                error = ?error,
                conversation_id,
                path = %path.display(),
                "a Conversation's attachments directory could not be removed, so it was \
                 deleted around it"
            ),
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

/// Sweep the attachments root once, as the server comes up.
///
/// Where the orphans come from is two places, and neither leaves anything that
/// would ever come back for the directory on its own. The Cleanup's delete is
/// the one thing that removes one, and a delete that could not have it logged
/// the path and deleted the rows anyway — see [`Attachments::remove`]. And a
/// database restored from before a file was attached names none of what the
/// machine still has. Nothing else is ever going to look at either.
///
/// Started rather than waited on, [`crate::worktrees::at_startup`]'s shape: the
/// directories this is deciding about are ones nothing is racing it for, the
/// keep-set being read after the listing rather than before it.
pub(crate) fn at_startup(state: &crate::AppState) {
    let pool = state.pool.clone();
    let data = state.data_dir.clone();

    tokio::spawn(async move { swept(&pool, &data).await });
}

/// The same sweep off what it needs rather than off the whole of the server:
/// the store to ask, and the Data Directory the root is under.
///
/// **The directory is read first and the record after it**, which is the other
/// way round from [`crate::worktrees::swept`] and is the whole of why this one
/// needs no lock. A file is only ever attached to a Conversation that is
/// already in the record, so a directory the listing found belongs to a row
/// that was written before the listing ran — and a directory made after it is
/// not a candidate at all, this pass having already decided what it is working
/// over. The worktrees' sweep is the other way because a checkout is made
/// before the record names it, and it holds a lock across that window instead.
///
/// **And nothing is deleted on a reading that failed.** A store error read as
/// an empty keep-set would be every Conversation there is, so the error ends
/// the pass and the orphans wait for the next start — the sweeps' rule all the
/// way through: the unrecoverable mistake is deleting what somebody still has,
/// and an orphan left behind is swept again.
async fn swept(pool: &sqlx::SqlitePool, data: &Path) {
    // A router with no Data Directory has nowhere to have put a file, and the
    // empty path would resolve to the working directory — which is somebody
    // else's. See [`crate::nowhere`].
    if data.as_os_str().is_empty() {
        return;
    }

    let root = Attachments::under(data).root;

    // Off the runtime's threads, as the deletions below are: a directory
    // listing blocks, however little there is in it.
    let listing = root.clone();

    let found = match tokio::task::spawn_blocking(move || candidates(&listing)).await {
        Ok(found) => found,
        Err(error) => {
            tracing::error!(error = ?error, "listing the attachments directories failed");
            return;
        }
    };

    // Nothing there, or nothing readable. Either way there is no keep-set worth
    // asking the store for.
    let Some(found) = found else {
        return;
    };

    if found.is_empty() {
        return;
    }

    let kept = match store::recorded_conversations(pool).await {
        Ok(kept) => kept,
        Err(error) => {
            tracing::error!(
                error = ?error,
                "reading which Conversations the record still has failed, so no attached files are being swept",
            );
            return;
        }
    };

    if let Err(error) = tokio::task::spawn_blocking(move || sweeping(&root, &found, &kept)).await {
        tracing::error!(error = ?error, "sweeping the orphaned attachments failed");
    }
}

/// What is under the attachments root, as immediate children of it — or `None`
/// where there is nothing to sweep because the directory could not be read.
///
/// **Every candidate comes out of reading this one directory**, and nothing
/// else is ever a candidate: no path here is built from a record, from a
/// request or from anything but this listing, which is what makes the boundary
/// below checkable at all.
///
/// A root that is not there is a Verkstead nothing has ever been attached to —
/// most of them, and not worth a word in the log.
fn candidates(root: &Path) -> Option<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::error!(
                error = ?error,
                path = %root.display(),
                "the attachments directory could not be read, so nothing is being swept",
            );
            return None;
        }
    };

    let mut found = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::error!(error = ?error, path = %root.display(), "an entry of the attachments directory could not be read, so it is being left alone");
                continue;
            }
        };

        let path = entry.path();

        // What `read_dir` hands back is an entry of this directory, said out
        // loud rather than trusted: what follows deletes, and a candidate that
        // is not a child of the directory that was listed is one no check here
        // describes.
        if path.parent() != Some(root) {
            tracing::error!(path = %path.display(), "an attachments entry is not a child of the attachments directory, so it is being left alone");
            continue;
        }

        found.push(path);
    }

    Some(found)
}

/// And the deciding: everything `found` holds that no Conversation in `kept`
/// names, taken away. Hands back what it deleted.
///
/// **A directory is kept exactly when a Conversation of that id is in the
/// record**, which is [`Attachments::directory`]'s naming read backwards and is
/// the whole of the rule — whatever state the Conversation is in. A Closed one
/// keeps its files for the Steer that brings it back and a Trimmed one keeps
/// them because they are the human's own input; only the Cleanup's delete takes
/// a Conversation out of the record at all. Everything else under this
/// directory is orphaned by definition: it is Verkstead's own Data Directory,
/// made for exactly one thing.
///
/// Every reading an entry is put through asks one question — *is this certainly
/// an orphan under this directory?* — and anything short of a yes leaves the
/// entry where it is. A reading that failed is one of those.
fn sweeping(root: &Path, found: &[PathBuf], kept: &[i64]) -> Vec<PathBuf> {
    let mut swept = Vec::new();

    // Resolved once, and it is the whole of the boundary: every deletion below
    // is checked against this rather than against the path it was joined from,
    // so a link anywhere above cannot move it. A root that will not resolve is
    // one that is not there, and there is nothing in it to sweep.
    let Ok(resolved_root) = root.canonicalize() else {
        return swept;
    };

    // The names Verkstead would have given those Conversations' directories,
    // made the one way they are ever made — see [`Attachments::directory`], so
    // that the keep-set and the naming cannot drift apart.
    let names: Vec<String> = kept.iter().map(i64::to_string).collect();

    for path in found {
        if path
            .file_name()
            .is_some_and(|name| names.iter().any(|kept| name == kept.as_str()))
        {
            continue;
        }

        // A link is deleted as a link and never followed. `remove_dir_all` on
        // one walks what is on the other end, which is by definition somewhere
        // else — and somewhere else is the one place nothing here may touch.
        let linked = match path.symlink_metadata() {
            Ok(metadata) => metadata.is_symlink(),
            Err(error) => {
                tracing::error!(error = ?error, path = %path.display(), "an attachments entry could not be read, so it is being left alone");
                continue;
            }
        };

        if linked {
            match std::fs::remove_file(path) {
                Ok(()) => {
                    tracing::info!(path = %path.display(), "a link in the attachments directory that no Conversation names was removed");
                    swept.push(path.clone());
                }
                Err(error) => {
                    tracing::error!(error = ?error, path = %path.display(), "a link in the attachments directory could not be removed");
                }
            }

            continue;
        }

        // And where it actually is, which is what the boundary is checked
        // against. Nothing that will not resolve is deleted: a reading that
        // failed is not a reading that says *outside*.
        let resolved = match path.canonicalize() {
            Ok(resolved) => resolved,
            Err(error) => {
                tracing::error!(error = ?error, path = %path.display(), "an attachments entry could not be resolved, so it is being left alone");
                continue;
            }
        };

        if resolved.parent() != Some(resolved_root.as_path()) {
            tracing::error!(path = %path.display(), resolved = %resolved.display(), "an attachments entry resolves outside the attachments directory, so it is being left alone");
            continue;
        }

        // A stray file under here is as unrecorded as a stray directory and goes
        // the same way; what it must not do is fail a `remove_dir_all` and be
        // reported as a directory that would not go.
        let taken = match path.is_dir() {
            true => std::fs::remove_dir_all(path),
            false => std::fs::remove_file(path),
        };

        match taken {
            Ok(()) => {
                tracing::info!(path = %path.display(), "attached files that no Conversation names were deleted");
                swept.push(path.clone());
            }
            Err(error) => {
                tracing::error!(error = ?error, path = %path.display(), "orphaned attached files could not be deleted");
            }
        }
    }

    swept
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cap is a round number in the words this product says sizes in, which
    /// is the whole of what lets the composer's refusal name it: *larger than
    /// 32 MB* is true of everything this refuses and of nothing it takes. A
    /// binary cap under that sentence would be a 33 MB file quietly attached
    /// after the human was told it would not be.
    #[test]
    fn the_cap_is_the_number_the_refusal_names() {
        assert_eq!(crate::skills::sized(MAX_BYTES as i64), "32.0 MB");
    }

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
    fn a_whole_directory_is_given_back_at_a_delete() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        attachments.keep(7, "notes.md", b"first").unwrap();
        attachments.keep(7, "wireframe.png", b"second").unwrap();

        attachments.remove(7);

        assert!(
            !state.path().join("attachments/7").exists(),
            "the delete takes the directory and everything in it",
        );
        assert!(
            state.path().join("attachments").exists(),
            "and leaves the root every other Conversation's is under",
        );

        // A Conversation nothing was ever attached to has no directory at all,
        // and that is most of them: nothing to remove and nothing to say.
        attachments.remove(9);
    }

    /// The whole of the sweep's rule in one run: the directory a Conversation
    /// in the record names stays, and the one no Conversation names goes.
    #[test]
    fn a_directory_a_conversation_names_survives_a_sweep_and_one_it_does_not_goes() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());
        let root = state.path().join("attachments");

        attachments
            .keep(7, "notes.md", b"a live Conversation's")
            .unwrap();
        attachments.keep(9, "notes.md", b"a deleted one's").unwrap();

        let found = candidates(&root).expect("the root is there to read");
        let swept = sweeping(&root, &found, &[7]);

        assert_eq!(swept, vec![root.join("9")]);
        assert!(
            root.join("7/notes.md").exists(),
            "the record still has Conversation 7",
        );
        assert!(
            !root.join("9").exists(),
            "and nothing in the record names Conversation 9",
        );
    }

    /// Everything else under the root goes as well. It is Verkstead's own Data
    /// Directory, made for exactly one thing: a directory named after no
    /// Conversation at all and a file that was never anything are both something
    /// nobody named.
    #[test]
    fn what_was_never_a_conversations_directory_is_swept_too() {
        let state = tempfile::tempdir().unwrap();
        let root = state.path().join("attachments");

        std::fs::create_dir_all(root.join("a-directory/deeper")).unwrap();
        std::fs::write(root.join("a-directory/deeper/notes.md"), "left\n").unwrap();
        std::fs::write(root.join("a-file"), "left\n").unwrap();

        let found = candidates(&root).expect("the root is there to read");
        let mut swept = sweeping(&root, &found, &[7]);
        swept.sort();

        assert_eq!(swept, vec![root.join("a-directory"), root.join("a-file")]);
    }

    /// A link is deleted as a link and never followed. What is on the other end
    /// is by definition outside the one directory this may delete inside, so
    /// walking it would be the whole of the mistake this is written against.
    ///
    /// On the platforms where a test may make a link at all — Windows wants a
    /// privilege for one, and the sweep's own reasoning is the same there.
    #[cfg(unix)]
    #[test]
    fn a_link_under_the_attachments_root_goes_as_a_link() {
        let state = tempfile::tempdir().unwrap();
        let root = state.path().join("attachments");

        std::fs::create_dir_all(&root).unwrap();

        let elsewhere = state.path().join("somebody-elses");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("their-work.md"), "not Verkstead's\n").unwrap();

        let link = root.join("9");
        std::os::unix::fs::symlink(&elsewhere, &link).unwrap();

        let found = candidates(&root).expect("the root is there to read");
        let swept = sweeping(&root, &found, &[]);

        assert_eq!(swept, vec![link.clone()]);
        assert!(
            link.symlink_metadata().is_err(),
            "the link itself is what was removed",
        );
        assert!(
            elsewhere.join("their-work.md").exists(),
            "and what it pointed at is untouched",
        );
    }

    /// A root nothing has ever been attached under is nothing to sweep, which is
    /// most installations and is not worth a word.
    #[test]
    fn a_root_that_is_not_there_sweeps_nothing() {
        let state = tempfile::tempdir().unwrap();

        assert_eq!(candidates(&state.path().join("attachments")), None);
    }

    /// A store that will not answer deletes nothing. An error read as an empty
    /// keep-set would be every Conversation there is, which is the one mistake
    /// here that cannot be undone by sweeping again.
    #[tokio::test]
    async fn a_keep_set_that_could_not_be_read_sweeps_nothing() {
        let state = tempfile::tempdir().unwrap();
        let attachments = Attachments::under(state.path());

        attachments.keep(9, "notes.md", b"nobody's").unwrap();

        let pool = store::open_database(&state.path().join("verkstead.db"))
            .await
            .unwrap();

        // The store taken out from under it: any way of failing would do, and
        // this is the one a test can arrange without touching the schema.
        pool.close().await;

        swept(&pool, state.path()).await;

        assert!(
            state.path().join("attachments/9/notes.md").exists(),
            "a reading that failed is not a reading that says delete",
        );
    }

    /// And a server with no Data Directory sweeps nothing. The empty path would
    /// resolve to the working directory, which is somebody else's.
    #[tokio::test]
    async fn a_server_with_no_data_directory_sweeps_nothing() {
        let state = tempfile::tempdir().unwrap();
        let pool = store::open_database(&state.path().join("verkstead.db"))
            .await
            .unwrap();

        swept(&pool, Path::new("")).await;
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
