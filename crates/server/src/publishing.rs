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

use crate::github::{self, Gh, Trouble};
use crate::settings::GitAuthor;

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
    gh: &Gh,
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
    let account = match github::authenticates_as(gh, token) {
        Ok(account) => account,
        Err(trouble) => return Publishing::Refused(trouble.why()),
    };

    if account.scopes.known_to_lack(github::GIST) {
        return Publishing::NoGistScope;
    }

    let gist = match github::create_gist(gh, token, description, name, PLACEHOLDER) {
        Ok(gist) => gist,
        Err(trouble) => return refusal(trouble),
    };

    match written(&gist, token, author, name, document) {
        Ok(()) => Publishing::At(gist.url),
        Err(why) => {
            // The gist outlives the failure otherwise, holding the placeholder
            // and standing at a URL. Whether the deleting worked is not this
            // failure's business — what is reported is what went wrong with the
            // publish, and a delete that also failed is logged where the rest of
            // the trouble is.
            if let Err(trouble) = github::delete_gist(gh, token, &gist.id) {
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
    gist: &github::Gist,
    token: &str,
    author: &GitAuthor,
    name: &str,
    document: &str,
) -> Result<(), String> {
    // The directory outlives the borrow below and is removed when it goes: a
    // publish leaves nothing on the disk it was made on either.
    let scratch =
        tempfile::tempdir().map_err(|error| format!("git had nowhere to work: {error}"))?;
    let clone = scratch.path();

    git(clone, token, &["clone", "--quiet", &gist.push, "."])?;

    std::fs::write(clone.join(name), document)
        .map_err(|error| format!("the share could not be written for git: {error}"))?;

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
/// The credential helper is declared on every call rather than only on the two
/// that talk to the network, because which of them does is git's business: a
/// clone may fetch, a push may re-fetch, and a helper that is there and unasked
/// costs nothing.
///
/// **The token goes through the environment.** The helper is a shell function
/// that reads `GH_TOKEN`, so what appears in the process list is the shape of
/// the helper and never the credential — which is exactly what putting the token
/// in the remote URL would do instead. The empty `credential.helper=` in front
/// of it clears whatever the machine has configured, so that a publish
/// authenticates as the token it was given and never as the host's stored login.
fn git(dir: &Path, token: &str, args: &[&str]) -> Result<(), String> {
    const HELPER: &str = r#"!f() { if test "$1" = get; then printf 'username=x-access-token\npassword=%s\n' "$GH_TOKEN"; fi; }; f"#;

    let output = Command::new("git")
        .args(["-c", "credential.helper=", "-c"])
        .arg(format!("credential.helper={HELPER}"))
        .args(args)
        .current_dir(dir)
        .env("GH_TOKEN", token)
        // Nothing here may stop to ask: a terminal prompt from a credential
        // helper that found nothing would be a server thread waiting for ever.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("git could not be run: {error}"))?;

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
fn said(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git said nothing about why")
        .to_owned()
}

/// A `gh` refusal, as a publish reports it.
///
/// The one worth telling apart is a token GitHub itself turned down for the
/// scope, which is what a *classic* token missing `gist` never reaches — its
/// scopes are read before anything is made — and what a fine-grained one whose
/// permissions were never checked lands on. Same answer either way, because it
/// is the same thing for the human to go and do.
fn refusal(trouble: Trouble) -> Publishing {
    match &trouble {
        Trouble::Refused(said) if unscoped(said) => Publishing::NoGistScope,
        _ => Publishing::Refused(trouble.why()),
    }
}

/// Whether what `gh` said is GitHub refusing the token rather than the request.
///
/// Two shapes, because GitHub has two ways of saying it. A fine-grained token
/// without the permission is told so — *not accessible by* — and a classic one
/// gets a `404`, which is GitHub's habit everywhere: telling somebody a resource
/// exists but is not theirs is telling them it exists.
///
/// The `404` is safe to read that way *here* and would not be anywhere else:
/// what was asked for is `POST /gists`, which is a collection rather than a
/// thing, and there is nothing about it for GitHub to have lost. Everything else
/// stays whatever it was.
fn unscoped(said: &str) -> bool {
    let said = said.to_lowercase();

    said.contains("not accessible by")
        || said.contains("requires authentication")
        || said.contains("not found")
}

/// The same shell-script `gh` [`crate::github`]'s own tests run, and asked on
/// the same platforms — and a git remote made on the local filesystem beside
/// it, which is what lets a publish be proved without a network.
#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The document a share is, standing in for the several megabytes a real one
    /// runs to. What matters about it here is that it arrives byte for byte, so
    /// it carries the things that do not survive being handled carelessly: a
    /// closing tag, a line that is not the last, and a character outside ASCII.
    const SHARE: &str = "<!doctype html>\n<script>ok()</script>\n<p>Aperçu</p>\n";

    /// A `gh` that answers the three calls a publish makes, and writes down
    /// which of them it was asked for.
    ///
    /// The gist it makes is a bare repository the test made, which is the whole
    /// reason this can be proved without a network: a gist is a git remote, and
    /// what a publish does with one is clone it, write the file and push. The
    /// scopes it answers with are whatever token it was run with, so a test
    /// hands a scope list where a token goes.
    fn gh(remote: &std::path::Path, notes: &std::path::Path) -> Gh {
        let remote = remote.display();
        let notes = notes.display();

        Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!(
                r#"case "$*" in
                    "api -i user")
                        printf 'HTTP/2.0 200 OK\r\nX-Oauth-Scopes: %s\r\n\r\n{{"login":"tobico"}}' "${{GH_TOKEN-unset}}" ;;
                    "api -X POST /gists --input -")
                        cat > "{notes}/created.json"
                        printf '{{"id":"9f1","html_url":"https://gist.github.com/tobico/9f1","git_push_url":"{remote}"}}' ;;
                    "api -X DELETE /gists/9f1")
                        : > "{notes}/deleted" ;;
                    *)
                        printf 'gh: no such command: %s\n' "$*" >&2; exit 1 ;;
                esac"#,
            ),
            "gh".to_owned(),
        ])
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

    #[test]
    fn a_publish_puts_the_share_in_the_gist_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let gh = gh(&remote, dir.path());

        assert_eq!(
            publish(
                &gh,
                Some("repo, gist"),
                &author(),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            ),
            Publishing::At("https://gist.github.com/tobico/9f1".to_owned()),
        );

        assert_eq!(holding(&remote, dir.path(), "share.html"), SHARE);
    }

    /// The gist is made secret and named for the Conversation, which is the half
    /// of a publish the API does — the file it is created with is a placeholder,
    /// the bytes above having arrived over git.
    #[test]
    fn the_gist_it_makes_is_secret_and_says_what_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let gh = gh(&remote, dir.path());

        publish(
            &gh,
            Some("repo, gist"),
            &author(),
            "A Verkstead conversation: sharing",
            "share.html",
            SHARE,
        );

        let asked: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("created.json")).unwrap(),
        )
        .unwrap();

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
    #[test]
    fn a_token_without_the_gist_scope_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let gh = gh(&remote, dir.path());

        assert_eq!(
            publish(
                &gh,
                Some("read:org, repo, workflow"),
                &author(),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            ),
            Publishing::NoGistScope,
        );

        assert!(!dir.path().join("created.json").exists());
    }

    /// And nothing configured is a refusal of its own rather than a publish made
    /// as whoever the host's `gh` is logged in as. This is the one place reading
    /// and writing differ: a read falls back, and a write must not.
    #[test]
    fn a_verkstead_with_no_token_publishes_as_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let gh = gh(&remote, dir.path());

        assert_eq!(
            publish(
                &gh,
                None,
                &author(),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            ),
            Publishing::NoToken,
        );

        assert!(!dir.path().join("created.json").exists());
    }

    /// A publish that falls over after the gist is made takes it back, so that a
    /// failure leaves no link standing over a placeholder.
    #[test]
    fn a_gist_the_share_never_reached_is_taken_back() {
        let dir = tempfile::tempdir().unwrap();
        let gh = gh(&dir.path().join("no-such-gist.git"), dir.path());

        let published = publish(
            &gh,
            Some("repo, gist"),
            &author(),
            "A Verkstead conversation: sharing",
            "share.html",
            SHARE,
        );

        assert!(
            matches!(published, Publishing::Refused(_)),
            "a clone that could not happen is a refusal, not {published:?}",
        );

        assert!(
            dir.path().join("deleted").exists(),
            "the gist the share never reached is still standing",
        );
    }

    /// A fine-grained token has permissions rather than scopes, so nothing can be
    /// read off it before the attempt — GitHub refuses the creation instead, and
    /// that refusal is the same thing for the human to go and do as a classic
    /// token's missing scope.
    #[test]
    fn a_token_github_itself_refuses_the_gist_to_is_the_same_answer() {
        let dir = tempfile::tempdir().unwrap();
        let gh = Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!(
                r#"case "$*" in
                    "api -i user")
                        printf 'HTTP/2.0 200 OK\r\n\r\n{{"login":"tobico"}}' ;;
                    *)
                        printf 'gh: Not Found (HTTP 404)\n' >&2; exit 1 ;;
                esac"#,
            ),
            "gh".to_owned(),
        ]);

        assert_eq!(
            publish(
                &gh,
                Some("github_pat_anything"),
                &author(),
                "A Verkstead conversation: sharing",
                "share.html",
                SHARE,
            ),
            Publishing::NoGistScope,
        );

        assert!(!dir.path().join("created.json").exists());
    }

    /// The commit in the gist is by the author the settings page was told,
    /// rather than by whoever the machine's git config says.
    #[test]
    fn the_commit_is_by_the_configured_author() {
        let dir = tempfile::tempdir().unwrap();
        let remote = gist(dir.path());
        let gh = gh(&remote, dir.path());

        publish(
            &gh,
            Some("repo, gist"),
            &author(),
            "A Verkstead conversation: sharing",
            "share.html",
            SHARE,
        );

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
