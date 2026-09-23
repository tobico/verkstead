//! Reading the Worktrees a Conversation has, for the Code pane: the roots its
//! tree stands on, one folder of one of them at a time, one file of one of
//! those opened — and that file written back.
//!
//! **The server reads as itself, with no Sandbox in front of it**
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *The server reads and
//! writes the Worktree, outside the Sandbox*). The Sandbox bounds what a
//! *session* may reach; the workbench is the human, who already reaches the
//! whole Worktree through a terminal in it, and the server already reads it on
//! their behalf — the Diff on a Set, the backlog, the roadmaps. What keeps a
//! session out of this is the Workbench Key over `/api/ui/`, which is what
//! these endpoints are under.
//!
//! **What bounds it is the roots** — see [`roots`]: the Conversation's own
//! Worktree and each companion's, which is the same thing that bounds a shell.
//! Every path that arrives here is measured against them before anything is
//! opened, and is measured twice: once as it was spelled, and again after it is
//! resolved, so that neither a `..` nor a symlink out of a checkout is a way
//! through.
//!
//! **A folder listing is one folder**, read when it is expanded and never a
//! walk — the shape [`crate::browsing`] answers a path field with, and for the
//! same reason. Nothing here recurses, and a folder holding ten thousand files
//! costs one `read_dir`.
//!
//! **Git is asked what is ignored** rather than having the answer
//! reimplemented: a checkout's `.gitignore`, the repository's own excludes and
//! the machine's global one are between them the only account of what a tree
//! should not show, and `git check-ignore` is the one thing that holds all
//! three. A root git will not answer about is listed whole rather than not at
//! all — what that costs is a `target/` in the tree, and what refusing would
//! cost is the tree.
//!
//! **A file is read whole, and says what kind of thing it was** — see [`read`]:
//! text, an image, a binary the server will not send, or a file over
//! [`MAX_BYTES`]. Text carries a version, which is a hash of the bytes it was
//! read as and what a write will name itself as being over (ADR 0019,
//! *Versioned reads, and a stale write is refused*).
//!
//! **And a write names that version** — see [`write`]: the file on disk is
//! hashed afresh, and one that has moved since the read is refused, which is
//! what draws the *Reload* / *Keep mine* bar in front of the human — both
//! halves of which come back here for a fresh read. A root that takes no writes
//! refuses before the disk is touched at all.
//!
//! The write goes into the file that is already there rather than through a
//! temporary file renamed over it, which is what keeps the checkout's own mode
//! on it — and, on Windows, the entries a session was granted on the Worktree
//! and inherited down to that file, so that what the server writes as the human
//! is a file the session can still read (ADR 0019, *The server reads and writes
//! the Worktree, outside the Sandbox*).
//!
//! **Nothing here refuses by status code**, the way registering a Repo refuses
//! and the way a browse's listing does: each refusal is a sentence the tree
//! draws where its rows would be — see [`verkstead_render::FolderListing`].
//!
//! **Not a record.** Nothing here writes to the *store*, puts anything on a
//! Timeline or reaches a Share — a save is the human's own hand in their own
//! checkout, and the record of it is the commit they make afterwards. It is a
//! reading of the disk, made afresh every time the tree asks — which is what an expand is, until the watcher of stage
//! 04 tells the page the disk has moved.

use std::collections::HashSet;
use std::path::{Component, Path};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};
use verkstead_render::{FileReading, FileRoot, FileWritten, FolderEntry, FolderListing};

use crate::repos::feeding;
use crate::resolved::{Resolved, resolve};
use crate::store;

/// The name no root shows and no path may be under: a repository's own insides.
///
/// A directory in a clone and a file in a worktree, and neither is the work.
const GIT: &str = ".git";

/// The Worktrees Code draws a root apiece for: the Conversation's own first,
/// then each companion's in the order the Conversation carries them.
///
/// **A read-only companion is among them, marked**, which is where this parts
/// company with [`crate::diffs::writable`] — the module this follows the shape
/// of. That one is composing a Diff and drops a read-only companion for what it
/// is: detached, bound read-only, with nothing uncommitted to find. A root is
/// something to *read*, and a companion checked out beside the work is worth
/// reading whichever way it was bound.
///
/// A repository with no Worktree is not among them: a Conversation before
/// grilling or after closing has none, which is the ordinary state rather than
/// a missing record.
pub(crate) fn roots(conversation: &store::Conversation) -> Vec<FileRoot> {
    let mut roots = Vec::new();

    if let Some(worktree) = &conversation.worktree {
        roots.push(FileRoot {
            repo: conversation.repo.name.clone(),
            path: worktree.display().to_string(),
            own: true,
            writable: true,
        });
    }

    for companion in &conversation.companions {
        if let Some(worktree) = &companion.worktree {
            roots.push(FileRoot {
                repo: companion.repo.name.clone(),
                path: worktree.display().to_string(),
                own: false,
                writable: companion.mode == store::CompanionMode::ReadWrite,
            });
        }
    }

    roots
}

/// What the folder at `path` holds, or the named reason it holds nothing.
///
/// Blocking: a directory is opened, every entry in it is asked what it is, and
/// git is asked which of them are ignored.
///
/// The order the checks come in is the order the sentences rank in. Which root
/// a path is under is settled first, because every other answer is about a path
/// that has one; `.git` next, because a path under it is refused whether or not
/// anything is there; and the disk is asked only once a path has been allowed.
pub(crate) fn folder(roots: &[FileRoot], path: &Path) -> FolderListing {
    let (root, real) = match bound(roots, path) {
        Bound::Inside { root, real } => (Path::new(&root.path), real),
        Bound::Outside => return FolderListing::Outside,
        Bound::UnderGit => return FolderListing::UnderGit,
        Bound::RootGone => return FolderListing::RootGone,
        Bound::Missing => return FolderListing::Missing,
    };

    if !real.is_dir() {
        return FolderListing::NotAFolder;
    }

    let reading = match std::fs::read_dir(&real) {
        Ok(reading) => reading,
        Err(error) => {
            return FolderListing::Unreadable {
                why: format!("the server cannot read it: {error}"),
            };
        }
    };

    // Built out of the path as it was asked for rather than out of the resolved
    // directory the entries were read from, so that every path the tree holds
    // is spelled the way its root is — see [`FolderEntry::path`].
    let mut entries: Vec<FolderEntry> = reading
        .filter_map(|read| {
            // An entry that will not read is left out rather than failing the
            // listing, and so is one whose name is not UTF-8: a row that cannot
            // be expanded or opened is worse than a row that is not there. The
            // browse's rule, for the browse's reason.
            let name = read.ok()?.file_name().to_str()?.to_owned();

            if name == GIT {
                return None;
            }

            let at = path.join(&name);

            Some(FolderEntry {
                folder: at.is_dir(),
                path: at.display().to_string(),
                name,
            })
        })
        .collect();

    let ignored = ignored(root, &entries);
    entries.retain(|entry| !ignored.contains(&asked_about(entry)));
    ordered(&mut entries);

    FolderListing::Listed {
        path: path.display().to_string(),
        entries,
    }
}

/// What one file of one of those roots is, read — or the named reason it is not
/// drawn.
///
/// Blocking: a file is opened and read whole.
///
/// The bound is the folder's, measured by the same [`bound`] — the roots are
/// the whole of what this API may reach, and a file is reached no more widely
/// than a folder is. What follows it is the kinds: the size is asked before the
/// bytes are, so a file too large to open is never read; an image is named by
/// its extension and sent as bytes; and everything else is text or is not,
/// which is the bytes' own answer.
pub(crate) fn read(roots: &[FileRoot], path: &Path) -> FileReading {
    let (root, real) = match bound(roots, path) {
        Bound::Inside { root, real } => (root, real),
        Bound::Outside => return FileReading::Outside,
        Bound::UnderGit => return FileReading::UnderGit,
        Bound::RootGone => return FileReading::RootGone,
        Bound::Missing => return FileReading::Missing,
    };

    let about = match std::fs::metadata(&real) {
        Ok(about) => about,
        Err(error) => return unreadable(&error),
    };

    if about.is_dir() {
        return FileReading::NotAFile;
    }

    // Asked of the size before the bytes, which is the whole point of having a
    // cap: a file nothing is going to draw is a file nothing should read into
    // memory to find that out.
    if about.len() > MAX_BYTES {
        return FileReading::TooLarge;
    }

    let bytes = match std::fs::read(&real) {
        Ok(bytes) => bytes,
        Err(error) => return unreadable(&error),
    };

    // And again on what was read, a file being something a build may have grown
    // between the one call and the other.
    if bytes.len() as u64 > MAX_BYTES {
        return FileReading::TooLarge;
    }

    if let Some(media_type) = pictured(path) {
        return FileReading::Image {
            path: path.display().to_string(),
            media_type: media_type.to_owned(),
            base64: STANDARD.encode(&bytes),
        };
    }

    match texted(bytes) {
        // Hashed off the text rather than off what was read, those being the
        // same bytes: a `String` holds exactly the UTF-8 it was decoded from, so
        // this is the version of the file rather than of the reading of it, and
        // nothing is copied to say so.
        Some(text) => FileReading::Text {
            version: version(text.as_bytes()),
            path: path.display().to_string(),
            text,
            writable: root.writable,
        },
        None => FileReading::Binary,
    }
}

/// Put `text` at `path`, if the file there is still the one `over` was read
/// from.
///
/// Blocking: a file is opened and hashed, and then written.
///
/// The order the checks come in is the order the sentences rank in, and it is
/// the read's with one step of its own in it. The bound is settled first,
/// because every other answer is about a path that has one; then whether the
/// root takes writes at all, that being true of the root whatever is at the
/// path; and the file itself only after both.
///
/// **The version is the last thing asked and the write follows it at once.**
/// What is on the disk is hashed and compared against the version the read
/// handed over, and a file that has moved is refused (ADR 0019, *Versioned
/// reads, and a stale write is refused*). Between that hash and the write there
/// is a window nothing can close — a filesystem has no compare-and-swap — and
/// the window is microseconds against the minutes an editor is open, which is
/// the collision this is built to catch.
///
/// **The bytes go into the file that is there** rather than into a new file
/// renamed over it. A rename would give the Worktree a fresh inode with a fresh
/// mode — and, on Windows, a fresh ACL inherited from the directory rather than
/// the one the file already carries — where a write in place leaves everything
/// about the file but its contents exactly as the checkout made it.
pub(crate) fn write(roots: &[FileRoot], path: &Path, over: &str, text: &str) -> FileWritten {
    let (root, real) = match bound(roots, path) {
        Bound::Inside { root, real } => (root, real),
        Bound::Outside => return FileWritten::Outside,
        Bound::UnderGit => return FileWritten::UnderGit,
        Bound::RootGone => return FileWritten::RootGone,
        Bound::Missing => return FileWritten::Missing,
    };

    // The root's own flag rather than the file's mode, which is the same thing
    // the read said when it opened the editor read-only: a companion checked
    // out detached is there to be read, whatever its permissions happen to say.
    if !root.writable {
        return FileWritten::ReadOnly;
    }

    match std::fs::metadata(&real) {
        Ok(about) if about.is_dir() => return FileWritten::NotAFile,
        Ok(_) => {}
        Err(error) => return unwritable(&error),
    }

    match versioned(&real) {
        Ok(now) if now != over => return FileWritten::Stale,
        Ok(_) => {}
        Err(error) => return unwritable(&error),
    }

    match std::fs::write(&real, text) {
        Ok(()) => FileWritten::Written {
            version: version(text.as_bytes()),
        },
        Err(error) => unwritable(&error),
    }
}

/// The version the file at `real` has right now.
///
/// Read in blocks rather than whole, which is the one place this parts company
/// with [`read`] above: what is being hashed is whatever is on the disk, and a
/// build that turned the file somebody has open into a gigabyte of log is a
/// file to answer *stale* about rather than one to pull into memory first.
fn versioned(real: &Path) -> std::io::Result<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(real)?;
    let mut hashing = Sha256::new();
    let mut block = [0u8; 64 * 1024];

    loop {
        match file.read(&mut block)? {
            0 => break,
            read => hashing.update(&block[..read]),
        }
    }

    Ok(format!("{:x}", hashing.finalize()))
}

/// What the filesystem said of a write, worded as the sentence the bar draws.
fn unwritable(error: &std::io::Error) -> FileWritten {
    FileWritten::Unwritable {
        why: format!("the server cannot write it: {error}"),
    }
}

/// How large a file Code opens.
///
/// A few megabytes, which is the number ADR 0019 puts on it: what is over it is
/// a build artefact, a database or a log, and none of the three is something to
/// hand an editor in a browser. A terminal beside the tab opens any of them.
///
/// **Decimal**, so that the cap is the number the viewer's refusal names —
/// [`crate::attachments::MAX_BYTES`]'s rule, for its reason: a binary cap under
/// a decimal sentence is a human told *larger than 2 MB* about a file that was
/// opened.
pub(crate) const MAX_BYTES: u64 = 2 * 1000 * 1000;

/// The version a read carries: a hash of the bytes it read.
///
/// SHA-256, spelled hex. Of the bytes rather than of the text, and a hash
/// rather than a modification time: two writes inside one clock tick are
/// exactly the collision this exists to catch, and what a save is measured
/// against is what is on the disk.
fn version(bytes: &[u8]) -> String {
    let mut hashing = Sha256::new();
    hashing.update(bytes);

    format!("{:x}", hashing.finalize())
}

/// The media type to draw `path` as, where it names a picture.
///
/// Read off the extension rather than off the bytes, because what decides
/// whether a browser draws it is this string: a file named `.png` that is
/// something else is a broken picture either way, and sniffing would only move
/// which of the two kinds it is broken as.
///
/// The formats a browser draws and no more. An `.svg` is not among them — it is
/// XML, it opens in the editor as what it is, and a browser handed one as a
/// picture is a browser running whatever script is in it.
fn pictured(path: &Path) -> Option<&'static str> {
    let named = path.extension()?.to_str()?.to_ascii_lowercase();

    match named.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

/// `bytes` as text, where they are text.
///
/// Git's own reading of it, and for git's reason: a repository is full of files
/// named for nothing in particular, so what says whether one is text is what is
/// in it rather than what it is called. Valid UTF-8 with no NUL byte in it —
/// the NUL being what tells an object file from a source file that happens to
/// decode, and what git itself looks for.
fn texted(bytes: Vec<u8>) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }

    String::from_utf8(bytes).ok()
}

/// What the filesystem said, worded as the sentence the tab draws.
fn unreadable(error: &std::io::Error) -> FileReading {
    FileReading::Unreadable {
        why: format!("the server cannot read it: {error}"),
    }
}

/// Where `path` is against the roots, measured twice.
///
/// The one bound both halves of this API are behind, because a file is reached
/// no more widely than the folder it is in: which root a path is under is
/// settled first, `.git` next — a path under it is refused whether or not
/// anything is there — and the disk is asked only once a path has been allowed.
///
/// **A path that climbs is under nothing.** `..` is refused outright rather
/// than folded away, and so is a `.`: the tree only ever asks for paths it was
/// handed, so a path spelled with either is a request somebody wrote by hand,
/// and lexical folding is the step that gets a bound wrong. What a symlink out
/// of a checkout does is caught below, on the resolved pair.
fn bound<'a>(roots: &'a [FileRoot], path: &Path) -> Bound<'a> {
    if !path.is_absolute() || path.components().any(|part| !plain(&part)) {
        return Bound::Outside;
    }

    // The first root it is under, roots being directories nothing nests inside
    // another. A root itself is in itself: the tree asks for a root's own
    // listing the moment somebody expands it.
    let Some(root) = roots
        .iter()
        .find(|root| path.starts_with(Path::new(&root.path)))
    else {
        return Bound::Outside;
    };

    if inside_git(Path::new(&root.path), path) {
        return Bound::UnderGit;
    }

    // The root first, so that a Worktree that has gone says so rather than
    // reading as a folder that has: they are one filesystem answer and two
    // different things to be told.
    let Resolved::At(real_root) = resolve(Path::new(&root.path)) else {
        return Bound::RootGone;
    };

    let Resolved::At(real) = resolve(path) else {
        return Bound::Missing;
    };

    // And measured again, resolved — which is the half of the bound that a
    // symlink out of the checkout has to get past. The spelling was checked
    // above; this is where the filesystem's own answer is.
    if !real.starts_with(&real_root) {
        return Bound::Outside;
    }

    Bound::Inside { root, real }
}

/// What that measuring came to: the root the path is in and where it really is,
/// or which of the four refusals it is.
///
/// The refusals are named rather than folded into one because each of them is a
/// different sentence in front of the human — and they are the same four
/// whether what was asked for was a folder or a file, which is why this is one
/// shape rather than two.
enum Bound<'a> {
    Inside {
        root: &'a FileRoot,
        /// The path as the filesystem has it, which is what is opened from here
        /// on.
        real: std::path::PathBuf,
    },
    Outside,
    UnderGit,
    RootGone,
    Missing,
}

/// Whether a component is one that names a directory rather than one that moves
/// about among them — which on Windows leaves the prefix and the root as they
/// are, those being where an absolute path starts.
fn plain(part: &Component<'_>) -> bool {
    !matches!(part, Component::CurDir | Component::ParentDir)
}

/// Whether `path` is the git directory of `root`, or anything under it.
///
/// Read off the segments between the two rather than off the whole path,
/// because a `.git` above a root is the business of whoever put the checkout
/// there: a Worktree made under a directory of somebody's own called `.git`
/// would otherwise be a root nothing in it could be listed from.
fn inside_git(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .map(|under| under.components().any(|part| part.as_os_str() == GIT))
        .unwrap_or(false)
}

/// Which of `entries` git says are ignored in `root`, as the paths they were
/// asked about by.
///
/// One run of `check-ignore` for the whole folder, with the paths fed in on
/// stdin: what comes back is the ignored ones, and a folder of ten thousand
/// rows is still one process. Which is why the paths go in on stdin rather than
/// as arguments — a folder wide enough to be worth reading this way is a folder
/// wide enough to overrun a command line.
///
/// **Git rather than a reimplementation.** What a checkout ignores is its own
/// `.gitignore` files at every level, the repository's `info/exclude` and the
/// machine's global one, and the index besides — a tracked file is ignored by
/// nothing, whatever a pattern says. `check-ignore` is the one thing that holds
/// all of that, and it is already what tells [`crate::diffs`] which untracked
/// files are worth a patch.
///
/// A root git will not answer about — no git on the machine, a directory that
/// is not a repository, a repository mid-rebase — ignores nothing, so the
/// folder lists whole. What that costs is a `target/` drawn in the tree of a
/// checkout that was not a checkout, and what a refusal would cost is the tree.
fn ignored(root: &Path, entries: &[FolderEntry]) -> HashSet<String> {
    if entries.is_empty() {
        return HashSet::new();
    }

    let asking: String = entries
        .iter()
        .map(|entry| format!("{}\0", asked_about(entry)))
        .collect();

    // Exit 1 is the ordinary "nothing here is ignored" rather than a failure —
    // see [`crate::repos::feeding`], which is where the codes are read.
    feeding(root, &["check-ignore", "-z", "--stdin"], &asking, &[0, 1])
        .unwrap_or_default()
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

/// How one entry is spelled to git.
///
/// A folder is asked about with its trailing separator on, which is what makes
/// a rule written `target/` match it: without one git is being asked about a
/// *file* of that name, and `target/` matches no file. The answer comes back
/// spelled the way it was asked, which is what lets the two be matched up.
fn asked_about(entry: &FolderEntry) -> String {
    match entry.folder {
        true => format!("{}/", entry.path),
        false => entry.path.clone(),
    }
}

/// Folders first, then by name — the browse's order, which is what the eye
/// expects of a tree.
fn ordered(entries: &mut [FolderEntry]) {
    entries.sort_by(|left, right| {
        right
            .folder
            .cmp(&left.folder)
            .then_with(|| left.name.cmp(&right.name))
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A root at `path`, writable and the Conversation's own, which is what
    /// nearly every test here wants: what a root *says* is the roots endpoint's
    /// subject, and what a listing does with one is this.
    fn root(path: &Path) -> FileRoot {
        FileRoot {
            repo: "verkstead".to_owned(),
            path: path.display().to_string(),
            own: true,
            writable: true,
        }
    }

    /// The entries of a listing, or a panic saying what came back instead.
    fn listed(listing: FolderListing) -> Vec<FolderEntry> {
        match listing {
            FolderListing::Listed { entries, .. } => entries,
            other => panic!("expected a listing, got {other:?}"),
        }
    }

    /// The rows' names, which is what the tree draws.
    fn names(listing: FolderListing) -> Vec<String> {
        listed(listing).into_iter().map(|row| row.name).collect()
    }

    /// A git repository with one commit in it, ignoring `target/`.
    fn repository(at: &Path) -> PathBuf {
        std::fs::create_dir_all(at).unwrap();

        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "ada@example.com"],
            vec!["config", "user.name", "Ada"],
        ] {
            let ran = std::process::Command::new("git")
                .args(&args)
                .current_dir(at)
                .output()
                .unwrap();
            assert!(ran.status.success(), "git {args:?} failed");
        }

        std::fs::write(at.join(".gitignore"), "target/\n*.log\n").unwrap();
        std::fs::write(at.join("README.md"), "# a repository\n").unwrap();

        for args in [vec!["add", "-A"], vec!["commit", "-m", "first"]] {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(at)
                .output()
                .unwrap();
        }

        at.to_path_buf()
    }

    /// Folders first and then by name, which is the tree's own order.
    #[test]
    fn a_folder_lists_what_is_in_it_folders_first() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir(worktree.join("src")).unwrap();
        std::fs::create_dir(worktree.join("docs")).unwrap();
        std::fs::write(worktree.join("Cargo.toml"), "[package]\n").unwrap();

        assert_eq!(
            names(folder(&[root(&worktree)], &worktree)),
            ["docs", "src", ".gitignore", "Cargo.toml", "README.md"]
        );
    }

    /// And one folder, never a walk: what is under a folder that was not asked
    /// about is not in the answer.
    #[test]
    fn a_folder_is_read_alone_and_nothing_under_it_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("src/api")).unwrap();
        std::fs::write(worktree.join("src/lib.rs"), "").unwrap();

        assert_eq!(
            names(folder(&[root(&worktree)], &worktree.join("src"))),
            ["api", "lib.rs"]
        );
    }

    /// What git ignores is not in the tree, and neither is the git directory —
    /// the two things ADR 0019 takes out of it.
    #[test]
    fn what_git_ignores_and_the_git_directory_are_left_out() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir(worktree.join("target")).unwrap();
        std::fs::write(worktree.join("build.log"), "").unwrap();
        std::fs::write(worktree.join("kept.rs"), "").unwrap();

        assert_eq!(
            names(folder(&[root(&worktree)], &worktree)),
            [".gitignore", "README.md", "kept.rs"]
        );
    }

    /// And a folder wide enough that git answers about it while it is still
    /// being asked still comes back.
    ///
    /// `check-ignore` prints each ignored path as it reads, so a caller that
    /// wrote the whole question before reading a word of the answer would stop
    /// dead here: git blocks writing into a pipe nobody is draining, this side
    /// blocks writing into the one git has stopped reading, and the folder
    /// never opens. See [`crate::repos::feeding`], which writes and reads at
    /// once for this reason.
    ///
    /// Two thousand of them, under names long enough that the question is a
    /// good deal more than the sixty-four kilobytes a pipe holds however short
    /// the directory it is run in happens to be — the hang is a pipe filling,
    /// so a test that proved it would have to be sure of filling one. A build
    /// directory ignored file by file rather than by its name is an ordinary
    /// thing for a checkout to hold.
    #[test]
    fn a_folder_of_ignored_files_wide_enough_to_fill_a_pipe_still_answers() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        for number in 0..2_000 {
            let name = format!("build-artefact-{number:05}-of-a-run-nobody-committed.log");
            std::fs::write(worktree.join(name), "").unwrap();
        }

        std::fs::write(worktree.join("kept.rs"), "").unwrap();

        assert_eq!(
            names(folder(&[root(&worktree)], &worktree)),
            [".gitignore", "README.md", "kept.rs"]
        );
    }

    /// A path under none of the roots is refused, and says which of the
    /// refusals it is: the whole of what bounds the files API.
    #[test]
    fn a_path_outside_every_root_is_refused() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = held.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();

        assert_eq!(
            folder(&[root(&worktree)], &elsewhere),
            FolderListing::Outside
        );

        // And a path that climbs out of one, which is the same refusal by the
        // other route: the spelling is measured before anything is opened.
        assert_eq!(
            folder(&[root(&worktree)], &worktree.join("../elsewhere")),
            FolderListing::Outside
        );

        // A relative path is under nothing at all, there being no directory
        // here that one could be relative to.
        assert_eq!(
            folder(&[root(&worktree)], Path::new("src")),
            FolderListing::Outside
        );
    }

    /// And a symlink out of a root is refused too, which is the half of the
    /// bound the spelling cannot see.
    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_a_root_is_outside_it() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = held.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();

        let out = worktree.join("out");
        std::os::unix::fs::symlink(&elsewhere, &out).unwrap();

        assert_eq!(folder(&[root(&worktree)], &out), FolderListing::Outside);
    }

    /// A path under the git directory has its own sentence: it is inside a
    /// root, and what it names is the repository's insides rather than the work.
    #[test]
    fn a_path_under_the_git_directory_says_so() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        assert_eq!(
            folder(&[root(&worktree)], &worktree.join(".git")),
            FolderListing::UnderGit
        );
        assert_eq!(
            folder(&[root(&worktree)], &worktree.join(".git/refs/heads")),
            FolderListing::UnderGit
        );

        // And a `.git` *above* a root is nobody's business here: what is read
        // is the segments between the root and the path.
        let under = repository(&held.path().join(".git/worktree"));
        assert!(matches!(
            folder(&[root(&under)], &under),
            FolderListing::Listed { .. }
        ));
    }

    /// A Worktree that has gone is its own answer, told apart from a folder in
    /// one that has: two filesystem answers that are the same and two different
    /// things for a human to read.
    #[test]
    fn a_root_that_is_gone_and_a_folder_that_is_gone_are_told_apart() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        assert_eq!(
            folder(&[root(&worktree)], &worktree.join("never-made")),
            FolderListing::Missing
        );

        std::fs::remove_dir_all(&worktree).unwrap();

        assert_eq!(
            folder(&[root(&worktree)], &worktree),
            FolderListing::RootGone
        );
    }

    /// And a path naming a file is a listing that has gone as deep as it goes.
    #[test]
    fn a_file_is_not_a_folder_to_list() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        assert_eq!(
            folder(&[root(&worktree)], &worktree.join("README.md")),
            FolderListing::NotAFolder
        );
    }

    /// A directory that is no repository ignores nothing, and lists whole
    /// rather than refusing: what that costs is a row, and what refusing would
    /// cost is the tree.
    #[test]
    fn a_root_git_will_not_answer_about_lists_whole() {
        let held = tempfile::tempdir().unwrap();
        let worktree = held.path().join("worktree");
        std::fs::create_dir(&worktree).unwrap();
        std::fs::write(worktree.join("notes.md"), "").unwrap();

        assert_eq!(names(folder(&[root(&worktree)], &worktree)), ["notes.md"]);
    }

    /// The second root is read as its own: a path in a companion is measured
    /// against that companion rather than against the first one listed.
    #[test]
    fn each_root_bounds_its_own_paths() {
        let held = tempfile::tempdir().unwrap();
        let own = repository(&held.path().join("own"));
        let companion = repository(&held.path().join("companion"));
        std::fs::write(companion.join("askance.md"), "").unwrap();

        let roots = [
            root(&own),
            FileRoot {
                repo: "askance".to_owned(),
                path: companion.display().to_string(),
                own: false,
                writable: false,
            },
        ];

        assert_eq!(
            names(folder(&roots, &companion)),
            [".gitignore", "README.md", "askance.md"]
        );
    }

    /// A one-pixel PNG, which is what a test wants when the subject is that the
    /// bytes come back rather than what is in them.
    const PIXEL: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H', b'D',
        b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89,
    ];

    /// Text comes back as text, with a version — which is what a write will
    /// later name itself as being over.
    #[test]
    fn a_text_file_comes_back_with_what_is_in_it_and_a_version() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        let at = worktree.join("README.md");

        let FileReading::Text {
            path,
            version,
            text,
            writable,
        } = read(&[root(&worktree)], &at)
        else {
            panic!("expected text, got {:?}", read(&[root(&worktree)], &at));
        };

        assert_eq!(path, at.display().to_string());
        assert_eq!(text, "# a repository\n");
        assert!(writable);
        assert!(!version.is_empty());
    }

    /// And the version is of the bytes: it holds while the file does, and moves
    /// the moment anything writes to it. The whole of what a refused stale
    /// write stands on.
    #[test]
    fn the_version_holds_while_the_file_does_and_moves_when_it_moves() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let at = worktree.join("README.md");

        let first = versioned(read(&[root(&worktree)], &at));
        assert_eq!(first, versioned(read(&[root(&worktree)], &at)));

        std::fs::write(&at, "# a repository, rewritten\n").unwrap();

        assert_ne!(first, versioned(read(&[root(&worktree)], &at)));
    }

    /// A picture comes back as bytes to draw, named by what to draw it as.
    #[test]
    fn a_picture_comes_back_as_the_bytes_to_draw_it_from() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        let at = worktree.join("assets/icon.PNG");
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, PIXEL).unwrap();

        let FileReading::Image {
            media_type, base64, ..
        } = read(&[root(&worktree)], &at)
        else {
            panic!("expected an image, got {:?}", read(&[root(&worktree)], &at));
        };

        // The extension however it is spelled: a repository holds `.PNG` as
        // readily as `.png`, and what a browser draws is the media type.
        assert_eq!(media_type, "image/png");
        assert_eq!(STANDARD.decode(base64).unwrap(), PIXEL);
    }

    /// And an `.svg` is not one of them: it is XML, it opens in the editor as
    /// what it is, and a browser handed one as a picture is a browser running
    /// whatever script is in it.
    #[test]
    fn a_drawing_that_is_xml_opens_as_the_text_it_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        let at = worktree.join("logo.svg");
        std::fs::write(&at, "<svg xmlns=\"http://www.w3.org/2000/svg\"/>").unwrap();

        assert!(matches!(
            read(&[root(&worktree)], &at),
            FileReading::Text { .. }
        ));
    }

    /// Anything else binary is a line saying so rather than bytes on the wire:
    /// an object file in an editor is mojibake, and one in a tab is a megabyte
    /// across the wire for nothing.
    #[test]
    fn anything_else_binary_is_said_rather_than_sent() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        let at = worktree.join("verkstead.o");
        std::fs::write(&at, [0x7f, b'E', b'L', b'F', 0x02, 0x00, 0x01]).unwrap();

        assert_eq!(read(&[root(&worktree)], &at), FileReading::Binary);

        // What says which it is is the bytes rather than the name: a file
        // called nothing in particular is read as what is in it.
        let named = worktree.join("notes");
        std::fs::write(&named, "a note\n").unwrap();
        assert!(matches!(
            read(&[root(&worktree)], &named),
            FileReading::Text { .. }
        ));
    }

    /// And a file over the cap is its own answer, whether or not it is text:
    /// nothing is wrong with it, and a terminal beside the tab will open it.
    #[test]
    fn a_file_over_the_cap_is_not_read_at_all() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        let at = worktree.join("build.log");
        std::fs::write(&at, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();

        assert_eq!(read(&[root(&worktree)], &at), FileReading::TooLarge);
    }

    /// The cap is a round number in the words this product says sizes in, which
    /// is what lets the viewer's line name it: *larger than 2 MB* is true of
    /// everything this refuses and of nothing it opens.
    #[test]
    fn the_cap_is_the_number_the_line_names() {
        assert_eq!(crate::skills::sized(MAX_BYTES as i64), "2.0 MB");
    }

    /// A file in a read-only root opens read-only, which is the root's own flag
    /// rather than the file's mode: a companion checked out detached is a root
    /// to read whatever its permissions say.
    #[test]
    fn a_file_in_a_read_only_root_opens_read_only() {
        let held = tempfile::tempdir().unwrap();
        let companion = repository(&held.path().join("companion"));

        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        let FileReading::Text { writable, .. } = read(&roots, &companion.join("README.md")) else {
            panic!("expected text");
        };

        assert!(!writable);
    }

    /// And the refusals are the folder's, said about a file: the bound is one
    /// bound, and a file is reached no more widely than the folder it is in.
    #[test]
    fn a_file_is_bounded_the_way_a_folder_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = held.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("secrets"), "").unwrap();

        assert_eq!(
            read(&[root(&worktree)], &elsewhere.join("secrets")),
            FileReading::Outside
        );
        assert_eq!(
            read(&[root(&worktree)], &worktree.join("../elsewhere/secrets")),
            FileReading::Outside
        );
        assert_eq!(
            read(&[root(&worktree)], &worktree.join(".git/config")),
            FileReading::UnderGit
        );
        assert_eq!(
            read(&[root(&worktree)], &worktree.join("never-written")),
            FileReading::Missing
        );

        // A folder is a row the tree expands rather than a tab it opens, which
        // is its own sentence for whoever asked for one by hand.
        assert_eq!(read(&[root(&worktree)], &worktree), FileReading::NotAFile);

        std::fs::remove_dir_all(&worktree).unwrap();
        assert_eq!(
            read(&[root(&worktree)], &worktree.join("README.md")),
            FileReading::RootGone
        );
    }

    /// The version a reading carries, or a panic saying what came back instead.
    fn versioned(reading: FileReading) -> String {
        match reading {
            FileReading::Text { version, .. } => version,
            other => panic!("expected text, got {other:?}"),
        }
    }

    /// A save lands on disk, and answers with the version the file now has —
    /// which is what the next save over it names.
    #[test]
    fn a_write_over_the_version_that_was_read_lands_on_disk() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let at = worktree.join("README.md");

        let roots = [root(&worktree)];
        let was = versioned(read(&roots, &at));
        let wrote = write(&roots, &at, &was, "# rewritten\n");

        let FileWritten::Written { version } = wrote else {
            panic!("expected a write, got {wrote:?}");
        };

        assert_eq!(std::fs::read_to_string(&at).unwrap(), "# rewritten\n");

        // The version answered back is the file's own: a read of it now says the
        // same thing, which is what makes the next save a write over what is
        // really there.
        assert_eq!(version, versioned(read(&roots, &at)));
    }

    /// And a write over a version that has moved is refused — which is the
    /// whole of what the Reload / Keep mine bar is drawn from.
    #[test]
    fn a_write_over_a_version_that_has_moved_is_refused() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let at = worktree.join("README.md");

        let roots = [root(&worktree)];
        let was = versioned(read(&roots, &at));

        // The agent, writing the same file while the human had it open.
        std::fs::write(&at, "# the agent got here first\n").unwrap();

        assert_eq!(write(&roots, &at, &was, "# mine\n"), FileWritten::Stale);

        // Nothing was written: the collision is the point, and the human's text
        // is still theirs to decide about.
        assert_eq!(
            std::fs::read_to_string(&at).unwrap(),
            "# the agent got here first\n"
        );

        // And a save over the version a fresh read hands over — which is what
        // both halves of that bar make the next one — lands.
        let now = versioned(read(&roots, &at));

        assert!(matches!(
            write(&roots, &at, &now, "# mine\n"),
            FileWritten::Written { .. }
        ));
        assert_eq!(std::fs::read_to_string(&at).unwrap(), "# mine\n");
    }

    /// A read-only root takes no write at all, and says so before it touches
    /// the disk: the root's own flag, which is the same thing that opened the
    /// editor read-only.
    #[test]
    fn a_write_into_a_read_only_root_is_refused() {
        let held = tempfile::tempdir().unwrap();
        let companion = repository(&held.path().join("companion"));
        let at = companion.join("README.md");

        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        let was = versioned(read(&roots, &at));

        assert_eq!(write(&roots, &at, &was, "# mine\n"), FileWritten::ReadOnly);
        assert_eq!(
            std::fs::read_to_string(&at).unwrap(),
            "# a repository\n",
            "a read-only root is not written even over the version it was read at"
        );
    }

    /// And a write is bounded the way a read is: the roots are the whole of
    /// what this API may reach, whichever direction the bytes are going.
    #[test]
    fn a_write_is_bounded_the_way_a_read_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = held.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("secrets"), "nothing of this conversation's\n").unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            write(&roots, &elsewhere.join("secrets"), "", "mine\n"),
            FileWritten::Outside
        );
        assert_eq!(
            write(
                &roots,
                &worktree.join("../elsewhere/secrets"),
                "",
                "mine\n"
            ),
            FileWritten::Outside
        );
        assert_eq!(
            write(&roots, &worktree.join(".git/config"), "", "mine\n"),
            FileWritten::UnderGit
        );
        assert_eq!(
            write(&roots, &worktree.join("never-written"), "", "mine\n"),
            FileWritten::Missing
        );
        assert_eq!(
            write(&roots, &worktree, "", "mine\n"),
            FileWritten::NotAFile
        );

        // And the one outside it that was there a moment ago is still what it
        // was: a refused write writes nothing anywhere.
        assert_eq!(
            std::fs::read_to_string(elsewhere.join("secrets")).unwrap(),
            "nothing of this conversation's\n"
        );

        std::fs::remove_dir_all(&worktree).unwrap();
        assert_eq!(
            write(&roots, &worktree.join("README.md"), "", "mine\n"),
            FileWritten::RootGone
        );
    }
}
