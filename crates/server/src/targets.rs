//! What a **Review** Conversation is to take up: the **Target** field read at
//! the press, and the Brief read to fill that field.
//!
//! A pull request is the one target that can be read out of prose: a
//! `github.com/<owner>/<repo>/pull/<n>` URL and a bare `#<n>` are both
//! unambiguous wherever they fall in a sentence, so the human writes about the
//! work and the target comes along with it. Nothing else is read out of prose —
//! a bare branch name is a word like any other, and goes in the field by hand.
//!
//! **Two readings of the one pair of shapes**, and they are not the same
//! question. [`pull_request_in`] scans prose for the first name in it, which is
//! what a saved Brief fills an empty Target with; [`pull_request_named`] asks
//! what a Target *is*, which is the whole of the field or nothing — a field
//! reading `fix #41 first` is a branch nobody has, rather than a pull request
//! with words around it, because what the human typed there is the name of one
//! thing.
//!
//! **The first of either, wherever it falls**, which is why one expression
//! covers both alternatives rather than two being scanned and compared: a URL
//! carrying its own fragment — `…/pull/41#issuecomment-9` — is one name rather
//! than two, and a single left-to-right scan is what says so.
//!
//! What is read here is a *number*, and a repository where the name said one.
//! Everything about the pull request itself — its title, its head branch, the
//! branch it merges into, whether its head is in a fork — is GitHub's, and is
//! asked of the configured `gh` at the press. See [`crate::github`].

use std::sync::LazyLock;

use regex::Regex;

/// A pull request a Brief names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedPullRequest {
    pub(crate) number: i64,

    /// The `owner/repo` a URL said it is in, or `None` where the name was a
    /// bare `#<n>`.
    ///
    /// A number by itself names a pull request of whichever repository it is
    /// read in, which is the Conversation's Repo; a URL names one outright, and
    /// a URL naming a repository this Repo's origin is not is refused rather
    /// than taken up against the wrong GitHub — see
    /// [`crate::conversations::take_up`].
    pub(crate) repository: Option<String>,
}

/// The first pull request `brief` names, or `None` where it names none.
///
/// **The `#` has to start a word.** One preceded by a letter, a digit or a
/// slash is not a bare number: `owner/repo#41` is GitHub's own way of naming a
/// pull request in *another* repository, and a URL's fragment is a `#` after
/// whatever the path ended in. Neither is something this Repo's origin can be
/// asked about, so neither is read as a target at all.
///
/// A `#` that opens a markdown heading is no target either, and falls out of
/// the same rule the other way: a heading's `#` is followed by a space rather
/// than by digits.
pub(crate) fn pull_request_in(brief: &str) -> Option<NamedPullRequest> {
    let found = NAMED.captures(brief)?;

    if let (Some(owner), Some(repo), Some(number)) = (
        found.name("owner"),
        found.name("repo"),
        found.name("number"),
    ) {
        return Some(NamedPullRequest {
            number: number.as_str().parse().ok()?,
            repository: Some(format!("{}/{}", owner.as_str(), repo.as_str())),
        });
    }

    Some(NamedPullRequest {
        number: found.name("hash")?.as_str().parse().ok()?,
        repository: None,
    })
}

/// And the pull request a **Target** field names, or `None` where what it holds
/// is a branch.
///
/// Anchored, where [`pull_request_in`] scans: the field holds the name of one
/// thing, so what is in it is a pull request only if the whole of it is one.
/// Anything else is a branch — including prose, which is a branch origin will
/// not have and is refused by name at the press rather than read as the number
/// somewhere inside it.
///
/// **A URL's trailing fragment and query come off**, because they are what a
/// human copies out of the address bar: `…/pull/41#issuecomment-9` and
/// `…/pull/41/files` are that pull request, written the way GitHub handed them
/// out. A trailing slash likewise.
pub(crate) fn pull_request_named(target: &str) -> Option<NamedPullRequest> {
    let found = FIELD.captures(target.trim())?;

    if let (Some(owner), Some(repo), Some(number)) = (
        found.name("owner"),
        found.name("repo"),
        found.name("number"),
    ) {
        return Some(NamedPullRequest {
            number: number.as_str().parse().ok()?,
            repository: Some(format!("{}/{}", owner.as_str(), repo.as_str())),
        });
    }

    Some(NamedPullRequest {
        number: found.name("hash")?.as_str().parse().ok()?,
        repository: None,
    })
}

/// And the `owner/repo` a pull request's own URL says it is in, or `None` where
/// the URL is not one of GitHub's.
///
/// What a named repository is checked against: `gh` answers for the Repo's
/// origin and hands back the pull request's URL, so the repository it answered
/// about is that URL's — see [`crate::conversations::take_up`].
pub(crate) fn repository_in(url: &str) -> Option<String> {
    let found = NAMED.captures(url)?;
    let owner = found.name("owner")?;
    let repo = found.name("repo")?;

    Some(format!("{}/{}", owner.as_str(), repo.as_str()))
}

/// The two ways a pull request is named in prose, as one expression so that the
/// first match is the first *name* rather than the first of either kind.
///
/// The URL alternative comes first, so that a URL is read whole where both
/// would match at the same place. The scheme and the `www.` are optional
/// because a human writes the address they copied, and GitHub hands out both.
static NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:https?://)?(?:www\.)?github\.com/(?P<owner>[\w.-]+)/(?P<repo>[\w.-]+)/pull/(?P<number>\d+)|(?:^|[^\w/])#(?P<hash>\d+)",
    )
    .expect("the pull-request expression is written here and compiles")
});

/// And the same two shapes as the whole of a **Target** field, which is what
/// anchoring them says: a field holds the name of one thing, and prose with a
/// number in it is not that.
///
/// The URL keeps whatever GitHub hung off the end of it — a fragment, a query,
/// the `/files` tab, a trailing slash — because those are what is in the
/// address bar when somebody copies one.
static FIELD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:(?:https?://)?(?:www\.)?github\.com/(?P<owner>[\w.-]+)/(?P<repo>[\w.-]+)/pull/(?P<number>\d+)(?:[/?#].*)?|#(?P<hash>\d+))$",
    )
    .expect("the target expression is written here and compiles")
});

#[cfg(test)]
mod tests {
    use super::*;

    /// What a Brief carrying a link says, which is a number and the repository
    /// it is in.
    #[test]
    fn a_url_names_its_number_and_its_repository() {
        let named = pull_request_in(
            "Wrap up https://github.com/tobico/verkstead/pull/41, which is the rate limiter.",
        )
        .expect("that names one");

        assert_eq!(named.number, 41);
        assert_eq!(named.repository.as_deref(), Some("tobico/verkstead"));
    }

    /// And a bare number names a number and nothing else: which repository it
    /// is in is the Conversation's Repo.
    #[test]
    fn a_bare_number_names_no_repository() {
        let named = pull_request_in("Please wrap #41 up.").expect("that names one");

        assert_eq!(named.number, 41);
        assert_eq!(named.repository, None);
    }

    /// The first of either, wherever in the prose it falls — and a URL is one
    /// name rather than a URL and the `#` of its own fragment.
    #[test]
    fn the_first_of_either_is_what_is_read() {
        assert_eq!(
            pull_request_in("#7 is the one, not https://github.com/tobico/verkstead/pull/41")
                .expect("that names one")
                .number,
            7,
        );

        let url = pull_request_in(
            "Start from https://github.com/tobico/verkstead/pull/41#issuecomment-9 and #7 after it",
        )
        .expect("that names one");

        assert_eq!(url.number, 41);
        assert_eq!(url.repository.as_deref(), Some("tobico/verkstead"));
    }

    /// A heading's `#` is followed by a space rather than by digits, so a Brief
    /// that opens on one names nothing.
    #[test]
    fn a_heading_names_nothing() {
        assert_eq!(
            pull_request_in("# Rate limiting\n\nThe public API needs a ceiling.\n"),
            None,
        );
    }

    /// And a `#` that does not start a word is not a bare number: GitHub's
    /// cross-repository shorthand names a repository this origin is not, and a
    /// number read out of it would be asked of the wrong GitHub.
    #[test]
    fn a_hash_inside_a_word_names_nothing() {
        assert_eq!(pull_request_in("tobico/askance#41 is the one"), None);
    }

    /// A Brief that names nothing at all.
    #[test]
    fn prose_with_no_pull_request_in_it_names_nothing() {
        assert_eq!(pull_request_in("The rate limiter needs a review."), None);
    }

    /// A number at the very start of the Brief is a number all the same.
    #[test]
    fn a_number_opening_the_brief_is_read() {
        assert_eq!(
            pull_request_in("#41 — the rate limiter")
                .expect("that names one")
                .number,
            41,
        );
    }

    /// A Target field holding a URL names that pull request, whichever of the
    /// addresses GitHub hands out was copied into it.
    #[test]
    fn a_target_holding_a_url_names_its_pull_request() {
        for field in [
            "https://github.com/tobico/verkstead/pull/41",
            "http://www.github.com/tobico/verkstead/pull/41",
            "github.com/tobico/verkstead/pull/41",
            "https://github.com/tobico/verkstead/pull/41/files",
            "https://github.com/tobico/verkstead/pull/41#issuecomment-9",
            "  https://github.com/tobico/verkstead/pull/41  ",
        ] {
            let named = pull_request_named(field).unwrap_or_else(|| panic!("{field} names one"));

            assert_eq!(named.number, 41);
            assert_eq!(named.repository.as_deref(), Some("tobico/verkstead"));
        }
    }

    /// And one holding a bare number names a number, in whichever repository
    /// the Conversation is on.
    #[test]
    fn a_target_holding_a_number_names_no_repository() {
        let named = pull_request_named("#41").expect("that names one");

        assert_eq!(named.number, 41);
        assert_eq!(named.repository, None);
    }

    /// Everything else in that field is a branch — a branch name, and prose
    /// with a number in it just the same. The field holds the name of one
    /// thing, so a number inside a sentence is not the thing it names.
    #[test]
    fn a_target_holding_anything_else_names_no_pull_request() {
        for field in [
            "rate-limiting",
            "feature/rate-limiting",
            "",
            "   ",
            "fix #41 first",
            "#41 — the rate limiter",
            "41",
            "tobico/askance#41",
        ] {
            assert_eq!(
                pull_request_named(field),
                None,
                "{field} is a branch, not a pull request",
            );
        }
    }

    /// And what `gh`'s own answer says about which repository it answered for.
    #[test]
    fn a_pull_requests_url_says_which_repository_it_is_in() {
        assert_eq!(
            repository_in("https://github.com/tobico/verkstead/pull/41").as_deref(),
            Some("tobico/verkstead"),
        );
        assert_eq!(repository_in("https://example.invalid/whatever"), None);
    }
}
