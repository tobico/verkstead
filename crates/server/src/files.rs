//! Reading the Worktrees a Conversation has, for the Code pane: the roots its
//! tree stands on, and one folder of one of them at a time.
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
//! **Nothing here refuses by status code**, the way registering a Repo refuses
//! and the way a browse's listing does: each refusal is a sentence the tree
//! draws where its rows would be — see [`verkstead_render::FolderListing`].
//!
//! **Not a record.** Nothing here writes to the store, puts anything on a
//! Timeline or reaches a Share. It is a reading of the disk, made afresh every
//! time the tree asks — which is what an expand is, until the watcher of stage
//! 04 tells the page the disk has moved.

use std::collections::HashSet;
use std::path::{Component, Path};

use verkstead_render::{FileRoot, FolderEntry, FolderListing};

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
    let Some(root) = owning(roots, path) else {
        return FolderListing::Outside;
    };

    if inside_git(root, path) {
        return FolderListing::UnderGit;
    }

    // The root first, so that a Worktree that has gone says so rather than
    // reading as a folder that has: they are one filesystem answer and two
    // different things to be told.
    let Resolved::At(real_root) = resolve(root) else {
        return FolderListing::RootGone;
    };

    let Resolved::At(real) = resolve(path) else {
        return FolderListing::Missing;
    };

    // And measured again, resolved — which is the half of the bound that a
    // symlink out of the checkout has to get past. The spelling was checked
    // above; this is where the filesystem's own answer is.
    if !real.starts_with(&real_root) {
        return FolderListing::Outside;
    }

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

/// Which root `path` is in, as it is spelled.
///
/// The first one it is under, roots being directories nothing nests inside
/// another. A root itself is in itself: the tree asks for a root's own listing
/// the moment somebody expands it.
///
/// **A path that climbs is under nothing.** `..` is refused outright rather
/// than folded away, and so is a `.`: the tree only ever asks for paths it was
/// handed, so a path spelled with either is a request somebody wrote by hand,
/// and lexical folding is the step that gets a bound wrong. What a symlink out
/// of a checkout does is caught in [`folder`], on the resolved pair.
fn owning<'a>(roots: &'a [FileRoot], path: &Path) -> Option<&'a Path> {
    if !path.is_absolute() || path.components().any(|part| !plain(&part)) {
        return None;
    }

    roots
        .iter()
        .map(|root| Path::new(&root.path))
        .find(|root| path.starts_with(root))
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
}
