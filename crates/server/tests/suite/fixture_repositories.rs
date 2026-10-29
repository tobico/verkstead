//! A git repository for a test, copied from a template rather than made.
//!
//! Making one is five `git` processes — init, two configs, add, commit — and a
//! process start on Windows is slow enough that the suites making one a test
//! spent a sixth of their time there. So the repository is made once, as a
//! template in the cargo target directory, and every test after copies the
//! directory tree: a file copy and no `git` at all. Git reads a copy as the
//! repository it is, nothing in `.git` naming its own path.
//!
//! **Once per machine rather than once per test.** The first test to want one
//! makes it, under a file lock, so that tests in other processes — nextest runs
//! each in a process of its own — wait for that one rather than each making
//! their own. What the lock guards is a marker written last, and the marker is
//! what a copier goes by: a template found without it was left half-made by a
//! process that died holding the lock, which the lock's release says, and it is
//! made again rather than trusted. Rename the directory below when what the
//! template holds changes, or a target directory that has one already goes on
//! copying the old.
//!
//! What a test does to its copy is its own: the copy is wherever the test put
//! it, under its own temp directory, and goes with it.
//!
//! **Who uses it.** `conversations` and `repos`, whose repositories were these
//! exactly. `joining` makes no repository at all. The `worktrees` tests are unit
//! tests inside the server crate, a test binary this module is not part of. The
//! suites with a repository of their own shape — a second branch, a remote, a
//! history — make it the way they always did.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where the templates are, in the target directory cargo gives integration
/// tests for their own files.
fn templates() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixture-repositories-1")
}

/// A git repository at `path`, with one commit on `main` so it has a branch to
/// call its default, and an identity configured so a test can commit in it.
pub fn repository(path: PathBuf) -> PathBuf {
    copy(&made().join(ONE_COMMIT), &path);

    path
}

/// A repository at `upstream` as [`repository`] makes, and a clone of it at
/// `clone` whose origin is `upstream`, with the same identity configured.
pub fn repository_and_clone(upstream: PathBuf, clone: PathBuf) -> (PathBuf, PathBuf) {
    let templates = made();
    copy(&templates.join(ONE_COMMIT), &upstream);
    copy(&templates.join(CLONED), &clone);

    // The one place a clone names anything outside itself. The template's is a
    // placeholder, so this is a substitution rather than a parse.
    let config = clone.join(".git").join("config");
    let quoted = upstream
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let text = fs::read_to_string(&config).unwrap();
    assert!(
        text.contains(UPSTREAM),
        "the clone template names no upstream"
    );
    fs::write(&config, text.replace(UPSTREAM, &format!("\"{quoted}\""))).unwrap();

    (upstream, clone)
}

const ONE_COMMIT: &str = "one-commit";
const CLONED: &str = "cloned";
const MARKER: &str = "made";

/// What the clone template's origin is, until a copy says where its own is.
const UPSTREAM: &str = "@UPSTREAM@";

/// The templates' directory, made first if no test has made it yet.
///
/// `File::lock` is newer than the workspace's `rust-version`, which no test is
/// built against: every check builds with the toolchain `ci.yml` pins.
#[allow(clippy::incompatible_msrv)]
fn made() -> PathBuf {
    let templates = templates();
    if templates.join(MARKER).exists() {
        return templates;
    }

    fs::create_dir_all(&templates).unwrap();
    let lock = File::create(templates.with_extension("lock")).unwrap();
    lock.lock().unwrap();

    // Again under the lock: whoever held it before may have made them.
    if !templates.join(MARKER).exists() {
        for half_made in [ONE_COMMIT, CLONED] {
            let _ = fs::remove_dir_all(templates.join(half_made));
        }

        let one_commit = templates.join(ONE_COMMIT);
        fs::create_dir_all(&one_commit).unwrap();
        git(&one_commit, &["init", "--initial-branch", "main"]);
        identity(&one_commit);
        fs::write(one_commit.join("README.md"), "# a repository\n").unwrap();
        git(&one_commit, &["add", "README.md"]);
        git(&one_commit, &["commit", "-m", "first"]);

        let cloned = templates.join(CLONED);
        git(
            &templates,
            &[
                "clone",
                &one_commit.to_string_lossy(),
                &cloned.to_string_lossy(),
            ],
        );
        // A clone inherits no identity, and a machine that has no global one —
        // which is every machine the checks run on — cannot commit in it.
        identity(&cloned);
        git(&cloned, &["config", "remote.origin.url", UPSTREAM]);

        File::create(templates.join(MARKER)).unwrap();
    }

    lock.unlock().unwrap();

    templates
}

fn identity(repo: &Path) {
    git(repo, &["config", "user.email", "test@verkstead.invalid"]);
    git(repo, &["config", "user.name", "Verkstead Test"]);
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git should be on the PATH for these tests");

    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// The tree at `from`, file for file, at `to`.
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let into = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &into);
        } else {
            fs::copy(entry.path(), &into).unwrap();
        }
    }
}
