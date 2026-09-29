//! The git leg of a move: the Conversation's branch packed against what the far
//! end already holds, and the working changes of its Worktree beside it
//! (ADR-0020, *Transfer*).
//!
//! **After the record has landed**, because a Worktree hangs off a Conversation
//! and the far end needs one to hang it from — see [`super::across`], where the
//! order is. This is the leg that carries the *work*: the row and the Timeline
//! are the two before it.
//!
//! **The bundle is packed against what the far end said it held.** A machine
//! that cloned the same repository yesterday is sent the branch and not the
//! history under it, so the far end is asked for the tips it holds of the
//! matched Repo first and those become the bundle's prerequisites — see
//! [`crate::peer::checkouts`], which is the end that answers. A far end that
//! holds every commit of the branch already gets no bundle at all: git refuses
//! to make an empty one, and what is left to do over there is the ref.
//!
//! **And the working changes are two different things.** Tracked changes travel
//! as a **binary patch** — `git diff --binary` against `HEAD`, which is what
//! carries a changed image or a compiled fixture and what [`crate::diffs`]
//! deliberately does not produce: the Diff a Question Set carries is prose for a
//! human, with binary left out. Untracked files git does not ignore travel as
//! raw bytes, each by its path relative to the Worktree. **Ignored files stay
//! behind** and the far end builds its own — a `target/` is that machine's to
//! make, and on the far side of a move it may not even be the same operating
//! system.
//!
//! **The line endings are settled and cost nothing.** A binary patch is written
//! in index form and `git apply` re-applies whatever working-tree convention the
//! target keeps, so a tree with `core.autocrlf` on and one with it off exchange
//! patches in both directions and land byte-identical content in each machine's
//! own convention. Untracked files are converted by neither end, which is right
//! for a file git is not tracking.
//!
//! **And the branch is followed where a session renamed it.** What goes over is
//! the name the checkout is actually on rather than the name the record was
//! written with — see [`crate::renames`], which does nothing at all where
//! nothing was renamed.
//!
//! **Nothing of this machine's own paths crosses.** The far end names its own
//! Worktree under its own Data Directory, the way the memory sync names its own
//! parts: what travels is a branch, some commits and some bytes.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use verkstead_render::{CheckoutAcross, TipsThere, UntrackedFile};

use crate::AppState;
use crate::peer::checkouts::{ONE_TRANSFER_CHECKOUT, tips_of};
use crate::relaying::{self, Call, Streamed, as_json};
use crate::repos::{self, git};
use crate::store;

/// The most the far end's tips may be: **one hundred and twenty-eight
/// kilobytes**.
///
/// A tip is forty hex digits and a pair of quotes on the wire, and the far end
/// answers with at most [`crate::peer::checkouts::MOST_THE_TIPS_ARE`] of them,
/// so this is comfortably twice what a full answer weighs. What it is for is
/// what every bound across a link is for — a machine that answers and then
/// writes without stopping.
const MOST_THE_TIPS_WEIGH: usize = 128 * 1024;

/// And the most a checkout may weigh on the way across: **sixty-four
/// megabytes**, counted over the bundle and the working changes together.
///
/// [`store::MOST_A_SLICE_IS`]'s bound said again over the other half of a move,
/// and for its reason: past it the move is refused whole, naming the
/// Conversation, rather than carried in part. What makes a checkout large is an
/// untracked directory of screenshots or a branch nobody has pushed anywhere;
/// ignored files are not in it at all.
pub(crate) const MOST_A_CHECKOUT_IS: usize = 64 * 1024 * 1024;

/// The Conversation with its branch read off the checkout rather than off the
/// record.
///
/// **The first thing a move does**, because the name is what both legs carry: a
/// row written under the old name would be a copy whose branch is not the branch
/// it arrived with, and a bundle packed under it would be a bundle of a ref that
/// is not there.
///
/// Nothing tells Verkstead a session renamed its branch — it is read off the
/// checkout, the way commits are — and the sweep that ordinarily follows one
/// runs only while a session does. A move runs at the end of a turn, which is
/// exactly when that sweep has stopped, so a rename in the last turn is a record
/// one name behind at precisely this moment. See [`crate::renames`], which does
/// nothing at all where nothing was renamed, which is nearly every move.
pub(crate) async fn followed(
    state: &AppState,
    mut conversation: store::Conversation,
) -> store::Conversation {
    let Some(worktree) = conversation.worktree.clone() else {
        return conversation;
    };

    if let Some(renamed) = crate::renames::follow(
        &state.pool,
        conversation.id,
        &conversation.repo.path,
        &worktree,
        &conversation.branch,
    )
    .await
    {
        conversation.branch = renamed;
    }

    conversation
}

/// Put the Conversation's branch and its working changes over to `device`,
/// against the copy it numbered `there`.
///
/// `repo_there` is which of that device's Repos this repository is, as the match
/// settled it a moment ago: the tips are asked of that Repo and the far end cuts
/// its Worktree from it.
///
/// The branch is the one on [`store::Conversation::branch`], which a move has
/// already put through [`followed`] — so this leg and the row that went before
/// it name the same branch by construction.
///
/// `Err` is the sentence the Notice carries, the way every other leg of a move
/// reports itself.
pub(crate) async fn across(
    state: &AppState,
    device: &str,
    conversation: &store::Conversation,
    repo_there: i64,
    there: i64,
) -> Result<(), String> {
    let branch = conversation.branch.clone();
    let tips = tips(state, device, repo_there).await?;

    let repo = conversation.repo.path.clone();
    let worktree = conversation.worktree.clone();
    let base_commit = conversation.base_commit.clone();
    let base_ref = conversation.base_ref.clone();
    let packing = branch.clone();

    let going = tokio::task::spawn_blocking(move || {
        packed(
            &repo,
            worktree.as_deref(),
            &packing,
            base_commit,
            base_ref,
            &tips,
        )
    })
    .await
    .map_err(|why| format!("the branch and the working changes could not be read: {why}"))??;

    let saying = serde_json::to_vec(&going)
        .map_err(|why| format!("the checkout could not be written down to send: {why}"))?;

    relaying::put_to(
        state.devices.as_ref(),
        device,
        Call {
            method: reqwest::Method::POST,
            onwards: ONE_TRANSFER_CHECKOUT.replace("{id}", &there.to_string()),
            headers: as_json(),
            body: Streamed::saying(saying),
        },
    )
    .await
    .map(|_| ())
    .map_err(|refusal| refusal.saying)
}

/// The tips that device holds of `repo_there`.
///
/// A reading rather than a press, and a failure is the whole move's: a bundle
/// packed against nothing would carry a repository's entire history down a link
/// for work that is one commit deep, and guessing at what the far end holds is
/// how a bundle arrives unopenable.
async fn tips(state: &AppState, device: &str, repo_there: i64) -> Result<Vec<String>, String> {
    let held: TipsThere = relaying::word_from(
        state.devices.as_ref(),
        device,
        relaying::asking(tips_of(repo_there)),
        MOST_THE_TIPS_WEIGH,
    )
    .await
    .map_err(|refusal| refusal.saying)?;

    Ok(held.tips)
}

/// The branch and the working changes, read off this machine.
///
/// Blocking throughout: every part of it is a git run over a checkout, and a
/// bundle of a repository nobody has pushed anywhere is not a quick one.
///
/// A Conversation with no Worktree — one whose directory was swept out from
/// under it — carries its branch and nothing else. There is nothing uncommitted
/// to find and nowhere to look for it, and the far end cuts a checkout from the
/// branch either way.
fn packed(
    repo: &Path,
    worktree: Option<&Path>,
    branch: &str,
    base_commit: Option<String>,
    base_ref: Option<String>,
    tips: &[String],
) -> Result<CheckoutAcross, String> {
    let named = format!("refs/heads/{branch}");

    let Some(commit) = crate::worktrees::resolve(repo, &named) else {
        return Err(format!(
            "the branch {branch} is not in this device's copy of the repository any more",
        ));
    };

    let bundle = bundle(repo, &named, tips)?;

    let mut weighs = bundle.as_ref().map_or(0, Vec::len);

    let patch = match worktree {
        Some(worktree) => tracked(worktree),
        None => None,
    };

    weighs += patch.as_ref().map_or(0, Vec::len);

    let untracked = match worktree {
        Some(worktree) => untracked(worktree, &mut weighs)?,
        None => Vec::new(),
    };

    if weighs > MOST_A_CHECKOUT_IS {
        return Err(format!(
            "the checkout of {branch} is larger than the {MOST_A_CHECKOUT_IS} bytes one may be \
             to cross a link",
        ));
    }

    Ok(CheckoutAcross {
        branch: branch.to_owned(),
        commit,
        base_commit,
        base_ref,
        bundle: bundle.map(|bundle| STANDARD.encode(&bundle)),
        patch: patch.map(|patch| STANDARD.encode(&patch)),
        untracked,
    })
}

/// The branch as a bundle, packed against the tips the far end holds and this
/// device has.
///
/// **Only the tips this device can resolve.** A prerequisite is a commit the
/// packing side has to walk, so a tip that is the far end's own work and nothing
/// of this one's is dropped rather than handed to git, which would refuse the
/// whole bundle over it.
///
/// `None` is a far end that already holds every commit of the branch, which git
/// refuses to make a bundle of at all — *Refusing to create empty bundle*. Asked
/// as a count first rather than read off that refusal, because an exit code
/// cannot say which failure it was.
fn bundle(repo: &Path, named: &str, tips: &[String]) -> Result<Option<Vec<u8>>, String> {
    let held = resolvable(repo, tips);

    let mut counting = vec!["rev-list", "--count", named];

    if !held.is_empty() {
        counting.push("--not");
        counting.extend(held.iter().map(String::as_str));
    }

    let Some(count) = git(repo, &counting) else {
        return Err("this device's copy of the repository could not be walked".to_owned());
    };

    // Nothing to pack: the far end is standing on the branch's own tip already,
    // or on something that reaches it. The ref is all that is left to write over
    // there, and the commit on the message is what it is written at.
    if count.trim() == "0" {
        return Ok(None);
    }

    let mut packing = vec!["bundle", "create", "-", named];

    if !held.is_empty() {
        packing.push("--not");
        packing.extend(held.iter().map(String::as_str));
    }

    // Bytes rather than text: a bundle is a pack file, and a lossy read of one
    // is a bundle that will not open — see [`crate::repos::bytes`].
    repos::bytes(repo, &packing, &[0])
        .map(Some)
        .ok_or_else(|| "the branch could not be packed into a bundle".to_owned())
}

/// Which of `tips` this device's copy of the repository actually has.
///
/// One run of `cat-file --batch-check` for the lot rather than one apiece: the
/// far end answers with as many tips as it has refs, and a process per ref would
/// be a move that took a minute to start.
fn resolvable(repo: &Path, tips: &[String]) -> Vec<String> {
    // Nothing that is not plainly an object id, because these go on a command
    // line: what the far end sends is read as data rather than trusted as an
    // argument.
    let asking: Vec<&String> = tips.iter().filter(|tip| is_an_object_id(tip)).collect();

    if asking.is_empty() {
        return Vec::new();
    }

    let asked: String = asking.iter().map(|tip| format!("{tip}\n")).collect();

    let Some(answered) = repos::feeding(repo, &["cat-file", "--batch-check"], &asked, &[0]) else {
        return Vec::new();
    };

    // `<oid> <type> <size>` for one it has, `<name> missing` for one it has not.
    answered
        .lines()
        .filter_map(|line| {
            let (id, said) = line.split_once(' ')?;

            said.starts_with("commit").then(|| id.to_owned())
        })
        .collect()
}

/// Whether a word off the wire is the spelling of a git object id, and so is
/// safe to hand to git as one.
///
/// Hex and nothing else, and long enough to be an id rather than a name: what
/// this keeps out is a member one version ahead sending something that reads as
/// an option, and a ref name arriving where a commit was asked for.
fn is_an_object_id(tip: &str) -> bool {
    tip.len() >= 40 && tip.len() <= 64 && tip.chars().all(|letter| letter.is_ascii_hexdigit())
}

/// The tracked changes of a Worktree: everything not in the last commit, staged
/// or not, as a binary patch.
///
/// `None` where the tree has nothing uncommitted in it, and where the directory
/// is not one git will answer about.
fn tracked(worktree: &Path) -> Option<Vec<u8>> {
    // Against `HEAD`, which is [`crate::diffs`]'s read one flag along: the flag
    // is what makes a changed image travel, and what the Diff a Set carries
    // leaves out on purpose.
    let patch = repos::bytes(worktree, &["diff", "--binary", "HEAD"], &[0])?;

    (!patch.is_empty()).then_some(patch)
}

/// And the untracked files git does not ignore, each with its bytes.
///
/// `weighs` is carried through rather than measured at the end, so that a
/// Worktree holding more than a link will take is refused before this device has
/// read the whole of it into memory.
///
/// A file that has gone between the listing and the read is left out with a line
/// in the log: the session is over by the time a move runs, but a checkout is a
/// directory on a machine somebody else is also using.
fn untracked(worktree: &Path, weighs: &mut usize) -> Result<Vec<UntrackedFile>, String> {
    let listed = git(
        worktree,
        &["ls-files", "-z", "--others", "--exclude-standard"],
    )
    .unwrap_or_default();

    let mut files = Vec::new();

    for path in listed.split('\0').filter(|path| !path.is_empty()) {
        let at = worktree.join(path);

        let bytes = match std::fs::read(&at) {
            Ok(bytes) => bytes,

            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!(
                    path,
                    "an untracked file went from the Worktree while it was being carried, so it \
                     did not travel",
                );

                continue;
            }

            Err(why) => {
                return Err(format!(
                    "the untracked file {path} could not be read: {why}"
                ));
            }
        };

        *weighs += bytes.len();

        if *weighs > MOST_A_CHECKOUT_IS {
            return Err(format!(
                "the checkout is larger than the {MOST_A_CHECKOUT_IS} bytes one may be to cross \
                 a link",
            ));
        }

        files.push(UntrackedFile {
            path: path.to_owned(),
            bytes: STANDARD.encode(&bytes),
        });
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use super::*;

    /// Run git in `dir`, and fail the test where it does not.
    fn run(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git should be on the PATH for these tests");

        assert!(status.success(), "git {args:?} in {}", dir.display());
    }

    /// A repository with one commit on `main`, which is a base for a branch to
    /// stand on.
    fn repository(path: PathBuf) -> PathBuf {
        std::fs::create_dir_all(&path).unwrap();

        run(&path, &["init", "--initial-branch", "main"]);
        run(&path, &["config", "user.email", "test@verkstead.invalid"]);
        run(&path, &["config", "user.name", "Verkstead Test"]);

        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();

        run(&path, &["add", "README.md"]);
        run(&path, &["commit", "-m", "first"]);

        path
    }

    /// The commit something resolves to, as a test reads one.
    fn at(repo: &Path, named: &str) -> String {
        crate::worktrees::resolve(repo, named).expect("it resolves")
    }

    /// A branch with a commit of its own on it, off `main`.
    fn branched(repo: &Path, branch: &str) {
        run(repo, &["checkout", "-b", branch]);
        std::fs::write(repo.join("work.txt"), "the work\n").unwrap();
        run(repo, &["add", "work.txt"]);
        run(repo, &["commit", "-m", "the work"]);
    }

    /// **The bundle is packed against what the far end said it held.** A machine
    /// standing on the base is sent the branch; the base itself is a
    /// prerequisite rather than cargo, which is what a bundle that will not open
    /// in an empty repository says.
    #[test]
    fn the_bundle_is_packed_against_the_tips_the_far_end_holds() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository(dir.path().join("verkstead"));

        let base = at(&repo, "refs/heads/main");
        branched(&repo, "rate-limiting");

        let packed = bundle(
            &repo,
            "refs/heads/rate-limiting",
            std::slice::from_ref(&base),
        )
        .unwrap()
        .expect("the branch is ahead of the base, so there is something to pack");

        let file = dir.path().join("packed.bundle");
        std::fs::write(&file, &packed).unwrap();

        // Where the base is, the bundle opens. The other repository here is the
        // same one: what a clone of it would hold of this branch's history is
        // exactly the base.
        assert!(
            git(&repo, &["bundle", "verify", &file.display().to_string()],).is_some(),
            "the bundle opens where its prerequisite is",
        );

        // And where it is not, it does not — which is what says the history under
        // the branch was left behind rather than carried.
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        run(&empty, &["init", "--initial-branch", "main"]);

        assert!(
            git(&empty, &["bundle", "verify", &file.display().to_string()],).is_none(),
            "a bundle packed against the base does not carry the base",
        );
    }

    /// **And a far end holding the branch already is sent no bundle at all.**
    /// Git refuses to make an empty one, so the count is asked first and the ref
    /// is the whole of what is left to write over there.
    #[test]
    fn a_far_end_that_holds_the_branch_is_sent_no_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository(dir.path().join("verkstead"));

        branched(&repo, "rate-limiting");

        let tip = at(&repo, "refs/heads/rate-limiting");

        assert_eq!(
            bundle(&repo, "refs/heads/rate-limiting", &[tip]).unwrap(),
            None,
            "there is nothing to pack for a machine standing on the tip",
        );
    }

    /// **A tip this device has never heard of is not handed to git.** The far
    /// end's own work is a commit this one cannot walk, and a prerequisite it
    /// cannot walk is a bundle git refuses to make.
    #[test]
    fn a_tip_this_device_does_not_have_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository(dir.path().join("verkstead"));

        let base = at(&repo, "refs/heads/main");
        branched(&repo, "rate-limiting");

        let theirs = "0".repeat(40);

        assert_eq!(
            resolvable(&repo, &[base.clone(), theirs.clone()]),
            vec![base.clone()],
            "only the commits this device has are prerequisites",
        );

        assert!(
            bundle(&repo, "refs/heads/rate-limiting", &[base, theirs])
                .unwrap()
                .is_some(),
            "the bundle is still packed, against what is left",
        );
    }

    /// And nothing that is not plainly an object id reaches a command line at
    /// all: what the far end sends is data rather than an argument.
    #[test]
    fn only_an_object_id_is_taken_for_one() {
        assert!(is_an_object_id(&"a".repeat(40)));
        assert!(is_an_object_id(&"0".repeat(64)));

        assert!(!is_an_object_id("--upload-pack=rm -rf /"));
        assert!(!is_an_object_id("refs/heads/main"));
        assert!(!is_an_object_id(&"a".repeat(39)));
        assert!(!is_an_object_id(&"a".repeat(65)));
        assert!(!is_an_object_id(""));
    }

    /// **The tracked changes carry a changed binary**, which is the whole reason
    /// this is a read of its own rather than the Diff a Question Set carries.
    #[test]
    fn the_patch_carries_a_changed_binary_file() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository(dir.path().join("verkstead"));

        std::fs::write(repo.join("fixture.bin"), [0u8, 1, 2, 3]).unwrap();
        run(&repo, &["add", "fixture.bin"]);
        run(&repo, &["commit", "-m", "a fixture"]);

        std::fs::write(repo.join("fixture.bin"), [255u8, 254, 253]).unwrap();

        let patch = tracked(&repo).expect("the tree has something uncommitted in it");
        let patch = String::from_utf8_lossy(&patch).into_owned();

        assert!(
            patch.contains("GIT binary patch"),
            "the changed binary is in the patch: {patch}",
        );
    }

    /// And a clean tree carries none.
    #[test]
    fn a_clean_tree_carries_no_patch() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository(dir.path().join("verkstead"));

        assert_eq!(tracked(&repo), None);
    }

    /// **Untracked files travel and ignored files stay behind**, which is the
    /// line between work somebody did and a build the far end makes for itself.
    #[test]
    fn the_untracked_files_travel_and_the_ignored_ones_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repository(dir.path().join("verkstead"));

        std::fs::write(repo.join(".gitignore"), "target/\n").unwrap();
        run(&repo, &["add", ".gitignore"]);
        run(&repo, &["commit", "-m", "ignore the build"]);

        std::fs::write(repo.join("notes.md"), "some notes\n").unwrap();
        std::fs::create_dir_all(repo.join("target")).unwrap();
        std::fs::write(repo.join("target").join("built"), "a build\n").unwrap();

        let mut weighs = 0;
        let carried = untracked(&repo, &mut weighs).unwrap();

        let paths: Vec<&str> = carried.iter().map(|file| file.path.as_str()).collect();

        assert_eq!(paths, ["notes.md"], "the build stays behind");
        assert_eq!(
            STANDARD.decode(&carried[0].bytes).unwrap(),
            b"some notes\n",
            "and the bytes are the bytes",
        );
        assert_eq!(weighs, b"some notes\n".len(), "counted towards the bound");
    }
}
