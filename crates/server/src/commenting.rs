//! One comment on every pull request a Conversation holds.
//!
//! The last act of the one-click share: the file is built ([`crate::sharing`]),
//! published where a link reaches it ([`crate::publishing`]), and then said —
//! once on each pull request the work ended up on, the Conversation's own
//! repository's and every companion's alike.
//!
//! **A comment rather than an edit.** Nothing here touches a pull request's
//! description: the description is whoever opened it writing about the work, and
//! a share is somebody handing a colleague a copy of the record. Sharing again
//! is a fresh snapshot, a fresh publish and a fresh comment, and what was said
//! before goes on standing where it was said — a comment nobody can be surprised
//! by is one that was never rewritten under them.
//!
//! **Every pull request is tried, whatever became of the ones before it.** They
//! are separate repositories and separate permissions, so one that has been
//! deleted or one the token may not write on says nothing about the next. What
//! could not be reached comes back named — see [`Landing`] — because the share
//! is published either way: the human's next move is to paste the link
//! themselves, and they can only do that if they are told where it did not go.

use crate::store;

/// What became of the comment on one pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Landing {
    /// Which pull request, as a human names one.
    pub(crate) number: i64,

    /// And which repository that number is in, where it is not the
    /// Conversation's own — the label a pull request's card draws, and `None`
    /// for the same reason: unlabeled means the repo the work is in.
    pub(crate) repo: Option<String>,

    /// Where the comment landed, or what the Forge said about why it did not.
    pub(crate) landed: Result<String, String>,
}

/// Say `body` on every one of them, and answer what became of each.
///
/// In the order they were recorded, which is the Conversation's own repository
/// first and the companions as the wrap-up found them — the order the workbench
/// draws their cards in, so a report of what happened reads down the same list.
///
/// Blocking from end to end: one call per pull request, and a Conversation ends
/// on one per repository it was worked in. The caller is on `spawn_blocking`.
pub(crate) fn on_each(
    gh: &dyn crate::forges::Forge,
    token: &str,
    pulls: &[(store::Repo, store::PullRequest)],
    body: &str,
) -> Vec<Landing> {
    pulls
        .iter()
        .map(|(repo, pull)| Landing {
            number: pull.number,
            repo: pull.repo.clone(),
            landed: gh
                .comment(&repo.path, token, pull.number, body)
                .map_err(|trouble| trouble.why()),
        })
        .collect()
}

/// Asked of GitHub's own component with a stand-in behind it — see
/// [`crate::stand_in`].
///
/// A multi-thread runtime, because the trait blocks the thread it is called on
/// and the stand-in answers on the runtime: under a single-threaded one the
/// call would wait on an answer nothing could serve.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::stand_in::{Asking, Forge, GitHub, Said};

    /// The comment as a share leaves one, cut down to what this module does with
    /// it: it is written once and said everywhere, byte for byte.
    const COMMENT: &str = "[Read this conversation](https://tobico.github.io/shares/#9f1)\n";

    /// A GitHub that takes a comment on `#41` and refuses everything else.
    ///
    /// A stand-in rather than a mock, for [`crate::stand_in`]'s reason: what is
    /// being proved is that the request went out with the right body, to the
    /// right repository, about the right number.
    async fn github() -> Arc<GitHub> {
        GitHub::answering(|asked| match (asked.asking, asked.number) {
            (Asking::Saying, Some(41)) => Said::This(
                r#"{"html_url":"https://github.com/tobico/verkstead/pull/41#issuecomment-1"}"#
                    .to_owned(),
            ),
            _ => Said::Refused(404, "Not Found".to_owned()),
        })
        .await
    }

    fn repo(path: &std::path::Path, name: &str) -> store::Repo {
        store::Repo {
            id: 1,
            path: PathBuf::from(path),
            name: name.to_owned(),
            default_branch: "main".to_owned(),
            author: None,
        }
    }

    fn pull(number: i64, repo: Option<&str>) -> store::PullRequest {
        store::PullRequest {
            number,
            title: "Conversation sharing".to_owned(),
            url: format!("https://github.com/tobico/verkstead/pull/{number}"),
            head: Some("conversation-sharing".to_owned()),
            base: None,
            repo: repo.map(str::to_owned),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_comment_is_said_in_the_repository_the_pull_request_is_in() {
        let dir = tempfile::tempdir().unwrap();
        let where_said = dir.path().join("verkstead");
        std::fs::create_dir(&where_said).unwrap();

        let github = github().await;
        let forge = Forge::standing_in(&github).alone();

        let landings = tokio::task::spawn_blocking({
            let pulls = [(repo(&where_said, "verkstead"), pull(41, None))];

            move || on_each(&*forge, "ghp_token", &pulls, COMMENT)
        })
        .await
        .unwrap();

        assert_eq!(
            landings,
            vec![Landing {
                number: 41,
                repo: None,
                landed: Ok("https://github.com/tobico/verkstead/pull/41#issuecomment-1".to_owned()),
            }],
        );

        // What GitHub was asked to say, and which repository it was asked
        // about: the body is the comment as it was written, and the request
        // names the repository the number belongs to.
        let asked = github.asked_about(Asking::Saying);
        assert_eq!(asked.len(), 1, "one comment, once: {asked:?}");

        let said: serde_json::Value = serde_json::from_str(&asked[0].body).unwrap();

        assert_eq!(said["body"], serde_json::json!(COMMENT));
        assert_eq!(asked[0].path, "/repos/tobico/verkstead/issues/41/comments");
    }

    /// A pull request the comment could not land on is named against the ones
    /// that worked, and the ones after it are still tried: they are separate
    /// repositories with separate permissions, and one that is gone says nothing
    /// about the next.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_pull_request_that_missed_out_is_named_and_the_rest_go_on() {
        let dir = tempfile::tempdir().unwrap();
        let forge = Forge::standing_in(&*github().await).alone();

        let landings = tokio::task::spawn_blocking({
            let pulls = [
                (
                    repo(dir.path(), "verkstead-site"),
                    pull(7, Some("verkstead-site")),
                ),
                (repo(dir.path(), "verkstead"), pull(41, None)),
            ];

            move || on_each(&*forge, "ghp_token", &pulls, COMMENT)
        })
        .await
        .unwrap();

        assert_eq!(landings.len(), 2);
        assert_eq!(landings[0].number, 7);
        assert_eq!(landings[0].repo.as_deref(), Some("verkstead-site"));
        assert!(
            landings[0]
                .landed
                .as_ref()
                .is_err_and(|why| why.contains("Not Found")),
            "in GitHub's own words, not {:?}",
            landings[0].landed,
        );

        assert!(
            landings[1].landed.is_ok(),
            "the one after it is still tried: {:?}",
            landings[1].landed,
        );
    }

    /// Sharing again leaves a second comment rather than rewriting the first,
    /// and nothing here ever asks GitHub to change anything that is already
    /// there: two presses are two comments, and the pull request's own
    /// description is never touched.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sharing_twice_leaves_two_comments_and_edits_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let where_said = dir.path().join("verkstead");
        std::fs::create_dir(&where_said).unwrap();

        let github = github().await;

        for _ in 0..2 {
            let forge = Forge::standing_in(&github).alone();
            let landed = tokio::task::spawn_blocking({
                let pulls = [(repo(&where_said, "verkstead"), pull(41, None))];

                move || on_each(&*forge, "ghp_token", &pulls, COMMENT)
            })
            .await
            .unwrap();

            assert!(landed[0].landed.is_ok(), "{:?}", landed[0].landed);
        }

        let asked: Vec<String> = github
            .asked_about(Asking::Saying)
            .into_iter()
            .map(|asked| asked.path)
            .collect();

        assert_eq!(
            asked,
            vec!["/repos/tobico/verkstead/issues/41/comments".to_owned(); 2],
        );
    }
}
