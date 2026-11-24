//! What the host still decides about what is said on a wrapping Conversation's
//! pull requests, for the **Wrap-up** Activity that reads it.
//!
//! The comments watcher is the component's — see `plugins/activities/wrap-up`:
//! it reads every pull request's comments through the Forge each round, holds
//! back what the review is to fold in, and dispatches a batch session about
//! what is said after it. What stays here is what only the host can say about
//! a comment: whether it is anybody's to answer at all.
//!
//! **What Verkstead itself said is not.** The comment Share to Pull Request
//! leaves ends with a marker of its own — see [`verkstead_render::SHARE_MARKER`]
//! — and a comment carrying it at the start of a line is dropped wherever the
//! comments are read: the fresh ones a batch session would be dispatched about
//! and the standing ones folded into the review alike. It is posted by the
//! configured token, which is usually the human's own account, so nothing about
//! who said it could tell it from what they write themselves. See
//! [`verkstead_said_it`].
//!
//! **Nor what the human said to ignore.** `config.yaml` holds a list of rules —
//! a regex over who said it, a regex over what it says, or both — and a comment
//! any one of them matches is skipped exactly as the share's own is. See
//! [`ignored`]. The rules are read off the file every time they are asked
//! about, so one added on a phone takes effect on the next round without a
//! restart.
//!
//! The one difference from the share's drop is that **a skipped comment is
//! written down as addressed**. The share's marker is Verkstead's own for as
//! long as that comment exists, so inspection answers for it for ever; a rule is
//! the human's and they may delete it, and a rule deleted after months of a bot
//! nagging would otherwise bring the whole of it back as sessions in one round.
//! Writing each one down as it is skipped is what makes taking a rule away
//! change what happens next rather than what happened.

use crate::AppState;
use crate::forges::Comment;
use crate::store;

/// Which of `said`, in the Repo `repo`, nobody is to be sent to answer — see
/// the module's account of both — writing the ones the rules match down as
/// addressed.
///
/// A recording that failed does not put the comment back. What is dispatched
/// about is decided by the rules as they stand, so a store that would not
/// answer costs this the writing down and nothing else — the next round skips
/// the comment again, and writes it down again.
pub(crate) async fn not_to_answer(
    state: &AppState,
    conversation_id: i64,
    repo: &store::Repo,
    said: &[Comment],
) -> Vec<String> {
    // Read off the file each time rather than held, which is what makes a rule
    // added on a phone take effect on the next round.
    let config = state.settings.config();
    let rules = config.ignored_comments();

    let mut own = Vec::new();
    let mut skipped = Vec::new();

    for comment in said {
        if verkstead_said_it(&comment.markdown) {
            own.push(comment.which.clone());
        } else if ignored(rules, comment) {
            skipped.push(comment.which.clone());
        }
    }

    if !skipped.is_empty() {
        tracing::info!(
            conversation_id,
            repo = repo.name,
            comments = skipped.len(),
            "the ignore rules match what was said, so nobody is being sent to address it",
        );

        if let Err(error) =
            store::record_addressed_comments(&state.pool, conversation_id, repo.id, &skipped).await
        {
            tracing::error!(error = ?error, conversation_id, repo = repo.name, "recording which comments the ignore rules skipped failed");
        }
    }

    own.extend(skipped);
    own
}

/// Whether `comment` is one of the classes the human never wants addressed.
///
/// The rules are theirs rather than Verkstead's — a review service filing the
/// same word about billing on every pull request, say. Any one rule matching is
/// enough, and a rule matches where every field it gives does: the author
/// pattern against the login of whoever said it, the body pattern against the
/// markdown they wrote. See [`crate::settings::IgnoreRule::matches`], which is
/// where the whole of the matching lives.
///
/// An account GitHub no longer has arrives with an empty login, which is matched
/// as the empty string: a rule naming an author does not match it, and a rule
/// about only the body goes on matching what it says.
fn ignored(rules: &[crate::settings::IgnoreRule], comment: &Comment) -> bool {
    rules
        .iter()
        .any(|rule| rule.matches(&comment.author, &comment.markdown))
}

/// Whether this is the comment a share left rather than something somebody wants
/// addressed.
///
/// **At the start of a line**, which is what leaves a reply to the share
/// something to answer: quote-replying on GitHub prefixes every line with `>`,
/// so the marker travels along inside the quote without ever beginning a line,
/// and what the human wrote under it is dispatched for like anything else.
///
/// Dropped by inspection every round rather than written down as addressed.
/// The comment is Verkstead's own for as long as it exists, so there is nothing
/// to remember: a server that came back up reads it as its own again.
fn verkstead_said_it(markdown: &str) -> bool {
    markdown
        .lines()
        .any(|line| line.starts_with(verkstead_render::SHARE_MARKER))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(author: &str, markdown: &str) -> Comment {
        Comment {
            which: format!("IC_{author}_{}", markdown.len()),
            author: author.to_owned(),
            at: "2026-08-21T09:00:00Z".to_owned(),
            about: String::new(),
            markdown: markdown.to_owned(),
        }
    }

    /// The same, left on a line of the diff rather than in the conversation.
    fn on_a_line(about: &str, markdown: &str) -> Comment {
        Comment {
            about: about.to_owned(),
            ..comment("tobico", markdown)
        }
    }

    /// The comment Share to Pull Request leaves is Verkstead's own, so neither
    /// session is ever given it: it is written as the configured token, which is
    /// usually the human's own account, and the marker in the body is the whole
    /// of how it is told from what they write themselves.
    #[test]
    fn the_comment_a_share_left_is_verksteads_own() {
        assert!(verkstead_said_it(&shared()));
    }

    /// And a human quote-replying to it is somebody talking. GitHub prefixes
    /// every line of a quote with `>`, so the marker rides along in the middle of
    /// a line rather than at the start of one — which is why the rule is written
    /// about the start of a line.
    #[test]
    fn a_quote_reply_of_the_share_is_still_somebody_to_answer() {
        let quoted: String = shared()
            .lines()
            .map(|line| format!("> {line}\n"))
            .collect::<String>()
            + "\nWhich of these is the one to keep?\n";

        assert!(!verkstead_said_it(&quoted), "{quoted}");
    }

    /// Nothing else is. A comment that happens to say what the marker is — this
    /// one — is a human writing about it rather than a share leaving one.
    #[test]
    fn a_comment_that_only_mentions_the_marker_is_addressed_like_any_other() {
        assert!(!verkstead_said_it(&format!(
            "The share writes {} at the end. Is that deliberate?",
            verkstead_render::SHARE_MARKER,
        )));
        assert!(!verkstead_said_it("Rename the `window` field."));
    }

    /// The share comment as it is left: the link, the itemization, and the
    /// marker on a line of its own at the end.
    fn shared() -> String {
        format!(
            "[Read this conversation](https://x/#9f1) — a read-only copy of \
             `rate-limiting`, taken 2026-08-30.\n\nA limiter that counts across \
             instances.\n\n{}\n",
            verkstead_render::SHARE_MARKER,
        )
    }

    /// A rule the human wrote is the other thing neither session is given, and
    /// what it is for: a review service filing the same word about billing on
    /// every pull request, where the alternative is a session spun up to address
    /// it each time.
    #[test]
    fn a_rule_the_human_wrote_is_what_it_says_it_is() {
        let rules = [rule(Some("coderabbitai"), Some("billing"))];

        assert!(ignored(
            &rules,
            &comment("coderabbitai", "Your billing information is missing."),
        ));
        assert!(!ignored(
            &rules,
            &comment("tobico", "Sort the billing out one of these days."),
        ));
        assert!(!ignored(
            &rules,
            &comment("coderabbitai", "This loop reads the vector twice."),
        ));
    }

    /// A rule giving one field constrains that one alone: an author with no body
    /// ignores everything that account writes, and a body with no author ignores
    /// that phrase from anybody.
    #[test]
    fn a_rule_giving_one_field_constrains_that_one_alone() {
        assert!(ignored(
            &[rule(Some("coderabbitai"), None)],
            &comment("coderabbitai", "This loop reads the vector twice."),
        ));
        assert!(ignored(
            &[rule(None, Some("billing"))],
            &comment("tobico", "Sort the billing out one of these days."),
        ));
    }

    /// And the rules combine with OR: a comment is ignored where any one of them
    /// matches, which is what makes the list a list rather than a rule with more
    /// fields.
    #[test]
    fn any_one_rule_matching_is_enough() {
        let rules = [
            rule(Some("dependabot"), None),
            rule(None, Some("(?i)billing")),
        ];

        assert!(ignored(
            &rules,
            &comment("dependabot", "Bump serde to 1.0.")
        ));
        assert!(ignored(
            &rules,
            &comment("coderabbitai", "Billing information is missing."),
        ));
        assert!(!ignored(
            &rules,
            &comment("tobico", "Rename the `window` field."),
        ));
    }

    /// A comment left on a line of the diff is matched the same way as one said
    /// in the conversation. What a rule reads is who said it and what they
    /// wrote, and both are the same comment whichever of the three places it
    /// was left in — see [`crate::forges::Forge::comments`].
    #[test]
    fn a_comment_on_the_diff_is_matched_like_any_other() {
        assert!(ignored(
            &[rule(None, Some("nit:"))],
            &on_a_line("`src/window.rs` line 12", "nit: this reads oddly."),
        ));
    }

    /// A settings file nobody has written a rule into ignores nothing, which is
    /// every comment on every pull request being somebody's to address.
    #[test]
    fn no_rules_ignore_nothing() {
        assert!(!ignored(
            &[],
            &comment("coderabbitai", "Billing is missing.")
        ));
    }

    /// An account GitHub no longer has arrives with an empty login. A rule
    /// naming an author does not match it — the empty string is not
    /// `coderabbitai` — and a rule about what was said goes on matching it.
    #[test]
    fn a_comment_by_nobody_is_matched_on_what_it_says() {
        assert!(!ignored(
            &[rule(Some("coderabbitai"), None)],
            &comment("", "Your billing information is missing."),
        ));
        assert!(ignored(
            &[rule(None, Some("billing"))],
            &comment("", "Your billing information is missing."),
        ));
    }

    /// One rule as the settings page would have written it down.
    fn rule(author: Option<&str>, body: Option<&str>) -> crate::settings::IgnoreRule {
        crate::settings::IgnoreRule::of(author.map(str::to_owned), body.map(str::to_owned))
    }
}
