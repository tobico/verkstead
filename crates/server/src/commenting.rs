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

use crate::github::{self, Gh};
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

    /// Where the comment landed, or what `gh` said about why it did not.
    pub(crate) landed: Result<String, String>,
}

/// Say `body` on every one of them, and answer what became of each.
///
/// In the order they were recorded, which is the Conversation's own repository
/// first and the companions as the wrap-up found them — the order the workbench
/// draws their cards in, so a report of what happened reads down the same list.
///
/// Blocking from end to end: one `gh` per pull request, and a Conversation ends
/// on one per repository it was worked in. The caller is on `spawn_blocking`.
pub(crate) fn on_each(
    gh: &Gh,
    token: &str,
    pulls: &[(store::Repo, store::PullRequest)],
    body: &str,
) -> Vec<Landing> {
    pulls
        .iter()
        .map(|(repo, pull)| Landing {
            number: pull.number,
            repo: pull.repo.clone(),
            landed: github::comment(gh, &repo.path, token, pull.number, body)
                .map_err(|trouble| trouble.why()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The comment as a share leaves one, cut down to what this module does with
    /// it: it is written once and said everywhere, byte for byte.
    const COMMENT: &str = "[Read this conversation](https://tobico.github.io/shares/#9f1)\n";

    /// A `gh` that takes a comment on `#41` and refuses everything else, writing
    /// down what it was given.
    ///
    /// A script rather than a mock, for [`crate::github`]'s reason: what is being
    /// proved is that a process was run with the right arguments in the right
    /// directory and given the right body on its stdin.
    fn gh(notes: &std::path::Path) -> Gh {
        let notes = notes.display();

        Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!(
                r#"printf '%s\n' "$*" >> "{notes}/asked"
                case "$*" in
                    "api -X POST repos/{{owner}}/{{repo}}/issues/41/comments --input -")
                        cat > "{notes}/said.json"
                        pwd > "{notes}/where"
                        printf '{{"html_url":"https://github.com/tobico/verkstead/pull/41#issuecomment-1"}}' ;;
                    *)
                        printf 'gh: Not Found (HTTP 404)\n' >&2; exit 1 ;;
                esac"#,
            ),
            "gh".to_owned(),
        ])
    }

    fn repo(path: &std::path::Path, name: &str) -> store::Repo {
        store::Repo {
            id: 1,
            path: PathBuf::from(path),
            name: name.to_owned(),
            default_branch: "main".to_owned(),
        }
    }

    fn pull(number: i64, repo: Option<&str>) -> store::PullRequest {
        store::PullRequest {
            number,
            title: "Conversation sharing".to_owned(),
            url: format!("https://github.com/tobico/verkstead/pull/{number}"),
            repo: repo.map(str::to_owned),
        }
    }

    #[test]
    fn the_comment_is_said_in_the_repository_the_pull_request_is_in() {
        let dir = tempfile::tempdir().unwrap();
        let where_said = dir.path().join("repo");
        std::fs::create_dir(&where_said).unwrap();

        let landings = on_each(
            &gh(dir.path()),
            "ghp_token",
            &[(repo(&where_said, "verkstead"), pull(41, None))],
            COMMENT,
        );

        assert_eq!(
            landings,
            vec![Landing {
                number: 41,
                repo: None,
                landed: Ok("https://github.com/tobico/verkstead/pull/41#issuecomment-1".to_owned()),
            }],
        );

        // What GitHub was asked to say, and where it was asked from: the body is
        // the comment as it was written, and the `gh` ran inside the repository
        // the number belongs to.
        let said: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("said.json")).unwrap())
                .unwrap();

        assert_eq!(said["body"], serde_json::json!(COMMENT));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("where"))
                .unwrap()
                .trim(),
            where_said.canonicalize().unwrap().display().to_string(),
        );
    }

    /// A pull request the comment could not land on is named against the ones
    /// that worked, and the ones after it are still tried: they are separate
    /// repositories with separate permissions, and one that is gone says nothing
    /// about the next.
    #[test]
    fn a_pull_request_that_missed_out_is_named_and_the_rest_go_on() {
        let dir = tempfile::tempdir().unwrap();

        let landings = on_each(
            &gh(dir.path()),
            "ghp_token",
            &[
                (
                    repo(dir.path(), "verkstead-site"),
                    pull(7, Some("verkstead-site")),
                ),
                (repo(dir.path(), "verkstead"), pull(41, None)),
            ],
            COMMENT,
        );

        assert_eq!(landings.len(), 2);
        assert_eq!(landings[0].number, 7);
        assert_eq!(landings[0].repo.as_deref(), Some("verkstead-site"));
        assert!(
            landings[0]
                .landed
                .as_ref()
                .is_err_and(|why| why.contains("Not Found")),
            "in `gh`'s own words, not {:?}",
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
    #[test]
    fn sharing_twice_leaves_two_comments_and_edits_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let pulls = [(repo(dir.path(), "verkstead"), pull(41, None))];

        for _ in 0..2 {
            assert!(
                on_each(&gh(dir.path()), "ghp_token", &pulls, COMMENT)[0]
                    .landed
                    .is_ok()
            );
        }

        let asked: Vec<String> = std::fs::read_to_string(dir.path().join("asked"))
            .unwrap()
            .lines()
            .map(str::trim)
            .map(str::to_owned)
            .collect();

        assert_eq!(
            asked,
            vec!["api -X POST repos/{owner}/{repo}/issues/41/comments --input -".to_owned(); 2],
        );
    }
}
