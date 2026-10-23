//! The bytes of a picture a diff names, fetched by git's own name for them.
//!
//! Nothing is stored for a picture (ADR-0026, *Fetched by blob id*). Every
//! file in a patch already names its exact bytes — the `index <before>..<after>`
//! line carries git's blob ids, whole ones since both reads that feed the
//! renderer pass `--full-index` — so what a picture in a diff needs to be drawn
//! is a read of that id out of the repository the diff came from, and this is
//! that read.
//!
//! **The object database first, and the Worktree standing in for it.** The
//! repository holds the *before* blob always, and the *after* blob the moment
//! the file is staged or committed. While the file is only in the Worktree its
//! blob is in no object database yet, so where git finds nothing the file at the
//! path the diff named is hashed the way git hashes a blob and served while the
//! hash still matches — and refused once the file has changed, that being a
//! picture the diff no longer describes.
//!
//! An id is matched as a prefix throughout, both against the object database
//! and against the Worktree's hash, so a seven-character id out of an older
//! Set is drawn wherever it is still unambiguous.
//!
//! Blocking: git is a process and a file is read whole, so the endpoint calls
//! this from a blocking task, as the file read and the commit pane's render do.

use std::path::Path;

use sha1::{Digest, Sha1};
use verkstead_render::FileRoot;

use crate::files::{Bound, MAX_BYTES, bound};
use crate::repos::{bytes, feeding};

/// What a read of one picture came to: its bytes, or which of the refusals it
/// is.
///
/// Named apart because each is its own sentence in front of the human — a
/// picture the repository has moved on from, one too large to draw, and a path
/// the Worktree stand-in will not reach.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Picture {
    /// The bytes, exactly as the blob holds them.
    Found(Vec<u8>),

    /// Nothing under that id: not in the object database, and not the file at
    /// the path as it stands either — never written, pruned, or changed since.
    Gone,

    /// Over [`MAX_BYTES`], asked of the size before the bytes are read.
    TooLarge,

    /// A path that is not inside the Worktree.
    Outside,

    /// A path under the Worktree's `.git`.
    UnderGit,
}

/// The picture `id` names in the repository checked out at `worktree`, with
/// the file at `path` under it standing in where the object database has no
/// such blob.
///
/// `path` is the diff's own, relative to the Worktree, and is bound the way the
/// Code pane binds a file read — see [`crate::files::bound`]: nothing outside
/// the Worktree, a symlink out of it included, and nothing under `.git`.
pub(crate) fn read(worktree: &Path, id: &str, path: &str) -> Picture {
    // Hex and nothing else, which is what keeps a revision — `HEAD:secrets` —
    // out of `cat-file`: an id is the one thing this is asked for by. Four is
    // the shortest abbreviation git resolves, and the zeroed id is the side of
    // an added or removed file that has no blob at all.
    let id = id.to_ascii_lowercase();
    if !(4..=64).contains(&id.len())
        || !id.bytes().all(|byte| byte.is_ascii_hexdigit())
        || id.bytes().all(|byte| byte == b'0')
    {
        return Picture::Gone;
    }

    if let Some(stored) = stored(worktree, &id) {
        return stored;
    }

    worktree_copy(worktree, &id, path)
}

/// The blob `id` names in the object database, or `None` where git has no
/// blob by that name — missing, ambiguous, or some other kind of object.
///
/// Asked through `--batch-check` first, which resolves a short id and says the
/// object's type and size without reading it: the cap is asked of the size
/// before any byte is.
fn stored(worktree: &Path, id: &str) -> Option<Picture> {
    let checked = feeding(
        worktree,
        &[
            "cat-file",
            "--batch-check=%(objectname) %(objecttype) %(objectsize)",
        ],
        &format!("{id}\n"),
        &[0],
    )?;

    // `<name> missing` and `<name> ambiguous` are the two answers that are not
    // three words, and neither is a blob.
    let mut words = checked.split_whitespace();
    let (Some(name), Some("blob"), Some(size)) = (words.next(), words.next(), words.next()) else {
        return None;
    };

    if size.parse::<u64>().ok()? > MAX_BYTES {
        return Some(Picture::TooLarge);
    }

    let read = bytes(worktree, &["cat-file", "blob", name], &[0])?;

    // And again on what was read, for the reason the Code pane checks twice.
    if read.len() as u64 > MAX_BYTES {
        return Some(Picture::TooLarge);
    }

    Some(Picture::Found(read))
}

/// The file at `path` under `worktree`, while it still hashes to `id`.
fn worktree_copy(worktree: &Path, id: &str, path: &str) -> Picture {
    // A diff's path is relative to the Worktree, so an absolute one is a
    // request somebody wrote by hand, and is under nothing.
    if path.is_empty() || Path::new(path).is_absolute() {
        return Picture::Outside;
    }

    // One root, the Worktree the diff was read out of, so that the bound is the
    // Code pane's own and not a second one written beside it.
    let roots = [FileRoot {
        repo: String::new(),
        path: worktree.display().to_string(),
        own: true,
        writable: false,
    }];

    let real = match bound(&roots, &worktree.join(path)) {
        Bound::Inside { real, .. } => real,
        Bound::Outside => return Picture::Outside,
        Bound::UnderGit => return Picture::UnderGit,
        Bound::RootGone | Bound::Missing => return Picture::Gone,
    };

    let Ok(about) = std::fs::metadata(&real) else {
        return Picture::Gone;
    };

    if !about.is_file() {
        return Picture::Gone;
    }

    if about.len() > MAX_BYTES {
        return Picture::TooLarge;
    }

    let Ok(read) = std::fs::read(&real) else {
        return Picture::Gone;
    };

    if read.len() as u64 > MAX_BYTES {
        return Picture::TooLarge;
    }

    // Hashed off the bytes that were read rather than asked of git beforehand,
    // so that what is served is exactly what was matched.
    if blob_id(&read).starts_with(id) {
        Picture::Found(read)
    } else {
        Picture::Gone
    }
}

/// The id git gives `bytes` as a blob: SHA-1 over a `blob <length>` header, a
/// NUL and the bytes, spelled hex.
///
/// The raw bytes with no filter applied, which is what `diff --no-index` names
/// an untracked file by — the case this exists for. A SHA-256 repository names
/// its blobs otherwise, and its Worktree copies are refused as changed rather
/// than served wrongly; its stored blobs are read the same as any other.
fn blob_id(bytes: &[u8]) -> String {
    let mut hashing = Sha1::new();
    hashing.update(format!("blob {}\0", bytes.len()));
    hashing.update(bytes);

    format!("{:x}", hashing.finalize())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use super::*;

    /// A repository with one committed picture in it, `logo.svg`.
    fn repository() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("repo");
        std::fs::create_dir_all(&path).unwrap();

        git(&path, &["init", "--initial-branch", "main"]);
        git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
        git(&path, &["config", "user.name", "Verkstead Tests"]);
        git(&path, &["config", "commit.gpgsign", "false"]);
        std::fs::write(path.join("logo.svg"), SVG).unwrap();
        git(&path, &["add", "-A"]);
        git(&path, &["commit", "-m", "first"]);

        (dir, path)
    }

    const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>\n";

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");

        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    #[test]
    fn a_blob_is_hashed_the_way_git_hashes_one() {
        let (_dir, repo) = repository();
        std::fs::write(repo.join("other.svg"), b"<svg/>\x00\xff").unwrap();

        assert_eq!(
            blob_id(&std::fs::read(repo.join("other.svg")).unwrap()),
            git(&repo, &["hash-object", "--no-filters", "other.svg"])
        );
    }

    #[test]
    fn a_stored_blob_is_read_by_its_whole_id_and_by_a_prefix() {
        let (_dir, repo) = repository();
        let id = git(&repo, &["rev-parse", "HEAD:logo.svg"]);

        // Off the disk, so that the bytes can only have come out of git.
        std::fs::remove_file(repo.join("logo.svg")).unwrap();

        assert_eq!(read(&repo, &id, "logo.svg"), Picture::Found(SVG.to_vec()));
        assert_eq!(
            read(&repo, &id[..7], "logo.svg"),
            Picture::Found(SVG.to_vec())
        );
        assert_eq!(
            read(&repo, &id.to_ascii_uppercase(), "logo.svg"),
            Picture::Found(SVG.to_vec())
        );
    }

    #[test]
    fn anything_but_a_blob_id_is_nothing() {
        let (_dir, repo) = repository();
        let tree = git(&repo, &["rev-parse", "HEAD^{tree}"]);

        for asked in ["HEAD:logo.svg", "HEAD", "abc", "", &"0".repeat(40), &tree] {
            assert_eq!(read(&repo, asked, "logo.svg"), Picture::Gone, "{asked:?}");
        }
    }

    #[test]
    fn a_worktree_copy_stands_in_while_it_still_hashes_to_the_id() {
        let (_dir, repo) = repository();
        std::fs::write(repo.join("new.svg"), b"<svg>new</svg>").unwrap();
        let id = blob_id(b"<svg>new</svg>");

        assert_eq!(
            read(&repo, &id, "new.svg"),
            Picture::Found(b"<svg>new</svg>".to_vec())
        );
        assert_eq!(
            read(&repo, &id[..7], "new.svg"),
            Picture::Found(b"<svg>new</svg>".to_vec())
        );

        std::fs::write(repo.join("new.svg"), b"<svg>changed</svg>").unwrap();
        assert_eq!(read(&repo, &id, "new.svg"), Picture::Gone);

        std::fs::remove_file(repo.join("new.svg")).unwrap();
        assert_eq!(read(&repo, &id, "new.svg"), Picture::Gone);
    }

    #[test]
    fn the_stand_in_is_bound_inside_the_worktree() {
        let (dir, repo) = repository();
        std::fs::write(dir.path().join("outside.svg"), b"<svg/>").unwrap();
        let id = blob_id(b"<svg/>");

        assert_eq!(read(&repo, &id, "../outside.svg"), Picture::Outside);
        assert_eq!(
            read(
                &repo,
                &id,
                &dir.path().join("outside.svg").display().to_string()
            ),
            Picture::Outside
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path().join("outside.svg"), repo.join("link.svg"))
                .unwrap();
            assert_eq!(read(&repo, &id, "link.svg"), Picture::Outside);
        }

        std::fs::write(repo.join(".git/picture.svg"), b"<svg/>").unwrap();
        assert_eq!(read(&repo, &id, ".git/picture.svg"), Picture::UnderGit);
    }

    #[test]
    fn a_picture_over_the_cap_is_refused_from_either_source() {
        let (_dir, repo) = repository();
        let large = vec![b'x'; MAX_BYTES as usize + 1];
        std::fs::write(repo.join("large.png"), &large).unwrap();
        let id = blob_id(&large);

        assert_eq!(read(&repo, &id, "large.png"), Picture::TooLarge);

        git(&repo, &["add", "large.png"]);
        std::fs::remove_file(repo.join("large.png")).unwrap();
        assert_eq!(read(&repo, &id, "large.png"), Picture::TooLarge);
    }
}
