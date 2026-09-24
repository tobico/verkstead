//! Reading the Worktrees a Conversation has, for the Code pane: the roots its
//! tree stands on, one folder of one of them at a time, one file of one of
//! those opened — and that file written back, a new one made beside it, or one
//! of them renamed.
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
//! **And a row of the tree makes one** — see [`make`]: an empty file or a
//! folder, named in full under a folder of a root, which is the first thing here
//! asked about a path that is not there yet. What is bounded is the folder above
//! it, and the making itself is the check for a name already taken — one
//! `create_new` rather than a look followed by a write, so that nothing the
//! agent writes in the window between them is overwritten.
//!
//! **And renames one** — see [`rename`]: a path in a root and the name it is to
//! have, which is a *name* rather than a path and is the whole of why a rename
//! cannot cross two roots. A root itself is refused, being a Worktree rather
//! than anything in one. The check for a name already taken is a look rather
//! than the making's atomic create, there being no portable rename that refuses
//! to overwrite — which is the one window in this module, and is said where it
//! is. What moves is the entry the tree drew rather than what it points at, the
//! way the removal below takes one away.
//!
//! **And takes one away** — see [`delete`]: a path in a root, and a folder
//! with everything under it. The rename's order of checks with nothing where the
//! name's would be, and the removal is of the entry the tree drew rather than of
//! what it points at — a link is unlinked, and the file at the end of it is
//! somebody else's. What asks the human first is the viewer, the confirm the app
//! puts in front of whatever cannot be taken back, so what arrives here has been
//! asked about.
//!
//! **And the palette is given every root at once** — see [`list`]: git's own
//! list of what each root holds, tracked and untracked-not-ignored, read afresh
//! every time the palette opens and capped at [`MAX_LISTED`] per root. The one
//! reading here that is not about a path somebody named, and so the one with no
//! bound to measure: what it answers *is* the paths, and a root git will not
//! answer about has none.
//!
//! **And the tree is given git's account of every root at once** — see
//! [`status`]: one `git status` per root, porcelain and NUL-separated, folded so
//! that a folder wears the strongest mark of anything under it. The palette's
//! shape said about what has *moved* rather than about what is there, and the
//! one reading here that a `commit` moves: a commit clears every mark in a
//! Worktree without touching a file.
//!
//! **And the watcher is told which directories to watch** — see [`watchable`]:
//! every non-ignored directory of a root, walked by the same ignore rules the
//! tree hides by, so that what follows the disk for the pane follows the part of
//! it the pane draws. The one reading here that is a walk, and it is one because
//! the platform's watcher takes a watch per directory and a `target/` would
//! spend a machine's whole allowance on rows nobody can see.
//!
//! **Nothing here refuses by status code**, the way registering a Repo refuses
//! and the way a browse's listing does: each refusal is a sentence the tree
//! draws where its rows would be — see [`verkstead_render::FolderListing`].
//!
//! **Not a record.** Nothing here writes to the *store*, puts anything on a
//! Timeline or reaches a Share — a save is the human's own hand in their own
//! checkout, and the record of it is the commit they make afterwards. It is a
//! reading of the disk, made afresh every time the tree asks — which is an
//! expand, a making or a rename, and every `files` Nudge the watcher announces:
//! the page re-reads the folders it has expanded, a listing apiece, because a
//! Nudge carries no payload and a folder is one `read_dir`.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};
use verkstead_render::{
    FileDeleted, FileList, FileListsView, FileMade, FileMark, FileReading, FileRenamed, FileRoot,
    FileStatus, FileStatusView, FileWritten, FolderEntry, FolderListing, Marked,
};

use crate::repos::{feeding, git};
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

    let asking: Vec<String> = entries.iter().map(asked_about).collect();
    let ignored = ignored(root, &asking);
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

/// Which of the two things a row's menu makes.
///
/// One function for both, because everything in front of the making is the
/// same: the path is bounded, the root is asked whether it takes writes, the
/// folder above it is asked whether it is a folder, and only the last call
/// differs. Two functions would be that paragraph written twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Making {
    /// An empty file, which opens as a tab the human types into.
    File,

    /// A folder, which opens nothing: there is nothing in it to open.
    Folder,
}

/// Make an empty file or a folder at `path`.
///
/// Blocking: a directory is resolved and one entry is created in it.
///
/// **The path is not there yet, which is what parts this from every other
/// function here.** [`bound`] resolves what it is handed and answers
/// [`Bound::Missing`] for a path with nothing at it, so what is bounded here is
/// the *folder* the new row goes in — and the name is the last segment of what
/// was asked for, which the bound above it has already refused a `..` or a `.`
/// in. So the four refusals a read has are all answers about the parent, and a
/// path whose own last segment is `.git` is caught by asking the whole path
/// about it.
///
/// The order the checks come in is [`write`]'s: the bound first, because every
/// other answer is about a path that has one; then whether the root takes writes
/// at all; then the folder itself.
///
/// **And the making is the check for a name already taken.** `create_new` on a
/// file and `create_dir` on a folder both refuse what is already there, in one
/// call the filesystem makes atomically — where a look followed by a create
/// would be a window for the agent to write the same name into, and a human
/// told the name was free while their file went over somebody's work.
///
/// What the *refusal* is worded as is not portable, though: Windows says access
/// denied rather than already-exists where the kind standing there is not the
/// kind asked for — a directory under `CREATE_NEW`, a file under
/// `CreateDirectoryW` — so a failure with something at the name is read as the
/// name being taken. That look is on the failure path alone and is about
/// something that is already there, so the atomic create is still the whole of
/// the check and there is no window it opens.
///
/// **What comes back is the folder joined to the name**, rather than the path as
/// it arrived: the viewer joins with a `/` wherever it runs, and a Worktree on
/// Windows is spelled with a `\`, so an echo would hand the tab a second name for
/// the file the next listing of that folder draws — and two names for one file is
/// two buffers. [`FolderEntry::path`] is the same join for the same reason.
pub(crate) fn make(roots: &[FileRoot], path: &Path, making: Making) -> FileMade {
    // A path that names nothing to make — a root of the filesystem, or one
    // ending in a separator — is under no root in the only sense that matters
    // here: there is no name at the end of it for a row to be drawn from.
    let (Some(name), Some(folder)) = (path.file_name(), path.parent()) else {
        return FileMade::Outside;
    };

    let (root, real) = match bound(roots, folder) {
        Bound::Inside { root, real } => (root, real),
        Bound::Outside => return FileMade::Outside,
        Bound::UnderGit => return FileMade::UnderGit,
        Bound::RootGone => return FileMade::RootGone,
        Bound::Missing => return FileMade::Missing,
    };

    // The folder is inside the root and outside its `.git`, and the name is one
    // segment — which leaves exactly one way for the new path to be under a git
    // directory the folder is not: to be called `.git` itself.
    if inside_git(Path::new(&root.path), path) {
        return FileMade::UnderGit;
    }

    // The root's own flag rather than the folder's mode, which is what the tree
    // drew when it left both rows off a read-only root — see
    // [`FileWritten::ReadOnly`], the same reading.
    if !root.writable {
        return FileMade::ReadOnly;
    }

    if !real.is_dir() {
        return FileMade::NotAFolder;
    }

    let at = real.join(name);

    let made = match making {
        // `create_new`, which is the O_EXCL the name check is: what is already
        // there is left exactly as it is, whatever kind of thing it is.
        Making::File => std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&at)
            .map(drop),

        // And `create_dir` rather than `create_dir_all`, which is the same
        // refusal: all the parents are there or the folder above is not a
        // folder, and a name with a separator in it is a request to make two
        // rows out of one field.
        Making::Folder => std::fs::create_dir(&at),
    };

    match made {
        // Spelled as the folder that was asked for with the name joined on,
        // rather than off `at` — which is the *resolved* parent — and rather than
        // echoing what arrived. [`FolderEntry::path`]'s rule and the same join,
        // so that the path the tab opens at is character for character the path
        // the next listing of that folder draws the row under: a viewer that
        // spelled the join with a `/` on a machine whose paths use `\` would
        // otherwise be handed two names for one file, and two names for one file
        // is two buffers.
        Ok(()) => FileMade::Made {
            path: folder.join(name).display().to_string(),
        },
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => FileMade::Taken,

        // And the same refusal where the filesystem worded it as something
        // else, which is Windows: `CREATE_NEW` over a *directory* and
        // `CreateDirectoryW` over a *file* both come back as access denied
        // rather than as already-exists, and [`FileMade::Taken`] promises the
        // one answer for all four crossings.
        //
        // A look rather than a second create, and only on the failure path —
        // where what it is asking about is something that is already there, so
        // there is no window for it to be wrong about and the atomic create
        // above is still the whole of the check. Of the entry rather than of
        // what it points at, a link standing at the name being something in the
        // way whatever is at the end of it.
        Err(_) if std::fs::symlink_metadata(&at).is_ok() => FileMade::Taken,

        Err(error) => FileMade::Unwritable {
            why: format!("the server cannot make it: {error}"),
        },
    }
}

/// Give whatever is at `path` the name `name`, in the folder it is already in.
///
/// Blocking: a path is resolved, the name beside it is looked at, and one entry
/// is moved.
///
/// **A name rather than a path, which is the whole of the bound on a rename.**
/// Two roots are two repositories, and a file taken out of one checkout and put
/// into another is not something a tree gets to ask for — so what arrives is one
/// segment, joined onto the folder the row is already in, and a `name` with a
/// separator in it is a path rather than a name (ADR 0019, *The tree*). Which is
/// also why there is no move here at all: a rename is this, and dragging a row
/// onto another folder is not a gesture the tree has.
///
/// The order the checks come in is [`make`]'s, with the one that is a rename's
/// alone in among them: the name is a name; the path is bounded, because every
/// other answer is about a path that has one; it is not a root, a root being a
/// Worktree rather than anything in one; the root takes writes at all; and the
/// disk is touched only after all four.
///
/// **And the look for a name already taken is a look**, which is where this
/// parts company with [`make`] and says so. A `create_new` is the filesystem's
/// own atomic refusal, and there is no portable rename that refuses to overwrite
/// — so what is here is a look followed by a move, and the window between them
/// is one nothing in std can close. Against a human's hand on a field it is
/// microseconds, and what it risks is an agent writing that exact name inside
/// them.
///
/// The one thing standing there that is not in the way is the path being renamed
/// itself, which is what a name differing only in its case is on a filesystem
/// that does not tell two cases apart: `README.md` to `readme.md` is a rename to
/// allow rather than a name already taken.
///
/// **And what is moved is the entry the tree drew**, rather than what it
/// resolves to — [`delete`]'s reading said about a move, for its reason: a row
/// that is a link is a name to move, and the file at the end of it is somebody
/// else's. Moved off `real` it would be a link left dangling exactly where the
/// row is, a file nobody asked about somewhere new, and an answer naming a path
/// with nothing at it for the tab to follow to. `real` is still what the bound
/// measures, and the two name the same entry for everything that is not a link.
pub(crate) fn rename(roots: &[FileRoot], path: &Path, name: &str) -> FileRenamed {
    // One plain segment and nothing else. A name with a separator in it, a `..`
    // or a `.` is a path somebody spelled into a field that asks for a name —
    // and a path is the one thing a rename is not asked by, so it is outside
    // whatever it would have named.
    let named = Path::new(name);

    if named.components().count() != 1 || !named.components().all(|part| plain(&part)) {
        return FileRenamed::Outside;
    }

    // And the folder it is already in, which is where the new name is joined on.
    // A path with no parent names no row — [`make`]'s first refusal, for its
    // reason.
    let Some(folder) = path.parent() else {
        return FileRenamed::Outside;
    };

    let (root, real) = match bound(roots, path) {
        Bound::Inside { root, real } => (root, real),
        Bound::Outside => return FileRenamed::Outside,
        Bound::UnderGit => return FileRenamed::UnderGit,
        Bound::RootGone => return FileRenamed::RootGone,
        Bound::Missing => return FileRenamed::Missing,
    };

    // A root is a Worktree rather than something in one, and what the human
    // knows it by is the Repo it is a checkout of. Asked as it was spelled and
    // again resolved, which is the bound's own two measurements: the tree never
    // spells a root any way but the roots listing's, and a symlink standing
    // where one is would otherwise be a way to rename the checkout.
    let at_the_root = Path::new(&root.path);

    if at_the_root == path || matches!(resolve(at_the_root), Resolved::At(it) if it == real) {
        return FileRenamed::IsRoot;
    }

    // The root's own flag rather than the file's mode — [`write`]'s reading and
    // [`make`]'s, for their reason.
    if !root.writable {
        return FileRenamed::ReadOnly;
    }

    // The path is inside the root and outside its `.git`, and the name is one
    // segment — which leaves exactly one way for the new path to be under a git
    // directory the old one was not: to be called `.git` itself.
    if name == GIT {
        return FileRenamed::UnderGit;
    }

    // The folder that was asked for with the new name joined on — which is
    // where the entry the tree drew goes, and is the path that comes back.
    // [`FileMade::Made`]'s rule and the same join, so that the path a tab
    // follows its file to is character for character the path the next listing
    // of that folder draws the row under; and one binding for both, so the move
    // and the answer cannot drift apart.
    let onto = folder.join(name);

    if standing(&onto, &real) {
        return FileRenamed::Taken;
    }

    // And what is moved is that entry rather than what it resolves to, which is
    // [`delete`]'s reading said about a move: a row that is a link is a name to
    // move, and the file at the end of it is somebody else's — moved off `real`
    // it would be a link left dangling where the tree drew it, a file nobody
    // asked about somewhere new, and an answer naming a path with nothing at
    // it. The two name the same entry for everything that is not a link, the
    // bound having measured both.
    match std::fs::rename(path, &onto) {
        Ok(()) => FileRenamed::Renamed {
            path: onto.display().to_string(),
        },
        Err(error) => FileRenamed::Unwritable {
            why: format!("the server cannot rename it: {error}"),
        },
    }
}

/// Take whatever is at `path` off the disk, a folder with everything under it.
///
/// Blocking: a path is resolved, one entry is looked at, and a tree is walked
/// where that entry is a directory.
///
/// **The whole of the check is [`rename`]'s, less the name.** Nothing is named
/// here, so there is nothing to be taken and nothing that could be spelled as a
/// path out of the root: what is left is the bound, a root itself, and the
/// root's own flag, in that order, with the disk touched only after all three.
///
/// **A root is refused**, which is the refusal that is this and the rename's
/// alone and is the harder of the two here: a Worktree deleted is the ground
/// taken out from under whatever session is standing in it, and a Worktree is
/// unmade by the Conversation that made it rather than by a row of a file tree.
/// Asked as it was spelled and again resolved, the bound's own two measurements.
///
/// **And what is removed is the entry the tree drew**, rather than what it
/// resolves to: the removal names `path` rather than the resolved `real`, so a
/// symlink is unlinked and the file at the end of it is left alone — which is
/// what the row under the hand was. The two name the same entry by then, the
/// bound having measured both.
///
/// **One call for a folder**, which is what makes one confirm enough: a
/// directory goes with everything under it in a single `remove_dir_all`, so
/// there is no half-deleted tree for a refusal to leave behind and nothing for
/// the human to be asked twice about.
pub(crate) fn delete(roots: &[FileRoot], path: &Path) -> FileDeleted {
    let (root, real) = match bound(roots, path) {
        Bound::Inside { root, real } => (root, real),
        Bound::Outside => return FileDeleted::Outside,
        Bound::UnderGit => return FileDeleted::UnderGit,
        Bound::RootGone => return FileDeleted::RootGone,
        Bound::Missing => return FileDeleted::Missing,
    };

    // A root is a Worktree rather than something in one — [`rename`]'s refusal,
    // read the same way and for a harder version of its reason.
    let at_the_root = Path::new(&root.path);

    if at_the_root == path || matches!(resolve(at_the_root), Resolved::At(it) if it == real) {
        return FileDeleted::IsRoot;
    }

    // The root's own flag rather than the entry's mode — [`write`]'s reading,
    // [`make`]'s and [`rename`]'s, for their reason.
    if !root.writable {
        return FileDeleted::ReadOnly;
    }

    // Of the entry rather than of what it points at, which is what says whether
    // a tree is being walked or one name is being unlinked: a link to a
    // directory is a name to take away, not a directory to empty.
    let entry = match std::fs::symlink_metadata(path) {
        Ok(entry) => entry,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return FileDeleted::Missing,
        Err(error) => {
            return FileDeleted::Unwritable {
                why: format!("the server cannot read it: {error}"),
            };
        }
    };

    let taken = if entry.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };

    match taken {
        Ok(()) => FileDeleted::Deleted,
        // Gone between the look and the removal, which is the agent's hand in
        // the same checkout: the row is not there, which is what the press was
        // asking for.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FileDeleted::Missing,
        Err(error) => FileDeleted::Unwritable {
            why: format!("the server cannot delete it: {error}"),
        },
    }
}

/// Every root's files, which is what the quick-open palette matches over.
///
/// Blocking: one run of `git ls-files` per root.
///
/// **Git's own list rather than a walk**: what it tracks, plus what it does not
/// track and does not ignore, which is the same account of a repository the
/// folder listing takes its ignores from. A walk would have to ask about every
/// directory it went into and then ask git about the lot anyway — and it would
/// walk a `target/` to find out it was not wanted.
///
/// **A root git will not answer about has nothing here.** No git on the machine,
/// a Worktree that has gone, a directory that is not a repository: the listing
/// beside this one falls the other way and shows a folder whole, because there
/// the answer is what to *leave out* and a tree with nothing in it would be
/// worse than a tree with a `target/` in it. Here git's answer is the list
/// itself, so there is nothing to fall back to.
///
/// **And every root is answered about**, whatever it says — a root with no files
/// is a row of the palette that says so, where a root missing from the answer
/// would be a checkout the human cannot tell from one holding nothing.
pub(crate) fn list(roots: &[FileRoot]) -> FileListsView {
    FileListsView {
        roots: roots.iter().map(listed).collect(),
    }
}

/// One root's, capped at [`MAX_LISTED`].
///
/// **Spelled in full the way the root is**, which is the join [`make`] makes and
/// makes for this reason: the page opens what it is given, and a path it had
/// joined with a `/` under a Worktree spelled with a `\` would be a second name
/// for a file the tree already has a name for — two names, two buffers, two
/// tabs over one file. Which is why git's own answer is joined on a segment at a
/// time rather than whole — see [`joined`], where the separator is.
///
/// Deduplicated on the way in, because one path can come back twice: a file in
/// conflict is in the index once per stage, and `ls-files` says it once per
/// stage with it. `--deduplicate` would do it in git, and is newer than the git
/// on plenty of machines this runs on.
fn listed(root: &FileRoot) -> FileList {
    let at = Path::new(&root.path);

    // `--others` is what is untracked and `--exclude-standard` is what makes
    // that *not ignored*: without it the answer is the whole build directory.
    // NUL-separated, so that a path with a newline or a quote in it arrives as
    // itself rather than as git's own quoting of it.
    let answer = git(
        at,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )
    .unwrap_or_default();

    let mut seen = HashSet::new();
    let mut files = Vec::new();
    let mut cut = false;

    for under in answer.split('\0').filter(|path| !path.is_empty()) {
        if !seen.insert(under) {
            continue;
        }

        if files.len() == MAX_LISTED {
            cut = true;
            break;
        }

        files.push(joined(at, under).display().to_string());
    }

    FileList {
        repo: root.repo.clone(),
        path: root.path.clone(),
        files,
        cut,
    }
}

/// How many of one root's files the palette is given.
///
/// A monorepo answers with a few hundred thousand paths and none of them would
/// be read on a page: what a palette is for is the file whose path nobody
/// remembers, and nobody scrolls to it. Ten thousand is over the whole of every
/// ordinary checkout — this one is under a thousand — so the cap is a bound on
/// the pathological case rather than something a human meets, and the root that
/// meets it says so rather than quietly matching over half a checkout.
pub(crate) const MAX_LISTED: usize = 10_000;

/// Every root's marks, which is what the tree draws on its rows.
///
/// Blocking: one run of `git status` per root.
///
/// **One reading per root and never a call per row**, answered for the whole
/// Conversation at once the way the palette's lists are: what a mark is about
/// is the difference between a checkout and its commit, and git says the whole
/// of that in one run.
///
/// **Its own reading rather than a field on a folder listing**, and a commit is
/// the argument for it: a commit made in a terminal beside the tree clears every
/// mark in the Worktree without touching a file, so nothing about any folder has
/// moved and every mark has changed. Which is why the page reads this back on a
/// `commit` Nudge as well as on a `files` one.
pub(crate) fn status(roots: &[FileRoot]) -> FileStatusView {
    FileStatusView {
        roots: roots.iter().map(marked).collect(),
    }
}

/// One root's, folded up so that a folder wears the strongest mark of anything
/// under it.
///
/// **Porcelain and NUL-separated**, so that a path with a newline or a quote in
/// its name arrives as itself rather than as git's own quoting of it — the
/// palette's list is read the same way for the same reason.
///
/// **Every untracked file rather than the folder holding them.** Git's default
/// is to name a wholly untracked directory once and leave what is in it unsaid,
/// which would be a folder marked over rows that were not: `--untracked-files=all`
/// is the answer that matches row for row with what the tree draws. Ignored
/// paths are not in it either way, which is the same account of the repository
/// the listing hides `target/` by.
///
/// **And renames are two paths rather than one.** `--no-renames` turns a rename
/// into the removal and the addition it is on the disk, which is both what a
/// tree wants — two rows moved, and both marked — and what keeps the parsing to
/// one path per record: a rename is the one porcelain record that carries two.
///
/// **A root git will not answer about is marked nothing** — no git on the
/// machine, a Worktree that has gone, a directory that is no checkout — which
/// is [`listed`]'s rule for the same reason: git's answer *is* the marks, so
/// there is nothing to fall back to and a tree of unmarked rows is still a tree.
fn marked(root: &FileRoot) -> FileStatus {
    let at = Path::new(&root.path);

    let answer = git(
        at,
        &[
            "status",
            "--porcelain",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ],
    )
    .unwrap_or_default();

    let mut marks: BTreeMap<String, Marked> = BTreeMap::new();

    for record in answer.split('\0').filter(|record| !record.is_empty()) {
        if marks.len() >= MAX_MARKED {
            break;
        }

        // `XY <path>`: the two status letters, the space git writes after them,
        // and the path as the filesystem holds it. Anything shorter is not a
        // record this understands, and what it does about one is nothing.
        let Some((state, under)) = record.split_at_checked(3) else {
            continue;
        };

        let mark = match state.starts_with("??") {
            true => Marked::Untracked,
            false => Marked::Changed,
        };

        // Spelled in full the way the root is, which is what the tree draws its
        // rows by: git answers with the path under the checkout, and a mark the
        // page could not match to a row would be a mark nothing drew. A segment
        // at a time, because git's own separator is not every platform's — see
        // [`joined`].
        let path = joined(at, under);

        // And folded up, the file first and then every folder over it as far as
        // the root, which is the row the fold is *for*: a change deep in a tree
        // is on the row above it before anybody expands one.
        for over in path.ancestors() {
            match marks.entry(over.display().to_string()) {
                Entry::Vacant(empty) => {
                    empty.insert(mark);
                }
                // The strongest wins, which is what makes a folder holding one
                // of each read as changed — see [`Marked`], where the order is.
                Entry::Occupied(mut held) => {
                    *held.get_mut() = (*held.get()).max(mark);
                }
            }

            if over == at {
                break;
            }
        }
    }

    FileStatus {
        repo: root.repo.clone(),
        path: root.path.clone(),
        marks: marks
            .into_iter()
            .map(|(path, mark)| FileMark { path, mark })
            .collect(),
    }
}

/// How many paths of one root the tree is given marks for.
///
/// The palette's cap read on a different reading, and a looser bound than it
/// looks: what is capped is what git *said*, and the folders folded over each of
/// them are inside the same count. A root where ten thousand paths have moved is
/// one where a tree of marks says nothing anybody can read — a `cargo build` in
/// a checkout with no `.gitignore` is the case — so what the cap is against is
/// the page being handed a payload the size of a build directory.
///
/// Nothing says it was reached, unlike the palette's. A list cut short is a
/// palette saying a file is not there, which is a wrong answer worth marking; a
/// mark cut short is a row drawn the way every unmarked row in the tree is
/// drawn, and there is nowhere on a row to say so.
const MAX_MARKED: usize = 10_000;

/// The directories of `root` a watcher of it watches: `from` and every
/// non-ignored directory under it, `from` first and the rest breadth-first.
///
/// Blocking: one `read_dir` per directory it goes into, and one run of
/// `check-ignore` per level it goes down.
///
/// **Ignore-aware, because the alternative exhausts the machine**
/// ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *Following the disk*).
/// The recommended watcher on Linux takes an inotify watch per directory, so a
/// recursive watch over a Rust `target/` would spend a machine's whole allowance
/// on files nobody can see. What is left out here is what the tree already hides
/// — git's own answer through [`ignored`], and `.git` by name — so a pane is
/// told about the rows it draws and about nothing else.
///
/// **A level at a time rather than a directory at a time**, which is what keeps
/// the git runs down to the depth of a checkout rather than the width of it: a
/// level's directories are read, every subdirectory of the lot is asked about in
/// one `check-ignore`, and what survives is the next level. An ignored directory
/// is never opened at all, which is why a checkout carrying a `target/` of a
/// hundred thousand files is walked in a moment.
///
/// **A root git will not answer about is walked whole**, the way the tree lists
/// one whole — see [`ignored`], where that falls out. What it costs is a build
/// directory watched in a directory that was no checkout.
///
/// **Links are not followed.** A directory reached through one is either in this
/// walk already under its own name or outside the root altogether, so following
/// one would be a way out of the Worktree and a way round in a circle.
///
/// **And `from` is where a walk after the first starts.** A directory made after
/// the watcher started is watched without anything being restarted, and what is
/// walked for it is the directory and whatever arrived inside it — a whole tree
/// moved in being one event and any number of directories.
///
/// At most `most` of them, which is what the watcher asking has left of the
/// machine's allowance — see [`crate::watchers::MAX_WATCHED`], where the bound
/// is and where the reason for one is.
pub(crate) fn watchable(root: &Path, from: &Path, most: usize) -> Vec<PathBuf> {
    if most == 0 || inside_git(root, from) || !from.is_dir() {
        return Vec::new();
    }

    // The root is watched whatever git says about its own top — which is
    // nothing, a repository not ignoring itself. Anything below one is asked.
    let asked = spelled_as_a_folder(from);

    if from != root && ignored(root, std::slice::from_ref(&asked)).contains(&asked) {
        return Vec::new();
    }

    let mut watchable = vec![from.to_owned()];
    let mut level = vec![from.to_owned()];

    while !level.is_empty() && watchable.len() < most {
        let mut below: Vec<PathBuf> = level.iter().flat_map(|at| folders_in(at)).collect();

        let asking: Vec<String> = below.iter().map(|at| spelled_as_a_folder(at)).collect();
        let ignored = ignored(root, &asking);

        below.retain(|at| !ignored.contains(&spelled_as_a_folder(at)));
        below.truncate(most - watchable.len());

        watchable.extend_from_slice(&below);
        level = below;
    }

    watchable
}

/// The directories `at` holds, spelled the way `at` is.
///
/// Of the entries themselves rather than of what they point at — see
/// [`watchable`], where the links are — and `.git` is not among them however it
/// got there. An entry that will not read is left out, and so is one whose name
/// is not UTF-8: the tree leaves both out of a listing, and git cannot be asked
/// about a path that will not spell.
fn folders_in(at: &Path) -> Vec<PathBuf> {
    let Ok(reading) = std::fs::read_dir(at) else {
        return Vec::new();
    };

    reading
        .filter_map(|read| {
            let read = read.ok()?;
            let name = read.file_name().to_str()?.to_owned();

            if name == GIT || !read.file_type().ok()?.is_dir() {
                return None;
            }

            Some(at.join(name))
        })
        .collect()
}

/// Whether something is at `onto` that is not `real` itself.
///
/// The look a rename does for a name already taken. Of the entry rather than of
/// what it points at, so that a link standing at the new name is something in
/// the way whatever is at the end of it — and then resolved, because the one
/// thing at the new name that is *not* in the way is the path being renamed,
/// which is what a rename that only changes a name's case is on a filesystem
/// that does not tell two cases apart.
fn standing(onto: &Path, real: &Path) -> bool {
    match std::fs::symlink_metadata(onto) {
        Err(_) => false,
        Ok(there) => there.is_symlink() || !matches!(resolve(onto), Resolved::At(it) if it == real),
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

/// And how large a body the write takes, which is not the same number.
///
/// What goes up is the text as a JSON string, with the path and the version
/// beside it — so a file exactly at the cap is a body some way over it: a
/// newline is two bytes written, a quote or a backslash two, and a control
/// character six. Six times the cap is what JSON can make of the widest of
/// those, and no text file reaches it — [`texted`] has already refused anything
/// with a NUL in it — so this refuses nothing a read allowed while still
/// bounding what one request can put in memory.
///
/// **Named at all because axum's own default is two mebibytes**, which is
/// *under* [`MAX_BYTES`]: without this the largest files Code opens would be
/// ones it lets somebody type into and then refuses to save — and refuses by
/// status, where every other refusal in this module is a sentence the tab
/// draws. The attachment routes raise theirs for the same reason.
pub(crate) const MAX_WRITE_BYTES: usize = 6 * MAX_BYTES as usize + 64 * 1024;

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

/// Which of the paths in `asking` git says are ignored in `root`, spelled the
/// way they were asked about — see [`asked_about`] and
/// [`spelled_as_a_folder`], which is how a caller spells them.
///
/// One run of `check-ignore` for the whole of the asking, with the paths fed in
/// on stdin: what comes back is the ignored ones, and a folder of ten thousand
/// rows is still one process. Which is why the paths go in on stdin rather than
/// as arguments — a folder wide enough to be worth reading this way is a folder
/// wide enough to overrun a command line. [`watchable`] asks a level of the
/// walk at a time for the same reason, and pays the same one process for it.
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
fn ignored(root: &Path, asking: &[String]) -> HashSet<String> {
    let asked: String = asking.iter().map(|ask| format!("{ask}\0")).collect();

    if asked.is_empty() {
        return HashSet::new();
    }

    // Exit 1 is the ordinary "nothing here is ignored" rather than a failure —
    // see [`crate::repos::feeding`], which is where the codes are read.
    feeding(root, &["check-ignore", "-z", "--stdin"], &asked, &[0, 1])
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
        true => spelled_as_a_folder(Path::new(&entry.path)),
        false => entry.path.clone(),
    }
}

/// And how a directory on its own is spelled to git, which is the spelling
/// [`watchable`] asks by — everything it asks about being one.
fn spelled_as_a_folder(path: &Path) -> String {
    format!("{}/", path.display())
}

/// One of git's own paths joined onto `root`, a segment at a time.
///
/// **Not `root.join(answer)`**, which is the whole point of this. Git answers
/// with a `/` between the segments on every platform, and `join` puts the
/// platform's own separator in front of what it is handed without touching what
/// is inside it — so a plain join under a Worktree spelled with a `\` reads
/// `…\crates/server\`, a spelling nothing else in this pane ever writes.
///
/// Which matters because these paths are matched against the rows the tree drew,
/// and matched as strings: a mark is drawn by looking a row's own path up, and a
/// palette's row is opened as a tab beside whatever the tree already has open.
/// A second spelling of one file is a mark nothing draws and a second buffer over
/// a file that already had one.
///
/// Empty segments are left out, so that nothing in git's answer can make this
/// push a separator with no name after it.
fn joined(root: &Path, answer: &str) -> PathBuf {
    answer
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(root.to_owned(), |at, segment| at.join(segment))
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
    use crate::watchers::MAX_WATCHED;

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
        std::fs::write(
            elsewhere.join("secrets"),
            "nothing of this conversation's\n",
        )
        .unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            write(&roots, &elsewhere.join("secrets"), "", "mine\n"),
            FileWritten::Outside
        );
        assert_eq!(
            write(&roots, &worktree.join("../elsewhere/secrets"), "", "mine\n"),
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

    /// Where a new file lands, and what it holds: nothing. What goes into it is
    /// a save through the tab it opens in, which is where content has always
    /// come from.
    #[test]
    fn a_new_file_is_made_empty_where_it_was_asked_for() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();

        // Segment by segment rather than in one string, so that it is spelled
        // the way this platform spells a path: what comes back is the folder
        // joined to the name, and a `/` written into the middle of it here
        // would be a name only one of the two ever used.
        let at = worktree.join("src").join("lib.rs");

        assert_eq!(
            make(&[root(&worktree)], &at, Making::File),
            FileMade::Made {
                path: at.display().to_string()
            }
        );
        assert_eq!(std::fs::read_to_string(&at).unwrap(), "");

        // And it is a row of the folder the moment it is read again, which is
        // what the press does with the answer — spelled the way the row under
        // it is, which is what lets the tab and the row be the one file.
        let listing = folder(&[root(&worktree)], &worktree.join("src"));
        assert_eq!(names(listing.clone()), ["lib.rs"]);
        assert_eq!(
            listed(listing).first().map(|row| row.path.clone()),
            Some(at.display().to_string())
        );
    }

    /// And the path it answers with is the folder joined to the name rather than
    /// the path as it arrived, which is what a viewer joining with a `/` on a
    /// machine whose paths use a `\` depends on.
    #[test]
    fn a_making_answers_with_the_folder_joined_to_the_name() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();

        // Asked for exactly the way the viewer asks: the folder spelled the way
        // the root it came from is, with a `/` before the name — which is the
        // one separator the page ever contributes, and on Windows is not this
        // platform's own.
        let asked = format!("{}/lib.rs", worktree.join("src").display());

        assert_eq!(
            make(&[root(&worktree)], Path::new(&asked), Making::File),
            FileMade::Made {
                path: worktree.join("src").join("lib.rs").display().to_string()
            }
        );
    }

    /// And a new folder is a folder, holding nothing and opening nothing.
    #[test]
    fn a_new_folder_is_made_where_it_was_asked_for() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        let at = worktree.join("crates");

        assert_eq!(
            make(&[root(&worktree)], &at, Making::Folder),
            FileMade::Made {
                path: at.display().to_string()
            }
        );
        assert!(at.is_dir());
        assert!(listed(folder(&[root(&worktree)], &at)).is_empty());
    }

    /// A name already taken is refused and what is there is left exactly as it
    /// was — whichever kind of thing it is, and whichever kind was asked for.
    #[test]
    fn a_name_already_taken_is_refused_and_nothing_is_overwritten() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            make(&roots, &worktree.join("README.md"), Making::File),
            FileMade::Taken
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("README.md")).unwrap(),
            "# a repository\n",
            "the file that was there is the file that is there"
        );

        assert_eq!(
            make(&roots, &worktree.join("src"), Making::Folder),
            FileMade::Taken
        );

        // And across the two kinds both ways: a file where a folder stands, and
        // a folder where a file does, are one refusal and one thing to do about
        // it. These two are where Windows words the refusal as access denied
        // rather than as already-exists, and where the look on the failure path
        // reads it back as what it is.
        assert_eq!(
            make(&roots, &worktree.join("src"), Making::File),
            FileMade::Taken
        );
        assert!(
            worktree.join("src").is_dir(),
            "the folder is still a folder"
        );

        assert_eq!(
            make(&roots, &worktree.join("README.md"), Making::Folder),
            FileMade::Taken
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("README.md")).unwrap(),
            "# a repository\n",
            "and the file is still the file"
        );
    }

    /// And a making that failed with *nothing* at the name is still the
    /// sentence about a disk rather than a name already taken — which is the
    /// bound on the look above, the one that reads a refusal Windows worded
    /// another way.
    #[test]
    #[cfg(unix)]
    fn a_making_that_failed_over_nothing_says_why_rather_than_taken() {
        use std::os::unix::fs::PermissionsExt;

        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let shut = worktree.join("shut");

        std::fs::create_dir(&shut).unwrap();
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o500)).unwrap();

        let made = make(&[root(&worktree)], &shut.join("mine.rs"), Making::File);

        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o700)).unwrap();

        // Root writes whatever the mode says and CI runs as somebody, so the
        // making is allowed to have landed. What it must never be is `Taken`:
        // there was no name taken, and telling the human to pick another one
        // would send them round a loop that has nothing at the end of it.
        assert!(
            matches!(made, FileMade::Unwritable { .. } | FileMade::Made { .. }),
            "a folder that takes no writes answers why, got {made:?}"
        );
    }

    /// A read-only root takes neither, before the disk is touched at all — the
    /// root's own flag rather than the folder's mode, which is what the tree
    /// drew when it left both rows off.
    #[test]
    fn a_read_only_root_takes_neither() {
        let held = tempfile::tempdir().unwrap();
        let companion = repository(&held.path().join("companion"));

        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        assert_eq!(
            make(&roots, &companion.join("mine.rs"), Making::File),
            FileMade::ReadOnly
        );
        assert_eq!(
            make(&roots, &companion.join("mine"), Making::Folder),
            FileMade::ReadOnly
        );

        assert!(!companion.join("mine.rs").exists());
        assert!(!companion.join("mine").exists());
    }

    /// And a making is bounded the way a read and a write are: the roots are the
    /// whole of what this API may reach, and the folder the new row goes in is
    /// what is measured against them.
    #[test]
    fn a_making_is_bounded_the_way_a_write_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = held.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            make(&roots, &elsewhere.join("mine.rs"), Making::File),
            FileMade::Outside
        );
        assert_eq!(
            make(&roots, &worktree.join("../elsewhere/mine.rs"), Making::File),
            FileMade::Outside
        );

        // A repository's insides, which Code does not touch — asked of the
        // folder above the new row and of the row's own name both, a `.git` at
        // the end of a path being the one way past the first check.
        assert_eq!(
            make(&roots, &worktree.join(".git/config"), Making::File),
            FileMade::UnderGit
        );
        assert_eq!(
            make(&roots, &worktree.join(".git"), Making::Folder),
            FileMade::UnderGit
        );

        // The folder it would go in is not there, which is not the same thing as
        // the Worktree having gone.
        assert_eq!(
            make(&roots, &worktree.join("nowhere/mine.rs"), Making::File),
            FileMade::Missing
        );

        // And something at that folder that is not a folder.
        assert_eq!(
            make(&roots, &worktree.join("README.md/mine.rs"), Making::File),
            FileMade::NotAFolder
        );

        // And a path with no name at the end of it, which names nothing to make.
        assert_eq!(
            make(&roots, Path::new("/"), Making::File),
            FileMade::Outside
        );

        assert!(!elsewhere.join("mine.rs").exists());

        std::fs::remove_dir_all(&worktree).unwrap();
        assert_eq!(
            make(&roots, &worktree.join("mine.rs"), Making::File),
            FileMade::RootGone
        );
    }

    /// A rename moves the thing where it stands, keeping what is in it — and
    /// answers with the folder joined to the new name, which is the path every
    /// open tab of it follows to.
    #[test]
    fn a_rename_moves_it_within_the_folder_it_is_in() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("src/lib.rs"), "fn main() {}\n").unwrap();

        let roots = [root(&worktree)];
        let at = worktree.join("src/lib.rs");

        assert_eq!(
            rename(&roots, &at, "main.rs"),
            FileRenamed::Renamed {
                path: worktree.join("src").join("main.rs").display().to_string()
            }
        );

        assert!(!at.exists());
        assert_eq!(
            std::fs::read_to_string(worktree.join("src/main.rs")).unwrap(),
            "fn main() {}\n",
            "what was in it is what is in it"
        );

        // And it is the row the folder draws the moment it is read again, which
        // is what the press does with the answer.
        assert_eq!(names(folder(&roots, &worktree.join("src"))), ["main.rs"]);
    }

    /// A folder renames with everything under it, which is what a tab inside one
    /// follows: the whole subtree moves in the one call the filesystem makes.
    #[test]
    fn a_folder_renames_with_what_is_under_it() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir_all(worktree.join("src/inner")).unwrap();
        std::fs::write(worktree.join("src/inner/deep.rs"), "deep\n").unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            rename(&roots, &worktree.join("src"), "crates"),
            FileRenamed::Renamed {
                path: worktree.join("crates").display().to_string()
            }
        );

        assert_eq!(
            std::fs::read_to_string(worktree.join("crates/inner/deep.rs")).unwrap(),
            "deep\n"
        );
        assert!(!worktree.join("src").exists());
    }

    /// And the path it answers with is the folder as it was spelled joined to the
    /// name, rather than the resolved parent — [`make`]'s rule, and what lets the
    /// tab and the row of the next listing be the one file.
    #[test]
    fn a_rename_answers_with_the_folder_joined_to_the_new_name() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("src/lib.rs"), "").unwrap();

        // Asked for with a separator that is not this platform's own, which is
        // what a path arriving from a page may carry: the answer is the folder
        // as it was spelled with the new name joined onto it the way this
        // platform joins, so the tab and the next listing's row are the one
        // string.
        let asked = format!("{}/lib.rs", worktree.join("src").display());

        assert_eq!(
            rename(&[root(&worktree)], Path::new(&asked), "main.rs"),
            FileRenamed::Renamed {
                path: worktree.join("src").join("main.rs").display().to_string()
            }
        );
    }

    /// A name already in the folder is refused with nothing moved, whichever kind
    /// of thing is standing there.
    #[test]
    fn a_name_already_taken_refuses_a_rename_and_nothing_moves() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("notes.md"), "notes\n").unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            rename(&roots, &worktree.join("notes.md"), "README.md"),
            FileRenamed::Taken
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("README.md")).unwrap(),
            "# a repository\n",
            "the file that was there is the file that is there"
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("notes.md")).unwrap(),
            "notes\n",
            "and the one being renamed did not move"
        );

        // And across the two kinds: a folder where a file stands, and a file
        // where a folder does, are one refusal and one thing to do about it.
        assert_eq!(
            rename(&roots, &worktree.join("src"), "README.md"),
            FileRenamed::Taken
        );
        assert_eq!(
            rename(&roots, &worktree.join("notes.md"), "src"),
            FileRenamed::Taken
        );
    }

    /// And a link standing at the new name is something in the way whatever it
    /// points at — including the very file being renamed, which is the one thing
    /// at that name that a rename would otherwise be allowed to destroy.
    #[cfg(unix)]
    #[test]
    fn a_link_at_the_new_name_is_in_the_way() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::write(worktree.join("notes.md"), "notes\n").unwrap();
        std::os::unix::fs::symlink("notes.md", worktree.join("alias.md")).unwrap();

        assert_eq!(
            rename(&[root(&worktree)], &worktree.join("notes.md"), "alias.md"),
            FileRenamed::Taken
        );
        assert!(worktree.join("notes.md").exists());
    }

    /// A root is a Worktree rather than something in one, so it has no name here
    /// to change — the one refusal that is a rename's alone, answered whether the
    /// root takes writes or not.
    #[test]
    fn a_root_itself_cannot_be_renamed() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        assert_eq!(
            rename(&[root(&worktree)], &worktree, "elsewhere"),
            FileRenamed::IsRoot
        );
        assert!(worktree.is_dir());

        // And a read-only root is the same answer: what it is comes before what
        // may be done in it, a Worktree being no more renameable for being
        // writable.
        let companion = repository(&held.path().join("companion"));
        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        assert_eq!(rename(&roots, &companion, "elsewhere"), FileRenamed::IsRoot);
    }

    /// A read-only root renames nothing in it, before the disk is touched at all
    /// — the root's own flag, which is what the tree read when it left the row
    /// off the menu.
    #[test]
    fn a_read_only_root_renames_nothing() {
        let held = tempfile::tempdir().unwrap();
        let companion = repository(&held.path().join("companion"));

        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        assert_eq!(
            rename(&roots, &companion.join("README.md"), "NOTES.md"),
            FileRenamed::ReadOnly
        );
        assert!(companion.join("README.md").exists());
        assert!(!companion.join("NOTES.md").exists());
    }

    /// And a rename is bounded the way a write is, with the name itself bounded
    /// beside the path: a name with a separator in it is a path somebody spelled
    /// into a field that asks for a name, and a path is the one thing that could
    /// carry a row out of the root it is in.
    #[test]
    fn a_rename_is_bounded_the_way_a_write_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = repository(&held.path().join("elsewhere"));
        std::fs::create_dir(worktree.join("src")).unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            rename(&roots, &elsewhere.join("README.md"), "NOTES.md"),
            FileRenamed::Outside
        );
        assert_eq!(
            rename(&roots, &worktree.join("../elsewhere/README.md"), "NOTES.md"),
            FileRenamed::Outside
        );

        // A name that is a path, which is the whole of what keeps a rename inside
        // its root: two roots are two repositories, and neither a climb, a
        // separator nor an absolute path is a name.
        for name in ["../escaped.md", "src/nested.md", "..", ".", ""] {
            assert_eq!(
                rename(&roots, &worktree.join("README.md"), name),
                FileRenamed::Outside,
                "{name} is a path rather than a name"
            );
        }
        assert_eq!(
            rename(
                &roots,
                &worktree.join("README.md"),
                &elsewhere.display().to_string()
            ),
            FileRenamed::Outside
        );

        // A repository's insides, which Code does not touch — asked of the path
        // and of the new name both, a name spelled `.git` being the one way past
        // the first.
        assert_eq!(
            rename(&roots, &worktree.join(".git/config"), "conf"),
            FileRenamed::UnderGit
        );
        assert_eq!(
            rename(&roots, &worktree.join("README.md"), ".git"),
            FileRenamed::UnderGit
        );

        // The path is not there, which is not the same thing as the Worktree
        // having gone.
        assert_eq!(
            rename(&roots, &worktree.join("nowhere.md"), "somewhere.md"),
            FileRenamed::Missing
        );

        assert_eq!(
            std::fs::read_to_string(worktree.join("README.md")).unwrap(),
            "# a repository\n"
        );
        assert!(!elsewhere.join("NOTES.md").exists());

        std::fs::remove_dir_all(&worktree).unwrap();
        assert_eq!(
            rename(&roots, &worktree.join("README.md"), "NOTES.md"),
            FileRenamed::RootGone
        );
    }
    /// A file deleted goes off the disk and out of the folder's next listing,
    /// which is the whole of what the menu's fourth row does.
    #[test]
    fn a_file_deleted_goes_off_the_disk() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("src/lib.rs"), "fn main() {}\n").unwrap();
        std::fs::write(worktree.join("src/main.rs"), "fn main() {}\n").unwrap();

        let roots = [root(&worktree)];

        assert_eq!(
            delete(&roots, &worktree.join("src/lib.rs")),
            FileDeleted::Deleted
        );
        assert!(!worktree.join("src/lib.rs").exists());

        // And it is off the listing the moment the folder is read again, which
        // is what the press does with the answer.
        assert_eq!(names(folder(&roots, &worktree.join("src"))), ["main.rs"]);
    }

    /// A folder deleted takes everything under it, in the one call — which is
    /// what makes one confirm in front of the press enough.
    #[test]
    fn a_folder_deleted_takes_what_is_under_it() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::create_dir_all(worktree.join("src/inner/deeper")).unwrap();
        std::fs::write(worktree.join("src/inner/deep.rs"), "deep\n").unwrap();
        std::fs::write(worktree.join("src/inner/deeper/deepest.rs"), "deepest\n").unwrap();

        assert_eq!(
            delete(&[root(&worktree)], &worktree.join("src")),
            FileDeleted::Deleted
        );

        assert!(!worktree.join("src").exists());
        assert!(worktree.join("README.md").exists(), "and nothing beside it");
    }

    /// A link is unlinked, and what it points at is left where it is: what the
    /// row under the hand was is the entry, rather than the file at the end of
    /// it.
    #[cfg(unix)]
    #[test]
    fn a_link_is_unlinked_and_what_it_points_at_stays() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        std::fs::write(worktree.join("notes.md"), "notes\n").unwrap();
        std::os::unix::fs::symlink("notes.md", worktree.join("alias.md")).unwrap();

        assert_eq!(
            delete(&[root(&worktree)], &worktree.join("alias.md")),
            FileDeleted::Deleted
        );

        assert!(!worktree.join("alias.md").exists());
        assert_eq!(
            std::fs::read_to_string(worktree.join("notes.md")).unwrap(),
            "notes\n",
            "the file at the end of it is somebody else's"
        );
    }

    /// A root is a Worktree rather than something in one, so nothing here takes
    /// one away — the refusal this shares with a rename, answered whether the
    /// root takes writes or not.
    #[test]
    fn a_root_itself_cannot_be_deleted() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        assert_eq!(delete(&[root(&worktree)], &worktree), FileDeleted::IsRoot);
        assert!(worktree.is_dir());

        let companion = repository(&held.path().join("companion"));
        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        assert_eq!(delete(&roots, &companion), FileDeleted::IsRoot);
        assert!(companion.is_dir());
    }

    /// A read-only root loses nothing in it, before the disk is touched at all —
    /// the root's own flag, which is what the tree read when it left the row off
    /// the menu.
    #[test]
    fn a_read_only_root_deletes_nothing() {
        let held = tempfile::tempdir().unwrap();
        let companion = repository(&held.path().join("companion"));

        let roots = [FileRoot {
            repo: "askance".to_owned(),
            path: companion.display().to_string(),
            own: false,
            writable: false,
        }];

        assert_eq!(
            delete(&roots, &companion.join("README.md")),
            FileDeleted::ReadOnly
        );
        assert!(companion.join("README.md").exists());
    }

    /// And a deletion is bounded the way a write is: a Conversation's own
    /// checkouts and no more, whatever path a request names.
    #[test]
    fn a_deletion_is_bounded_the_way_a_write_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let elsewhere = repository(&held.path().join("elsewhere"));

        let roots = [root(&worktree)];

        assert_eq!(
            delete(&roots, &elsewhere.join("README.md")),
            FileDeleted::Outside
        );
        assert_eq!(
            delete(&roots, &worktree.join("../elsewhere/README.md")),
            FileDeleted::Outside
        );

        // A repository's insides, which Code does not touch.
        assert_eq!(
            delete(&roots, &worktree.join(".git/config")),
            FileDeleted::UnderGit
        );

        // The path is not there, which is not the same thing as the Worktree
        // having gone.
        assert_eq!(
            delete(&roots, &worktree.join("nowhere.md")),
            FileDeleted::Missing
        );

        assert_eq!(
            std::fs::read_to_string(elsewhere.join("README.md")).unwrap(),
            "# a repository\n"
        );
        assert!(worktree.join(".git/config").exists());

        std::fs::remove_dir_all(&worktree).unwrap();
        assert_eq!(
            delete(&roots, &worktree.join("README.md")),
            FileDeleted::RootGone
        );
    }

    /// The files of one root, as the paths they would be opened by — which is
    /// the whole point of this reading.
    fn files(list: &FileListsView, at: usize) -> Vec<String> {
        list.roots[at].files.clone()
    }

    /// What git tracks and what it does not track and does not ignore, both —
    /// and nothing it ignores, nothing inside `.git`, and no folders.
    #[test]
    fn a_roots_list_is_what_git_tracks_and_what_it_does_not_ignore() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("crates/server")).unwrap();
        std::fs::create_dir_all(worktree.join("target/debug")).unwrap();
        std::fs::write(worktree.join("crates/server/lib.rs"), "").unwrap();
        std::fs::write(worktree.join("target/debug/verkstead"), "").unwrap();
        std::fs::write(worktree.join("build.log"), "").unwrap();

        let listed = list(&[root(&worktree)]);

        assert_eq!(listed.roots.len(), 1);
        assert_eq!(listed.roots[0].repo, "verkstead");
        assert_eq!(listed.roots[0].path, worktree.display().to_string());
        assert!(!listed.roots[0].cut);

        // `.gitignore` and `README.md` are tracked; `crates/server/lib.rs` is
        // untracked and not ignored; `target/` and `*.log` are ignored, and the
        // git directory is in nobody's answer.
        let mut paths = files(&listed, 0);
        paths.sort();

        assert_eq!(
            paths,
            [
                worktree.join(".gitignore").display().to_string(),
                worktree.join("README.md").display().to_string(),
                worktree.join("crates/server/lib.rs").display().to_string(),
            ]
        );
    }

    /// Every root is answered about, in the roots' own order, each with the
    /// Repo it is a checkout of — two roots being able to hold the same path.
    #[test]
    fn every_root_is_a_list_of_its_own_in_the_order_they_come() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let alongside = repository(&held.path().join("askance"));

        std::fs::write(worktree.join("notes.md"), "mine\n").unwrap();
        std::fs::write(alongside.join("notes.md"), "theirs\n").unwrap();

        let mut companion = root(&alongside);
        companion.repo = "askance".to_owned();
        companion.own = false;
        companion.writable = false;

        let listed = list(&[root(&worktree), companion]);

        assert_eq!(
            listed
                .roots
                .iter()
                .map(|one| one.repo.as_str())
                .collect::<Vec<_>>(),
            ["verkstead", "askance"]
        );

        // The same path under two roots, spelled in full each time: which of
        // them a row is in is the root it is under rather than anything the
        // name says.
        assert!(files(&listed, 0).contains(&worktree.join("notes.md").display().to_string()));
        assert!(files(&listed, 1).contains(&alongside.join("notes.md").display().to_string()));

        // And a read-only root is listed like any other: it is a root to read,
        // and quick open opens files rather than writing them.
        assert!(!listed.roots[1].cut);
    }

    /// A root git will not answer about lists nothing — which is the folder
    /// listing's rule read the other way up: there git's answer says what to
    /// leave out, and here it *is* the list.
    #[test]
    fn a_root_git_will_not_answer_about_lists_nothing() {
        let held = tempfile::tempdir().unwrap();
        let plain = held.path().join("not-a-repository");
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(plain.join("README.md"), "# not a checkout\n").unwrap();

        let listed = list(&[root(&plain)]);

        assert!(files(&listed, 0).is_empty(), "{:?}", listed.roots[0].files);
        assert!(!listed.roots[0].cut);

        // And so does a Worktree that is no longer on disk, which is the same
        // answer for the same reason: nobody is there to ask.
        let gone = held.path().join("gone");
        let listed = list(&[root(&gone)]);

        assert!(files(&listed, 0).is_empty(), "{:?}", listed.roots[0].files);
    }

    /// A root holding more than the cap is cut, and says so — the palette
    /// saying it rather than quietly matching over half a checkout.
    #[test]
    fn a_root_over_the_cap_is_cut_and_says_so() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let many = worktree.join("many");
        std::fs::create_dir(&many).unwrap();

        // One over the cap, counting the two the repository was made with: the
        // list stops at the cap and the flag is what says there was more.
        for at in 0..=MAX_LISTED {
            std::fs::write(many.join(format!("{at}.rs")), "").unwrap();
        }

        let listed = list(&[root(&worktree)]);

        assert_eq!(listed.roots[0].files.len(), MAX_LISTED);
        assert!(listed.roots[0].cut);

        // And a root under it is not cut, which is every ordinary checkout.
        std::fs::remove_dir_all(&many).unwrap();
        let listed = list(&[root(&worktree)]);

        assert!(!listed.roots[0].cut);
    }

    /// The marks of one root, as the paths they are drawn on paired with what
    /// each of them says.
    fn marks(view: &FileStatusView, at: usize) -> Vec<(String, Marked)> {
        view.roots[at]
            .marks
            .iter()
            .map(|mark| (mark.path.clone(), mark.mark))
            .collect()
    }

    /// And the same said about one root, as the paths *under* it: what the fold
    /// is about is which rows are marked, and a temporary directory in front of
    /// every one of them says nothing about that.
    ///
    /// **Nothing normalised on the way out.** What a mark has to be is spelled
    /// the way the row it is drawn on is, and a reading that put every separator
    /// the same way round would pass whichever way they really were — which is
    /// the one thing worth proving here on Windows. So the expectations are
    /// spelled by [`spelled`] instead.
    fn marked_under(root: &Path, view: &FileStatusView) -> Vec<(String, Marked)> {
        marks(view, 0)
            .into_iter()
            .map(|(path, mark)| {
                let under = Path::new(&path)
                    .strip_prefix(root)
                    .map(|under| under.to_string_lossy().into_owned())
                    // The root's own row, which is the last thing the fold marks
                    // and is drawn as the repository rather than as a path.
                    .unwrap_or_default();

                (under, mark)
            })
            .collect()
    }

    /// A path under a root as this platform spells one, written here a segment
    /// at a time so that the spelling is the platform's rather than git's.
    ///
    /// What the tree joins its rows by, and so what a mark has to match: on
    /// Windows that is `crates\server`, and a mark spelled `crates/server` is a
    /// mark no row ever looks up.
    fn spelled(under: &str) -> String {
        under.split('/').collect::<PathBuf>().display().to_string()
    }

    /// A file that has moved is marked changed, one git has never seen is marked
    /// untracked, and every folder over either carries the mark as far as the
    /// root — the strongest one where a folder holds both.
    #[test]
    fn a_change_is_marked_and_so_is_every_folder_over_it() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        // A tracked file edited, and a new one written deep in a tree of folders
        // that were not there before.
        std::fs::write(worktree.join("README.md"), "# edited\n").unwrap();
        std::fs::create_dir_all(worktree.join("crates/server")).unwrap();
        std::fs::write(worktree.join("crates/server/lib.rs"), "").unwrap();

        let view = status(&[root(&worktree)]);

        assert_eq!(view.roots.len(), 1);
        assert_eq!(view.roots[0].repo, "verkstead");
        assert_eq!(view.roots[0].path, worktree.display().to_string());

        assert_eq!(
            marked_under(&worktree, &view),
            [
                // The root wears both, so it wears the stronger.
                (String::new(), Marked::Changed),
                (spelled("README.md"), Marked::Changed),
                (spelled("crates"), Marked::Untracked),
                (spelled("crates/server"), Marked::Untracked),
                (spelled("crates/server/lib.rs"), Marked::Untracked),
            ]
        );
    }

    /// And every mark is spelled the way the row it is drawn on is, which is
    /// what lets the tree look one up by its own path.
    ///
    /// **The one thing here that a Linux reading cannot say.** Git answers with
    /// a `/` between the segments whatever the platform, and a row of the tree is
    /// joined with the platform's own — so on Windows a mark joined whole would
    /// read `…\crates/server` over a row that reads `…\crates\server`, and
    /// nothing below the top of a root would ever find its mark. Read against
    /// the listings rather than against strings written here, the listings being
    /// what draws the rows.
    ///
    /// The palette's list is asked the same question, for the same reason said
    /// about a tab: a path spelled two ways is two buffers over one file.
    #[test]
    fn a_mark_is_spelled_the_way_the_row_it_is_drawn_on_is() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("crates/server")).unwrap();
        std::fs::write(worktree.join("crates/server/lib.rs"), "").unwrap();

        let roots = [root(&worktree)];

        // The walk down to it, as the tree draws it: one listing per level, and
        // the row's own path off each.
        let row = |within: &Path, name: &str| {
            listed(folder(&roots, within))
                .into_iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("the tree drew no {name} in {}", within.display()))
                .path
        };

        let crates = row(&worktree, "crates");
        let server = row(Path::new(&crates), "server");
        let lib = row(Path::new(&server), "lib.rs");

        let marked: Vec<String> = status(&roots).roots[0]
            .marks
            .iter()
            .map(|mark| mark.path.clone())
            .collect();

        for at in [&crates, &server, &lib] {
            assert!(marked.contains(at), "{at} is not among {marked:?}");
        }

        // And the palette opens the file by the string the tree would have
        // handed over for it.
        assert!(files(&list(&roots), 0).contains(&lib), "{lib}");
    }

    /// And a folder holding one of each wears the changed one, wherever in it
    /// the two are: the mark is there so that what changed is visible before a
    /// diff is, and an untracked file is already visible by being a row.
    #[test]
    fn a_folder_holding_both_wears_the_changed_one() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("src/main.rs"), "fn main() {}\n").unwrap();
        commit(&worktree);

        std::fs::write(worktree.join("src/main.rs"), "fn main() {/**/}\n").unwrap();
        std::fs::write(worktree.join("src/new.rs"), "").unwrap();

        assert_eq!(
            marked_under(&worktree, &status(&[root(&worktree)])),
            [
                (String::new(), Marked::Changed),
                (spelled("src"), Marked::Changed),
                (spelled("src/main.rs"), Marked::Changed),
                (spelled("src/new.rs"), Marked::Untracked),
            ]
        );
    }

    /// Every untracked file rather than the folder holding them, which is what
    /// matches the tree row for row: git's own default names a wholly untracked
    /// directory once and leaves what is in it unsaid.
    #[test]
    fn every_file_of_an_untracked_folder_is_marked() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("notes")).unwrap();
        std::fs::write(worktree.join("notes/one.md"), "").unwrap();
        std::fs::write(worktree.join("notes/two.md"), "").unwrap();

        assert_eq!(
            marked_under(&worktree, &status(&[root(&worktree)])),
            [
                (String::new(), Marked::Untracked),
                (spelled("notes"), Marked::Untracked),
                (spelled("notes/one.md"), Marked::Untracked),
                (spelled("notes/two.md"), Marked::Untracked),
            ]
        );
    }

    /// And what git ignores is marked nothing, which is the same account of the
    /// repository the tree hides a `target/` by: a row nobody draws is a row
    /// nothing marks.
    #[test]
    fn nothing_ignored_is_marked_and_a_clean_checkout_is_marked_nothing() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("target/debug")).unwrap();
        std::fs::write(worktree.join("target/debug/verkstead"), "").unwrap();
        std::fs::write(worktree.join("build.log"), "").unwrap();

        assert!(
            marks(&status(&[root(&worktree)]), 0).is_empty(),
            "{:?}",
            marks(&status(&[root(&worktree)]), 0)
        );
    }

    /// And a commit clears them all, with nothing in the tree having been
    /// written — which is the whole reason this is a reading of its own rather
    /// than a field on a folder listing.
    #[test]
    fn a_commit_clears_every_mark_without_a_file_moving() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::write(worktree.join("README.md"), "# edited\n").unwrap();
        std::fs::create_dir_all(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("src/main.rs"), "fn main() {}\n").unwrap();

        assert!(!marks(&status(&[root(&worktree)]), 0).is_empty());

        // The folder listing says exactly what it said before, and every mark
        // has gone.
        let before = folder(&[root(&worktree)], &worktree);
        commit(&worktree);

        assert_eq!(folder(&[root(&worktree)], &worktree), before);
        assert!(
            marks(&status(&[root(&worktree)]), 0).is_empty(),
            "{:?}",
            marks(&status(&[root(&worktree)]), 0)
        );
    }

    /// Every root is answered about, in the roots' own order — the palette's
    /// rule, and a read-only companion is marked like any other: it is a
    /// checkout to read, and something may well have moved in it.
    #[test]
    fn every_root_is_marked_in_the_order_they_come() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));
        let alongside = repository(&held.path().join("askance"));

        std::fs::write(worktree.join("README.md"), "# mine\n").unwrap();
        std::fs::write(alongside.join("README.md"), "# theirs\n").unwrap();

        let mut companion = root(&alongside);
        companion.repo = "askance".to_owned();
        companion.own = false;
        companion.writable = false;

        let view = status(&[root(&worktree), companion]);

        assert_eq!(
            view.roots
                .iter()
                .map(|one| one.repo.as_str())
                .collect::<Vec<_>>(),
            ["verkstead", "askance"]
        );

        // The same path under two roots, spelled in full each time — which of
        // them a mark is about is the root it is under.
        assert!(marks(&view, 0).contains(&(
            worktree.join("README.md").display().to_string(),
            Marked::Changed
        )));
        assert!(marks(&view, 1).contains(&(
            alongside.join("README.md").display().to_string(),
            Marked::Changed
        )));
    }

    /// And a root git will not answer about is marked nothing, which draws its
    /// rows unmarked rather than failing to draw them — [`list`]'s rule, said
    /// about the marks.
    #[test]
    fn a_root_git_will_not_answer_about_is_marked_nothing() {
        let held = tempfile::tempdir().unwrap();
        let plain = held.path().join("not-a-repository");
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(plain.join("README.md"), "# not a checkout\n").unwrap();

        let view = status(&[root(&plain)]);

        assert_eq!(view.roots.len(), 1);
        assert!(marks(&view, 0).is_empty(), "{:?}", view.roots[0].marks);

        // And so does a Worktree that is no longer on disk: nobody is there to
        // ask.
        let view = status(&[root(&held.path().join("gone"))]);

        assert!(marks(&view, 0).is_empty(), "{:?}", view.roots[0].marks);
    }

    /// Everything in the checkout, committed by the same hand the repository
    /// was made by — what a commit in a terminal beside the tree comes to.
    fn commit(at: &Path) {
        for args in [vec!["add", "-A"], vec!["commit", "-m", "the work"]] {
            let ran = std::process::Command::new("git")
                .args(&args)
                .current_dir(at)
                .output()
                .unwrap();
            assert!(ran.status.success(), "git {args:?} failed");
        }
    }

    /// What the watcher is given: every non-ignored directory of the root, the
    /// root itself first, and neither `.git` nor what git ignores among them.
    #[test]
    fn the_walk_is_every_directory_the_tree_would_draw() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        for at in ["src/deep/deeper", "docs", "target/debug/build", "src/logs"] {
            std::fs::create_dir_all(worktree.join(at)).unwrap();
        }

        assert_eq!(
            walked(&worktree, &worktree),
            [
                "",
                "docs",
                "src",
                "src/deep",
                "src/deep/deeper",
                // `*.log` is a rule about files, and a directory called `logs`
                // is not one of them.
                "src/logs",
            ],
        );
    }

    /// And a walk from a directory that has just appeared is that directory and
    /// what arrived inside it, which is how a `mkdir` in a terminal comes to be
    /// watched without anything being restarted.
    #[test]
    fn a_walk_from_a_directory_that_appeared_is_it_and_what_is_under_it() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("made/inside")).unwrap();

        assert_eq!(
            walked(&worktree, &worktree.join("made")),
            ["made", "made/inside"],
        );
    }

    /// An ignored directory is nothing to walk from either: a build making its
    /// own `target/` is a directory that appeared, and the answer about it is the
    /// answer the tree gives.
    #[test]
    fn an_ignored_directory_is_walked_from_nowhere() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        std::fs::create_dir_all(worktree.join("target/debug")).unwrap();

        assert_eq!(
            walked(&worktree, &worktree.join("target")),
            Vec::<String>::new()
        );

        // And neither is the git directory, whatever asked about it.
        assert_eq!(
            walked(&worktree, &worktree.join(".git")),
            Vec::<String>::new()
        );

        // Nor a directory that is not there at all, which is what a rename leaves
        // behind at the name it moved off.
        assert_eq!(
            walked(&worktree, &worktree.join("gone")),
            Vec::<String>::new()
        );
    }

    /// A root git will not answer about is walked whole, the way the tree lists
    /// one whole: what a refusal would cost is the watch.
    #[test]
    fn a_root_git_will_not_answer_about_is_walked_whole() {
        let held = tempfile::tempdir().unwrap();
        let worktree = held.path().join("worktree");
        std::fs::create_dir_all(worktree.join("target/debug")).unwrap();

        assert_eq!(walked(&worktree, &worktree), ["", "target", "target/debug"]);
    }

    /// And the walk is bounded, because the watch behind it is: what the watcher
    /// has left of its allowance is what it asks for, and a checkout wider than
    /// that is cut rather than refused.
    #[test]
    fn the_walk_stops_where_the_allowance_does() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        for at in ["one", "two", "three"] {
            std::fs::create_dir(worktree.join(at)).unwrap();
        }

        assert_eq!(watchable(&worktree, &worktree, 3).len(), 3);

        // And nothing at all where there is nothing left of it, which is the
        // walk a watcher already at its cap asks for.
        assert_eq!(walked(&worktree, &worktree), ["", "one", "three", "two"]);
        assert_eq!(watchable(&worktree, &worktree, 0), Vec::<PathBuf>::new());
    }

    /// A checkout carrying a large ignored directory is walked in a moment,
    /// because an ignored directory is never opened at all.
    ///
    /// The number here is a thousand directories nobody should look inside, and
    /// what the walk pays for them is one `check-ignore` answer about the one
    /// directory above them. A second is a hundred times what that takes and is
    /// the claim the stage makes, so the measurement is of the right thing and
    /// nowhere near the wire.
    #[test]
    fn a_large_ignored_directory_is_walked_past_in_a_moment() {
        let held = tempfile::tempdir().unwrap();
        let worktree = repository(&held.path().join("worktree"));

        for at in 0..1_000 {
            std::fs::create_dir_all(worktree.join(format!("target/debug/{at}"))).unwrap();
        }

        std::fs::create_dir(worktree.join("src")).unwrap();

        let started = std::time::Instant::now();
        let walked = walked(&worktree, &worktree);

        assert_eq!(walked, ["", "src"]);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "walking past an ignored directory took {:?}",
            started.elapsed()
        );
    }

    /// The walk of `from` in `root`, as the paths under the root rather than as
    /// the whole of them, sorted so that the reading is of what is watched rather
    /// than of the order a filesystem happened to hand back.
    fn walked(root: &Path, from: &Path) -> Vec<String> {
        let mut walked: Vec<String> = watchable(root, from, MAX_WATCHED)
            .iter()
            .map(|at| {
                at.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        // Where it started first, and the rest by name: what the walk promises
        // about its order is breadth-first, which is neither.
        if let Some(rest) = walked.get_mut(1..) {
            rest.sort();
        }

        walked
    }
}
