//! What a **Review** Conversation is to take up: the **Target** field read at
//! the press, and the Brief read to fill that field.
//!
//! **The reading is the Forge's.** A pull request is the one target that can be
//! read out of prose, and what counts as a name of one is whatever that Forge
//! says: GitHub takes a `github.com/<owner>/<repo>/pull/<n>` URL and a bare
//! `#<n>`, which is what its component answers
//! `target-in` and `target-named` with. Nothing here reads a string any more —
//! what is left is the shape the answer comes back in, which is the host's
//! because the host is what holds a Conversation. See
//! [`crate::forges::Forge::target_in`], and the GitHub plugin's `names` module,
//! where the reading and its proofs live.
//!
//! What comes back is a *number*, and a repository where the name said one.
//! Everything about the pull request itself — its title, its head branch, the
//! branch it merges into, whether its head is in a fork — is the Forge's, and
//! is asked of it at the press.

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
