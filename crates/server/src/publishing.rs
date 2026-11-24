//! Publishing a share: the file put somewhere a link reaches.
//!
//! A share downloads as one self-contained document — see [`crate::sharing`] —
//! and that is the whole of what a colleague needs, provided somebody emails it
//! to them. Publishing is the other way to hand it over: the same bytes, in a
//! **secret gist**, so that what goes on a pull request is a link.
//!
//! This is Verkstead's first write to GitHub of its own. Everything before it
//! reads — which pull request a branch has, what its checks say, what was
//! commented on it — and a read that falls back to whatever login the host's
//! `gh` happens to have is a question asked twice. A *write* under that login
//! would be a file in an account nobody chose, so a publish with no token
//! configured refuses instead of falling back. See [`Publishing::NoToken`].
//!
//! ## Two halves, because the API will not take the file
//!
//! The Gists API's cap on what a gist may be **created** with is undocumented,
//! and has been reported at a megabyte since 2015 — GitHub documents the
//! megabyte on the way *out* (a file read back through the API comes back
//! `truncated`) and says nothing about the way in. A share is several megabytes:
//! the viewer inlined, every diff whole, and the diagram renderer where anything
//! was drawn.
//!
//! So the gist is created with a placeholder and the bytes arrive over **git**,
//! which has no such cap and is what GitHub's own documentation points at for a
//! gist file too large to fetch. One publish is therefore: create, clone, write,
//! commit, push — and, if any of that fails after the gist exists, delete it, so
//! that a publish that did not happen leaves nothing behind that looks as though
//! it did.
//!
//! ```text
//!   gh api POST /gists  ->  git clone  ->  the share written  ->  git push
//!        (placeholder)                                              |
//!        gh api DELETE /gists/{id}  <-- anything above failed  <----+
//! ```
//!
//! The token reaches git the way it reaches `gh`: in the environment, read by a
//! credential helper written on the command line. Never in the remote URL — a
//! URL is an argument, and an argument is on every process list on the machine.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::forges::{Credential, Trouble};
use crate::settings::{Author, GitAuthor, Settings};
use crate::unseen::Unseen;

/// What became of a publish, in the server's own words — see
/// [`verkstead_render::SharePublished`], which is this said to the workbench.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Publishing {
    /// It is up, at this page.
    At(String),

    /// Nothing is configured to publish as.
    NoToken,

    /// The token is somebody's, and GitHub says gists are not among what it may
    /// do.
    NoGistScope,

    /// Anything else, in `gh`'s or git's own words.
    Refused(String),
}

/// Put `document` up as a secret gist called `name`, and answer where it went.
///
/// `description` is what the gist is called on GitHub's own pages. It is
/// readable by whoever holds the link, which is the same audience as the file,
/// so it says what the file is rather than hiding it.
///
/// Blocking from end to end — a `gh`, a clone and a push. The caller is on
/// `spawn_blocking`.
pub(crate) fn publish(
    gh: &dyn crate::forges::Forge,
    token: Option<&str>,
    author: &GitAuthor,
    description: &str,
    name: &str,
    document: &str,
) -> Publishing {
    let Some(token) = token else {
        return Publishing::NoToken;
    };

    // Asked before anything is made, so that a token that cannot publish is a
    // named refusal rather than a gist half made and taken back. The same call
    // the settings page verifies with, which is why the answer is the account's
    // scopes rather than a yes or a no.
    let account = match gh.verify(token) {
        Ok(account) => account,
        Err(trouble) => return Publishing::Refused(trouble.why()),
    };

    if account.scopes.known_to_lack(crate::forges::GIST) {
        return Publishing::NoGistScope;
    }

    let gist = match gh.create_gist(token, description, name, PLACEHOLDER) {
        Ok(gist) => gist,
        Err(trouble) => return refusal(gh, trouble),
    };

    match written(&gist, token, author, name, document) {
        Ok(()) => Publishing::At(gist.url),
        Err(why) => {
            // The gist outlives the failure otherwise, holding the placeholder
            // and standing at a URL. Whether the deleting worked is not this
            // failure's business — what is reported is what went wrong with the
            // publish, and a delete that also failed is logged where the rest of
            // the trouble is.
            if let Err(trouble) = gh.delete_gist(token, &gist.id) {
                tracing::error!(
                    error = ?trouble,
                    gist = gist.id,
                    "taking back the gist of a share that could not be published failed",
                );
            }

            Publishing::Refused(why)
        }
    }
}

/// [`publish`] as the Author a Conversation's Repo recorded as `recorded` —
/// see [`crate::settings::Config::author_of`] — and answer the token it was
/// published as beside what became of it.
///
/// Its token for the gist, and its name on the gist's commit. Both files are
/// read here, at the moment of the publish, for the reason every other reader
/// of them reads them late. The token comes back because the comments that
/// follow a publish are left as whoever the gist belongs to.
pub(crate) fn publish_as(
    gh: &dyn crate::forges::Forge,
    settings: &Settings,
    recorded: Option<&str>,
    description: &str,
    name: &str,
    document: &str,
) -> (Publishing, Option<String>) {
    let secrets = settings.secrets();
    let config = settings.config();

    let author = config.author_of(recorded);

    // An Author whose token's copy has not arrived is refused by that name
    // rather than as an Author with no token, which it is not.
    if author.is_some_and(|author| config.token_awaited(author.id())) {
        return (Publishing::Refused(crate::forges::awaited()), None);
    }

    let token = author.and_then(|author| config.token_of(author, &secrets));

    let published = publish(
        gh,
        token,
        &author.map(Author::git_author).unwrap_or_default(),
        description,
        name,
        document,
    );

    (published, token.map(str::to_owned))
}

/// What the gist holds between being made and being pushed to, which is a few
/// seconds at most.
///
/// Something rather than nothing, because the API will not create a gist with an
/// empty file — and a sentence rather than a space, because the one reader it
/// could ever have is somebody who caught a publish mid-flight.
const PLACEHOLDER: &str = "The share is on its way.\n";

/// The git half: the gist cloned, the file written into it, and the lot pushed
/// back.
///
/// Cloned rather than force-pushed onto. A clone lands on whatever branch the
/// gist's own HEAD names, and pushing that back is a commit on top of the
/// placeholder's; a push to a branch guessed at here would be a gist whose page
/// went on showing the placeholder because the branch it displays was the other
/// one.
fn written(
    gist: &crate::forges::Gist,
    token: &str,
    author: &GitAuthor,
    name: &str,
    document: &str,
) -> Result<(), String> {
    // The directory outlives the borrow below and is removed when it goes: a
    // publish leaves nothing on the disk it was made on either.
    let scratch =
        tempfile::tempdir().map_err(|error| format!("Git had nowhere to work: {error}."))?;
    let clone = scratch.path();

    git(clone, token, &["clone", "--quiet", &gist.push, "."])?;

    std::fs::write(clone.join(name), document)
        .map_err(|error| format!("The share couldn't be written for git: {error}."))?;

    git(clone, token, &["add", "--", name])?;
    git(
        clone,
        token,
        &[
            // Said on the command line rather than written into the clone's
            // config, because it is the same fact either way and one of them
            // leaves a file behind. Verkstead's own where nobody has been named:
            // git refuses to commit without an identity, and a publish that
            // failed because the settings page had never been filled in would be
            // a refusal about the wrong thing.
            "-c",
            &format!("user.name={}", author.name().unwrap_or("Verkstead")),
            "-c",
            &format!(
                "user.email={}",
                author.email().unwrap_or("verkstead@localhost")
            ),
            "commit",
            "--quiet",
            "--message",
            &format!("Share {name}"),
        ],
    )?;

    git(clone, token, &["push", "--quiet", "origin", "HEAD"])?;

    Ok(())
}

/// One git command in `dir`, authenticating as `token` where it reaches GitHub.
///
/// Through the helper every git of Verkstead's own on a Repo goes through —
/// see [`Credential::on`], which is where the token reaches git, and where it
/// never reaches the process list.
fn git(dir: &Path, token: &str, args: &[&str]) -> Result<(), String> {
    let output = Credential::token(token)
        .on(&mut Command::new("git"))
        .args(args)
        .unseen()
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Git couldn't be run: {error}."))?;

    if output.status.success() {
        return Ok(());
    }

    Err(said(&String::from_utf8_lossy(&output.stderr)))
}

/// What git said, as one line for the human.
///
/// The first line it wrote that says anything, which is where git puts the
/// reason — everything after it is the advice, and a refusal on a page is not
/// the place for a paragraph of it.
///
/// Shared with [`crate::repos`], which reads a git that would not make a
/// repository the same way: one sentence about what went wrong, for a modal
/// that has one line to say it on.
pub(crate) fn said(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Git said nothing about why.")
        .to_owned()
}

/// A refusal to make a gist, as a publish reports it.
///
/// The one worth telling apart is a token the Forge itself turned down for the
/// scope, which is what a *classic* token missing `gist` never reaches — its
/// scopes are read before anything is made — and what a fine-grained one whose
/// permissions were never checked lands on. Same answer either way, because it
/// is the same thing for the human to go and do. Which refusal that is, is the
/// Forge's to say — see [`crate::forges::Forge::lacks_scope`].
fn refusal(forge: &dyn crate::forges::Forge, trouble: Trouble) -> Publishing {
    if forge.lacks_scope(&trouble) {
        return Publishing::NoGistScope;
    }

    Publishing::Refused(trouble.why())
}

/// Asked of GitHub's own component with a stand-in behind it — see
/// [`crate::stand_in`] — and a git remote made on the local filesystem beside
/// it, which is what lets a publish be proved without a network.
///
/// Every test here runs on a multi-thread runtime: the trait blocks the thread
/// it is called on and the stand-in answers on the runtime, so a publish is
/// made from `spawn_blocking` and the answer waited for.
#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::stand_in::{Asked, Asking, Forge, GitHub, Said};

    /// The document a share is, standing in for the several megabytes a real one
    /// runs to. What matters about it here is that it arrives byte for byte, so
    /// it carries the things that do not survive being handled carelessly: a
    /// closing tag, a line that is not the last, and a character outside ASCII.
    const SHARE: &str = "<!doctype html>\n<script>ok()</script>\n<p>Aperçu</p>\n";

    /// A GitHub that answers the three calls a publish makes.
    ///
    /// The gist it makes is a bare repository the test made, which is the whole
    /// reason this can be proved without a network: a gist is a git remote, and
    /// what a publish does with one is clone it, write the file and push. The
    /// scopes it answers with are whatever token it was asked with, so a test
    /// hands a scope list where a token goes.
    async fn github(remote: &std::path::Path) -> Arc<GitHub> {
        let remote = remote.display().to_string();

        GitHub::answering(move |asked: &Asked| match asked.asking {
            Asking::Verifying => Said::Account {
                login: "tobico".to_owned(),
                scopes: asked.token.clone(),
            },
            Asking::Publishing => Said::This(
                serde_json::json!({
                    "id": "9f1",
                    "html_url": "https://gist.github.com/tobico/9f1",
                    "git_push_url": remote,
                })
                .to_string(),
            ),
            Asking::Unpublishing => Said::This("{}".to_owned()),
            _ => Said::Refused(404, "Not Found".to_owned()),
        })
        .await
    }

    /// Publish, off the runtime's threads the way every caller does.
    async fn published(
        github: &Arc<GitHub>,
        token: Option<&str>,
        description: &str,
        name: &str,
        document: &str,
    ) -> Publishing {
        let forge = Forge::standing_in(github).alone();
        let (token, description) = (token.map(str::to_owned), description.to_owned());
        let (name, document) = (name.to_owned(), document.to_owned());

        tokio::task::spawn_blocking(move || {
            publish(
                &*forge,
                token.as_deref(),
                &author(),
                &description,
                &name,
                &document,
            )
        })
        .await
        .unwrap()
    }

    /// A bare repository with one commit on it, standing for the gist the API
    /// has just made — which is a repository holding the placeholder.
    fn gist(dir: &std::path::Path) -> PathBuf {
        let remote = dir.join("gist.git");
        run(dir, &["init", "--quiet", "--bare", "gist.git"]);

        let seeding = dir.join("seed");
        std::fs::create_dir(&seeding).unwrap();
        run(&seeding, &["init", "--quiet"]);
        std::fs::write(seeding.join("share.html"), PLACEHOLDER).unwrap();
        run(&seeding, &["add", "--", "share.html"]);
        run(
            &seeding,
            &[
                "-c",
                "user.name=GitHub",
                "-c",
                "user.email=noreply@github.com",
                "commit",
                "--quiet",
                "--message",
                "Initial gist commit",
            ],
        );
        run(
            &seeding,
            &["push", "--quiet", &remote.display().to_string(), "HEAD"],
        );

        remote
    }

    /// What the gist is holding now, read back the way a reader would: a fresh
    /// clone of the remote the publish pushed to.
    fn holding(remote: &std::path::Path, dir: &std::path::Path, name: &str) -> String {
        let read = dir.join("read");
        run(
            dir,
            &["clone", "--quiet", &remote.display().to_string(), "read"],
        );

        std::fs::read_to_string(read.join(name)).unwrap()
    }

    fn run(dir: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }

    fn author() -> GitAuthor {
        GitAuthor::of(
            Some("Tobias Cohen".to_owned()),
            Some("tobi@tobico.net".to_owned()),
        )
    }

    /// What the gist was asked to be made as, read off what the stand-in was
    /// asked.
    fn created(github: &Arc<GitHub>) -> Option<serde_json::Value> {
        github
            .asked_about(Asking::Publishing)
            .first()
            .and_then(|asked| serde_json::from_str(&asked.body).ok())
    }

    /// A share is published as its Conversation's Repo's Author: every call of
    /// the publish is made with that Author's token, the gist's commit is by its
    /// name, and the token handed back for the comments is the same one.
    ///
    /// The tokens are scope lists, for [`github`]'s reason, each with the gist
    /// scope and a word of its own to tell them apart by.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_share_is_published_as_the_repos_own_author() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let github = github(&remote).await;

        let data_dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(data_dir.path());

        std::fs::write(
            settings.config_path(),
            concat!(
                "authors:\n",
                "  - id: work\n",
                "    name: Tobias Cohen\n",
                "    email: tobi@tobico.net\n",
                "  - id: home\n",
                "    name: Tobi at Home\n",
                "    email: tobi@example.com\n",
            ),
        )
        .unwrap();
        std::fs::write(
            settings.secrets_path(),
            "author_tokens:\n  work: 'repo, gist, work'\n  home: 'repo, gist, home'\n",
        )
        .unwrap();

        let (published, token) = {
            let forge = Forge::standing_in(&github).alone();

            tokio::task::spawn_blocking(move || {
                publish_as(
                    &*forge,
                    &settings,
                    Some("home"),
                    "A Verkstead conversation: sharing",
                    "share.html",
                    SHARE,
                )
            })
            .await
            .unwrap()
        };

        assert_eq!(
            published,
            Publishing::At("https://gist.github.com/tobico/9f1".to_owned()),
        );
        assert_eq!(
            token.as_deref(),
            Some("repo, gist, home"),
            "what the comments are left as"
        );

        let tokens: Vec<Option<String>> = github
            .asked()
            .into_iter()
            .map(|asked| asked.token)
            .collect();

        assert!(
            tokens.len() >= 2
                && tokens
                    .iter()
                    .all(|token| token.as_deref() == Some("repo, gist, home")),
            "every call of the publish was made as the Repo's Author: {tokens:?}"
        );

        let read = dir.path().join("read");
        holding(&remote, dir.path(), "share.html");
        let output = Command::new("git")
            .args(["log", "-1", "--format=%an <%ae>"])
            .current_dir(&read)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "Tobi at Home <tobi@example.com>",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_publish_puts_the_share_in_the_gist_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let github = github(&remote).await;

        assert_eq!(
            published(
                &github,
                Some("repo, gist"),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            )
            .await,
            Publishing::At("https://gist.github.com/tobico/9f1".to_owned()),
        );

        assert_eq!(holding(&remote, dir.path(), "share.html"), SHARE);
    }

    /// The gist is made secret and named for the Conversation, which is the half
    /// of a publish the API does — the file it is created with is a placeholder,
    /// the bytes above having arrived over git.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_gist_it_makes_is_secret_and_says_what_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let github = github(&remote).await;

        published(
            &github,
            Some("repo, gist"),
            "A Verkstead conversation: sharing",
            "share.html",
            SHARE,
        )
        .await;

        let asked = created(&github).expect("the gist was asked for");

        assert_eq!(asked["public"], serde_json::json!(false));
        assert_eq!(
            asked["description"],
            serde_json::json!("A Verkstead conversation: sharing"),
        );
        assert_eq!(
            asked["files"]["share.html"]["content"],
            serde_json::json!(PLACEHOLDER),
        );
    }

    /// A token GitHub says cannot write gists is a named refusal, and nothing is
    /// made: the scopes are asked about before the API is asked for anything, so
    /// a token that cannot publish leaves no gist behind to be taken back.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_token_without_the_gist_scope_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let github = github(&remote).await;

        assert_eq!(
            published(
                &github,
                Some("read:org, repo, workflow"),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            )
            .await,
            Publishing::NoGistScope,
        );

        assert_eq!(created(&github), None);
    }

    /// And nothing configured is a refusal of its own rather than a publish made
    /// as nobody in particular. This is the one place reading and writing
    /// differ: a read answers that there is nobody to ask as, and a write is
    /// refused before anything is asked.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_verkstead_with_no_token_publishes_as_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let github = github(&remote).await;

        assert_eq!(
            published(
                &github,
                None,
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            )
            .await,
            Publishing::NoToken,
        );

        assert!(github.asked().is_empty());
    }

    /// A publish that falls over after the gist is made takes it back, so that a
    /// failure leaves no link standing over a placeholder.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_gist_the_share_never_reached_is_taken_back() {
        let dir = tempfile::tempdir().unwrap();
        let github = github(&dir.path().join("no-such-gist.git")).await;

        let published = published(
            &github,
            Some("repo, gist"),
            "A Verkstead conversation: sharing",
            "share.html",
            SHARE,
        )
        .await;

        assert!(
            matches!(published, Publishing::Refused(_)),
            "a clone that could not happen is a refusal, not {published:?}",
        );

        assert_eq!(
            github.asked_about(Asking::Unpublishing).len(),
            1,
            "the gist the share never reached is still standing",
        );
    }

    /// A fine-grained token has permissions rather than scopes, so nothing can be
    /// read off it before the attempt — GitHub refuses the creation instead, and
    /// that refusal is the same thing for the human to go and do as a classic
    /// token's missing scope.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_token_github_itself_refuses_the_gist_to_is_the_same_answer() {
        let github = GitHub::answering(|asked: &Asked| match asked.asking {
            Asking::Verifying => Said::Account {
                login: "tobico".to_owned(),
                // A fine-grained token: GitHub leaves the header off.
                scopes: None,
            },
            _ => Said::Refused(404, "Not Found".to_owned()),
        })
        .await;

        assert_eq!(
            published(
                &github,
                Some("github_pat_anything"),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            )
            .await,
            Publishing::NoGistScope,
        );
    }

    /// The commit in the gist is by the author the settings page was told,
    /// rather than by whoever the machine's git config says.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_commit_is_by_the_configured_author() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let github = github(&remote).await;

        published(
            &github,
            Some("repo, gist"),
            "A Verkstead conversation: sharing",
            "share.html",
            SHARE,
        )
        .await;

        let said = Command::new("git")
            .args(["log", "-1", "--format=%an <%ae>"])
            .current_dir(&remote)
            .output()
            .unwrap();

        assert_eq!(
            String::from_utf8_lossy(&said.stdout).trim(),
            "Tobias Cohen <tobi@tobico.net>",
        );
    }
}
