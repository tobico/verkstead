//! Following a Conversation's branch to the name a session gave it.
//!
//! A session may decide the work is not what its branch was called and rename
//! the branch in its own Worktree, which is what the naming instruction asks of
//! the first session of a Conversation Verkstead named for itself. Nothing tells
//! Verkstead that happened — a session commits and renames without reporting
//! either — so it is read off the checkout, the way commits are.
//!
//! **Following rather than repairing.** A recorded branch that is not what the
//! Worktree is on used to be one thing: a checkout that had come adrift, put
//! back by rebuilding it. It is now two, and which of them this is has a plain
//! reading — see [`crate::worktrees::renamed`]. A rename moves the record; every
//! other mismatch is still broken and still rebuilds.
//!
//! **One act.** The Conversation's branch and every mirroring companion's move
//! together: a companion left on the empty *mirroring* setting takes its name
//! from the Conversation's, so a record that moved on its own would be a mirror
//! rule resolving to a name no companion branch has. The companions are renamed
//! first and the record written last, so that from the moment anything can read
//! the new name the branches are already under it. A companion the human named
//! is left alone — that name is theirs and it was never the Conversation's.

use std::path::Path;

use sqlx::SqlitePool;

use crate::store;

/// Follow `branch` where the Worktree at `worktree` says it has been renamed to,
/// and give back the name it now has.
///
/// `None` is the ordinary answer and says nothing has moved: the checkout is on
/// the branch the record names, or the mismatch is one of the broken ones the
/// health check already answers. It is also what a follow that could not be
/// finished comes back as — the record is left where it was, and the sweep after
/// this one reads the same rename again.
///
/// Cheap to ask, which it has to be: the commit sweep asks it every couple of
/// seconds for as long as a session runs. A Worktree still on its own branch is
/// settled by one `git symbolic-ref`, and nothing else here happens at all.
pub(crate) async fn follow(
    pool: &SqlitePool,
    conversation_id: i64,
    repo: &Path,
    worktree: &Path,
    branch: &str,
) -> Option<String> {
    let renamed = {
        let repo = repo.to_owned();
        let worktree = worktree.to_owned();
        let branch = branch.to_owned();

        match tokio::task::spawn_blocking(move || {
            crate::worktrees::renamed(&repo, &worktree, &branch)
        })
        .await
        {
            Ok(renamed) => renamed?,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading a Worktree's branch failed");
                return None;
            }
        }
    };

    // Only now, a rename being rare and this being the reading that costs
    // something: the companions are what the same act has to move, and nothing
    // short of a rename moves any of them.
    let conversation = match store::load_conversation(pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a Conversation to follow its rename failed");
            return None;
        }
    };

    mirror(&conversation, &renamed).await;

    if let Err(error) = store::follow_branch(pool, conversation_id, &renamed).await {
        tracing::error!(error = ?error, conversation_id, "following a renamed branch failed");
        return None;
    }

    tracing::info!(
        conversation_id,
        from = branch,
        to = renamed,
        "a session renamed its branch, and the record has followed it",
    );

    Some(renamed)
}

/// Rename every mirroring companion's branch to `branch`.
///
/// The read-write companions left on the empty setting and no others: a
/// companion the human gave a name is on that name whatever the Conversation's
/// is called, and a read-only one is detached and holds no branch at all.
///
/// A rename git will not make is logged and left. What it leaves behind is a
/// companion whose branch is not what the mirror rule now says — the pull
/// request that repository gets at the wrap-up is the thing that would notice —
/// and that is worth saying loudly and worth going on past: the Conversation's
/// own branch has moved, and a record that would not follow it because a
/// companion would not follow it is a Conversation whose commits stop arriving.
async fn mirror(conversation: &store::Conversation, branch: &str) {
    for companion in &conversation.companions {
        if companion.mode != store::CompanionMode::ReadWrite || !companion.branch.is_empty() {
            continue;
        }

        let Some(worktree) = companion.worktree.clone() else {
            continue;
        };

        let renamed = {
            let branch = branch.to_owned();
            tokio::task::spawn_blocking(move || crate::worktrees::rename(&worktree, &branch)).await
        };

        if !matches!(renamed, Ok(true)) {
            tracing::error!(
                conversation_id = conversation.id,
                repo = companion.repo.name,
                branch,
                "a mirroring companion's branch could not be renamed along with \
                 the Conversation's, so it is on a name the mirror rule no \
                 longer resolves to",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::worktrees;

    /// The rename a session makes, followed: the record moves to the name the
    /// checkout is on, and everything that reads the record reads the new name.
    ///
    /// Which is the whole point of following rather than repairing — the health
    /// check agrees about the same directory the moment the record does, and the
    /// directory itself is untouched.
    #[tokio::test]
    async fn a_session_renaming_its_branch_moves_the_record() {
        let mut bench = Workbench::new().await;
        let conversation = bench.conversation("verkstead-7f3a", &[]).await;

        git(&bench.worktree, &["branch", "-m", "rate-limiting"]);

        assert_eq!(
            follow(
                &bench.pool,
                conversation,
                &bench.repo,
                &bench.worktree,
                "verkstead-7f3a",
            )
            .await
            .as_deref(),
            Some("rate-limiting"),
        );

        assert_eq!(
            store::conversation_branch(&bench.pool, conversation)
                .await
                .unwrap()
                .as_deref(),
            Some("rate-limiting"),
        );
        assert!(
            worktrees::healthy(&bench.repo, &bench.worktree, "rate-limiting"),
            "the health check agrees about the checkout it was calling broken",
        );

        // Whose the name is has not moved with it: the Conversation was started
        // on a name Verkstead invented and it is still on one — a better name,
        // picked by a session rather than typed by the human.
        let loaded = store::load_conversation(&bench.pool, conversation)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.branch, "rate-limiting");
        assert!(!loaded.branch_named);
    }

    /// A name the human typed is followed the same way and stays theirs.
    ///
    /// Nothing asks a session to rename a branch somebody named — the
    /// instruction goes out only where Verkstead named it — but a session that
    /// renamed one anyway has left the record describing a branch that is not
    /// there, and describing what is there is what following is for.
    #[tokio::test]
    async fn a_name_the_human_typed_is_still_theirs_after_it_moves() {
        let mut bench = Workbench::new().await;
        let conversation = bench.named_conversation("rate-limiting").await;

        git(&bench.worktree, &["branch", "-m", "throttling"]);

        assert_eq!(
            follow(
                &bench.pool,
                conversation,
                &bench.repo,
                &bench.worktree,
                "rate-limiting",
            )
            .await
            .as_deref(),
            Some("throttling"),
        );

        let loaded = store::load_conversation(&bench.pool, conversation)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.branch, "throttling");
        assert!(loaded.branch_named);
    }

    /// A mirroring companion's branch is renamed in the same act, and a
    /// companion the human named keeps the name they gave it.
    ///
    /// The mirror rule resolves to the Conversation's branch, so a record that
    /// moved without the companion branches moving would be a rule resolving to
    /// a name no companion is on.
    #[tokio::test]
    async fn a_mirroring_companion_is_renamed_along_and_a_named_one_is_not() {
        let mut bench = Workbench::new().await;
        let conversation = bench
            .conversation("verkstead-7f3a", &["", "companion-of-their-own"])
            .await;

        git(&bench.worktree, &["branch", "-m", "rate-limiting"]);

        follow(
            &bench.pool,
            conversation,
            &bench.repo,
            &bench.worktree,
            "verkstead-7f3a",
        )
        .await
        .expect("the Conversation's own branch moved");

        let companions = store::companions(&bench.pool, conversation).await.unwrap();

        let mirroring = companions[0].worktree.clone().unwrap();
        let named = companions[1].worktree.clone().unwrap();

        assert_eq!(
            head(&mirroring),
            "rate-limiting",
            "the mirroring companion follows the Conversation's branch",
        );
        assert_eq!(
            head(&named),
            "companion-of-their-own",
            "and the one the human named keeps the name they gave it",
        );
    }

    /// Nothing that is not a rename moves anything.
    ///
    /// Asked every couple of seconds for as long as a session runs, so the
    /// answer on a Worktree still on its own branch has to be *nothing
    /// happened* — and the answer on a broken one has to be the same, the
    /// rebuild being what deals with that.
    #[tokio::test]
    async fn a_worktree_that_was_not_renamed_leaves_the_record_alone() {
        for state in ["untouched", "wandered", "detached"] {
            let mut bench = Workbench::new().await;
            let conversation = bench.conversation("verkstead-7f3a", &[]).await;

            match state {
                "untouched" => {}
                "wandered" => git(&bench.worktree, &["checkout", "-b", "elsewhere"]),
                _ => git(&bench.worktree, &["checkout", "--detach"]),
            }

            assert_eq!(
                follow(
                    &bench.pool,
                    conversation,
                    &bench.repo,
                    &bench.worktree,
                    "verkstead-7f3a",
                )
                .await,
                None,
                "{state} is not a rename",
            );
            assert_eq!(
                store::conversation_branch(&bench.pool, conversation)
                    .await
                    .unwrap()
                    .as_deref(),
                Some("verkstead-7f3a"),
                "so the record stands: {state}",
            );
        }
    }

    /// A store, a Repo, a companion Repo and the checkouts of a Conversation
    /// that has started grilling — which is the state a rename is read in.
    struct Workbench {
        /// Held for the directories below, which go when it does.
        _dir: tempfile::TempDir,
        pool: sqlx::SqlitePool,
        data: PathBuf,
        repo: PathBuf,
        repo_id: i64,
        companions: Vec<(i64, PathBuf)>,
        worktree: PathBuf,
    }

    impl Workbench {
        async fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let data = dir.path().join("data");
            let pool = crate::open_database(&dir.path().join("verkstead.db"))
                .await
                .unwrap();

            let repo = repository(&dir.path().join("verkstead"));
            let repo_id = store::register_repo(&pool, &repo, "verkstead", "main")
                .await
                .unwrap()
                .unwrap()
                .id;

            let companions = ["askance", "bookshelf"]
                .iter()
                .map(|name| {
                    let path = repository(&dir.path().join(name));

                    (path, *name)
                })
                .collect::<Vec<_>>();

            let mut registered = Vec::new();

            for (path, name) in companions {
                let id = store::register_repo(&pool, &path, name, "main")
                    .await
                    .unwrap()
                    .unwrap()
                    .id;

                registered.push((id, path));
            }

            Self {
                _dir: dir,
                pool,
                data,
                repo,
                repo_id,
                companions: registered,
                worktree: PathBuf::new(),
            }
        }

        /// A Conversation on a name Verkstead invented, grilling, with one
        /// companion per entry of `companions` — each entry being that
        /// companion's branch name, where empty is *mirroring*.
        async fn conversation(&mut self, branch: &str, companions: &[&str]) -> i64 {
            let id = store::start_unnamed_conversation(&self.pool, self.repo_id, branch)
                .await
                .unwrap()
                .unwrap();

            self.start(id, branch, companions).await
        }

        /// And the same on a name the human typed.
        async fn named_conversation(&mut self, branch: &str) -> i64 {
            let id = store::start_conversation(&self.pool, self.repo_id, branch)
                .await
                .unwrap()
                .unwrap();

            self.start(id, branch, &[]).await
        }

        /// The checkouts a grill start makes, and the record of them.
        ///
        /// The checkout goes on the Workbench rather than being handed back:
        /// there is one Conversation to a Workbench, so `bench.worktree` is the
        /// Worktree in every test that has one.
        async fn start(&mut self, id: i64, branch: &str, companions: &[&str]) -> i64 {
            let mine = self.data.join("worktrees/verkstead-work");

            assert!(worktrees::add(&self.repo, &mine, branch, "HEAD"));

            let mut made = Vec::new();

            for (nth, named) in companions.iter().enumerate() {
                let (repo_id, repo) = &self.companions[nth];
                let path = self.data.join(format!("worktrees/companion-{nth}"));
                let name = match named.is_empty() {
                    true => branch,
                    false => named,
                };

                assert!(worktrees::add(repo, &path, name, "HEAD"));

                store::add_companion(&self.pool, id, *repo_id)
                    .await
                    .unwrap();
                store::configure_companion(
                    &self.pool,
                    id,
                    *repo_id,
                    store::Change::Mode(store::CompanionMode::ReadWrite),
                )
                .await
                .unwrap();
                store::configure_companion(&self.pool, id, *repo_id, store::Change::Branch(named))
                    .await
                    .unwrap();

                made.push(store::CompanionWorktree {
                    repo_id: *repo_id,
                    path,
                    base_commit: Some(head_commit(repo)),
                });
            }

            store::start_grilling(&self.pool, id, &head_commit(&self.repo), &mine, &made)
                .await
                .unwrap();

            self.worktree = mine;

            id
        }
    }

    /// A git repository at `path`, with one commit on `main`.
    fn repository(path: &Path) -> PathBuf {
        std::fs::create_dir_all(path).unwrap();

        git(path, &["init", "--initial-branch", "main"]);
        git(path, &["config", "user.email", "test@verkstead.invalid"]);
        git(path, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
        git(path, &["add", "README.md"]);
        git(path, &["commit", "-m", "first"]);

        path.to_owned()
    }

    /// The branch checked out at `path`, by name.
    fn head(path: &Path) -> String {
        read(path, &["symbolic-ref", "--short", "HEAD"])
    }

    /// The commit `path` is on.
    fn head_commit(path: &Path) -> String {
        read(path, &["rev-parse", "HEAD"])
    }

    fn read(path: &Path, args: &[&str]) -> String {
        crate::repos::git(path, args)
            .expect("git should answer in these tests")
            .trim()
            .to_owned()
    }

    fn git(path: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(path)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");
    }
}
