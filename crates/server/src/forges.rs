//! The **Forge** as the server reaches it: one interface between Verkstead and
//! wherever a Repo's pull requests live (ADR-0030, *Forge*).
//!
//! [`Forge`] carries every operation the server asks of a Forge — finding a
//! pull request by branch or by number, opening one as a draft against a base,
//! reading its details, checks, comments and how it landed, commenting and
//! replying on it, listing the open ones, publishing a Share and taking one
//! down, creating a repository, verifying a token and its scopes, reading a
//! target out of prose, and saying whether a remote is one of its own — and
//! every caller goes through it. The answers are the shapes below, and a
//! failure is one of the interface's error kinds, [`Trouble`].
//!
//! **One implementation for now**: GitHub, in-process over the host's tool —
//! see [`crate::github`], the one module that runs it. What that tool prints is
//! read into these shapes there and nowhere else, so a caller never reads any
//! tool's text.
//!
//! **Blocking, every operation.** An implementation may run a process or wait
//! on the network, so the async callers ask from `spawn_blocking` with a
//! handle of their own — [`Forge::as_author`] hands one out, and it is `Send`.
//!
//! **As an Author.** Every operation about a Repo is asked as the Author the
//! Repo records, whose token is read at the moment of the call — see
//! [`Forge::as_author`] — so a token saved through the settings page reaches
//! the next call without a restart.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use crate::store;
use crate::targets::NamedPullRequest;

/// A pull request to open: the branch it is from and the one it goes into, and
/// what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "`verkstead pull-request open` is its first caller"
    )
)]
pub(crate) struct Opening {
    pub(crate) head: String,
    pub(crate) base: String,
    pub(crate) title: String,
    pub(crate) body: String,
}

/// Where a Repo's pull requests live, as Verkstead asks it.
///
/// Every operation blocks — see the module's notes.
pub(crate) trait Forge: Send + Sync + std::fmt::Debug {
    /// The same Forge, asking as the Author a Repo recorded as `recorded` — its
    /// `author`, read off the store with the rest of it. `None` is a Repo that
    /// records none, which is the one Author's where there is only one.
    fn as_author(&self, recorded: Option<&str>) -> Arc<dyn Forge>;

    /// What Verkstead's own git on the Repo authenticates as — the same token
    /// this Forge would be asked with, read now. See [`Credential`].
    fn credential(&self) -> Credential;

    /// Whether there is a token to ask as at all — which, where there is none,
    /// would leave a call to the host's own login.
    ///
    /// What a write asks before it is made: a read under the host's login is a
    /// read, and a repository made under it is one in an account nobody chose.
    fn tokened(&self) -> bool;

    /// The token the host itself is logged in to this Forge with, where it has
    /// one: what the onboarding wizard prefills its token field from.
    fn host_token(&self) -> Option<String>;

    /// Who `token` authenticates as, and what the Forge will let it do.
    fn verify(&self, token: &str) -> Result<Account, Trouble>;

    /// The pull request open on `branch` of `repo`.
    fn pull_request(&self, repo: &Path, branch: &str) -> Result<store::PullRequest, Trouble>;

    /// The pull request numbered `number` of `repo`, open or not.
    fn pull_request_numbered(&self, repo: &Path, number: i64) -> Result<Numbered, Trouble>;

    /// Open `opening` as a draft, and answer it.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "`verkstead pull-request open` is its first caller"
        )
    )]
    fn open_pull_request(&self, repo: &Path, opening: &Opening) -> Result<Numbered, Trouble>;

    /// Every pull request open on `repo`, which is what a stack is walked off.
    fn open_pull_requests(&self, repo: &Path) -> Result<Vec<Numbered>, Trouble>;

    /// What the details pane draws of pull request `number`, and the facts that
    /// ride the same question.
    fn details(&self, repo: &Path, number: i64) -> Result<Details, Trouble>;

    /// The checks against pull request `number`'s head, and whether it merges.
    fn checks(&self, repo: &Path, number: i64) -> Result<Suite, Trouble>;

    /// Whether pull request `number` merges, and where it has got to.
    fn landing(&self, repo: &Path, number: i64) -> Result<Landing, Trouble>;

    /// Everything said on pull request `number`, in the order it was said.
    fn comments(&self, repo: &Path, number: i64) -> Result<Vec<Comment>, Trouble>;

    /// Say `body` on pull request `number` as `token`, and answer where it was
    /// said.
    fn comment(&self, repo: &Path, token: &str, number: i64, body: &str)
    -> Result<String, Trouble>;

    /// Say `body` on pull request `number` as the Author, and answer where.
    fn reply(&self, repo: &Path, number: i64, body: &str) -> Result<String, Trouble>;

    /// Publish a Share: one file, `name`, holding `content`, as `token`.
    fn create_gist(
        &self,
        token: &str,
        description: &str,
        name: &str,
        content: &str,
    ) -> Result<Gist, Trouble>;

    /// And take a published one down again.
    fn delete_gist(&self, token: &str, id: &str) -> Result<(), Trouble>;

    /// Make a private repository called `name` of the one at `repo`, and push it
    /// there.
    fn create_repository(&self, repo: &Path, name: &str) -> Result<(), Trouble>;

    /// Whether the extension `name` of the tool this Forge lends a session is
    /// there, asked in a session's `environment` — `Ok(())` where it is.
    fn tool_extension(&self, name: &str, environment: &[(String, OsString)])
    -> Result<(), Trouble>;

    /// Whether `trouble`, which making a gist failed with, is the Forge turning
    /// the token down for what it may do rather than turning the request down.
    fn lacks_scope(&self, trouble: &Trouble) -> bool;

    /// The first pull request `prose` names, which is what a saved Brief fills
    /// an empty Target with.
    fn target_in(&self, prose: &str) -> Option<NamedPullRequest>;

    /// The pull request a Target field names as the whole of it, or `None`
    /// where what it holds is a branch.
    fn target_named(&self, target: &str) -> Option<NamedPullRequest>;

    /// The repository a pull request's `url` is in, as the Forge names one.
    fn repository_in(&self, url: &str) -> Option<String>;

    /// The repository on this Forge that the remote `origin` is, or `None`
    /// where it is not one of this Forge's own.
    fn repository_of(&self, origin: &str) -> Option<String>;
}

/// What a git of Verkstead's own presents to a remote: an Author's token, or
/// nothing at all, which leaves git to the host's own credentials.
///
/// Every git that reaches a remote on a Repo goes through this — the fetch
/// before a cut, the fetch that draws a page, the checks' `ls-remote` and a
/// share's clone and push — so that a private repository under the second
/// Author's account is fetched as that Author, exactly as the Forge is already asked
/// about it. See [`Forge::credential`].
#[derive(Clone)]
pub(crate) struct Credential(pub(crate) AsWhom);

/// Who a git or a Forge call of Verkstead's own authenticates as.
#[derive(Clone)]
pub(crate) enum AsWhom {
    /// Whatever the host is logged in as: an Author with no token, or no
    /// Author at all.
    Host,

    /// This token and no other.
    Token(String),

    /// Nobody: the Repo's Author's record says a token is held for it, and
    /// this device's copy has not arrived. The host's login
    /// in its place is the mistake an Author per Repo exists to stop
    /// (ADR-0025), so a git asks as nobody and the Forge is not asked.
    Awaited,
}

impl Credential {
    /// The host's own, which is what an Author with no token leaves git to:
    /// nothing is overridden, exactly as before there were Authors.
    #[cfg(test)]
    pub(crate) fn host() -> Credential {
        Credential(AsWhom::Host)
    }

    /// This token and no other — see [`crate::publishing`], which has read one
    /// itself.
    pub(crate) fn token(token: &str) -> Credential {
        Credential(AsWhom::Token(token.to_owned()))
    }

    /// Have `git` authenticate as it, and never stop to ask.
    ///
    /// **The token goes through the environment.** The helper is a shell
    /// function that reads `GH_TOKEN`, so what appears in the process list is
    /// the shape of the helper and never the credential — which is exactly what
    /// putting the token in the remote URL would do instead. The empty
    /// `credential.helper=` in front of it clears whatever the machine has
    /// configured, so that a git given a token authenticates as that token and
    /// never as the host's stored login.
    ///
    /// Declared on every call rather than only on the ones that talk to the
    /// network, because which of them does is git's business: a clone may
    /// fetch, a push may re-fetch, and a helper that is there and unasked costs
    /// nothing. And so `command` is expected to be a `git` with no arguments
    /// yet: `-c` goes in front of the subcommand.
    ///
    /// Where there is no token the machine's configuration stands untouched —
    /// and the prompt is turned off all the same: a terminal prompt from a
    /// credential helper that found nothing would be a server thread waiting
    /// for ever. Where a token is awaited the machine's helpers are cleared
    /// and none put in their place, so git asks as nobody rather than as the
    /// host.
    pub(crate) fn on<'c>(&self, command: &'c mut Command) -> &'c mut Command {
        const HELPER: &str = r#"!f() { if test "$1" = get; then printf 'username=x-access-token\npassword=%s\n' "$GH_TOKEN"; fi; }; f"#;

        match &self.0 {
            AsWhom::Host => {}
            AsWhom::Token(token) => {
                command
                    .args(["-c", "credential.helper=", "-c"])
                    .arg(format!("credential.helper={HELPER}"))
                    .env("GH_TOKEN", token);
            }
            AsWhom::Awaited => {
                command
                    .args(["-c", "credential.helper="])
                    .env_remove("GH_TOKEN");
            }
        }

        command.env("GIT_TERMINAL_PROMPT", "0")
    }
}

/// Never the token: a `Credential` in a log line says whose it is not, and
/// whether there is one is all a reader needs.
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            AsWhom::Token(_) => f.write_str("Credential(an Author's token)"),
            AsWhom::Host => f.write_str("Credential(the host's)"),
            AsWhom::Awaited => f.write_str("Credential(nobody's, a token awaited)"),
        }
    }
}

/// What a Forge call or a git about a Repo says in place of running, where the
/// Repo's Author holds a token on another device and the copy of it has not
/// reached this one.
pub(crate) fn awaited() -> String {
    "This repo's author has a token on another device, and it hasn't reached this one yet."
        .to_owned()
}

/// Why the Forge could not answer: the interface's error kinds, which every
/// operation of [`Forge`] fails with.
///
/// Each of these is a different thing for the human to go and do, which is the
/// whole reason they are told apart — and the reason a caller branches on the
/// kind rather than on anybody's text: [`crate::runner`] and [`crate::done`]
/// tell a branch with no pull request from a Forge that would not answer by
/// the variant, and what a Forge's own tool printed is read into one of these
/// inside that Forge's implementation and nowhere else. Its sentence is
/// [`Trouble::why`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Trouble {
    /// Nobody is logged in to the Forge: no token for the Author, and nothing
    /// on the host to fall back on.
    NotLoggedIn,

    /// The repository has no remote for the Forge to work out which of its
    /// repositories it is.
    NoRemote,

    /// GitHub has nothing under what it was asked about: no pull request on the
    /// branch, or none under the number.
    ///
    /// Two sentences out of GitHub and one reason, because the selector is the
    /// caller's and not GitHub's: asked with a branch it says there are no pull
    /// requests found, and asked with a number it says it could not resolve one.
    /// Every caller already knows which it asked with — see [`Self::why`], which
    /// words it for the branch, and [`crate::conversations::resolve`], which
    /// words it for the number.
    NoPullRequest,

    /// Something else, in the Forge's own words.
    Refused(String),

    /// A failure in Verkstead rather than at the Forge — it could not be asked,
    /// said nothing, or answered what could not be read — in a sentence of
    /// Verkstead's own, so that none of it is passed off as a quotation. The
    /// host's, and never one of the kinds a Forge itself fails with.
    Ours(String),
}

/// Who a token is, and what it may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Account {
    /// The login GitHub answered with.
    pub(crate) login: String,

    /// And the scopes it says the token carries.
    pub(crate) scopes: Scopes,
}

/// What GitHub said about a token's scopes, which has three answers rather than
/// two.
///
/// The third is the one worth the type: a **fine-grained** token has permissions
/// rather than scopes, and GitHub answers for one with no scopes header at all.
/// A missing header read as an empty list would have Verkstead refuse to publish
/// with a token that publishes perfectly well, and send the human to re-issue it
/// — so *nothing said* is kept apart from *said, and `gist` was not among them*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scopes {
    /// GitHub named them. Empty is a classic token with no scopes ticked, which
    /// really can do nothing.
    Named(Vec<String>),

    /// GitHub named none, which says nothing either way.
    Unsaid,
}

impl Scopes {
    /// Whether this token is known *not* to carry `scope`.
    ///
    /// The question asked in the negative, and deliberately: what a caller wants
    /// to know is whether to refuse, and a token GitHub said nothing about is
    /// not one to refuse — it is one to try.
    pub(crate) fn known_to_lack(&self, scope: &str) -> bool {
        match self {
            Scopes::Named(named) => !named.iter().any(|named| named == scope),
            Scopes::Unsaid => false,
        }
    }
}

/// The scope publishing a share needs, which is the one Verkstead's own writes
/// to GitHub turn on.
pub(crate) const GIST: &str = "gist";

/// A gist Verkstead has just made: where a reader goes, where git puts the file
/// in, and what to take back if it never gets one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Gist {
    /// GitHub's id for it, which is what deleting one names.
    pub(crate) id: String,

    /// The page a link to a share points at.
    pub(crate) url: String,

    /// And the git remote behind that page, as GitHub gave it — a gist is a
    /// repository, and this is how the file that matters gets into one.
    pub(crate) push: String,
}

/// One pull request of a repository, by its number, as the Forge answers for
/// that repository's origin.
///
/// What a **Review** is started on: the Target names a number — see
/// [`crate::targets`] — and this is the whole of what GitHub has to say about
/// it before the take-up can run. Nothing about the pull request's own words is
/// here: the Brief is the human's, and nothing of a pull request is ever read
/// into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Numbered {
    pub(crate) number: i64,
    pub(crate) title: String,

    /// Where it is — and, with it, which repository the Forge answered about: a pull
    /// request's URL carries its owner and its repository, which is what a URL
    /// the human named is checked against. See [`crate::targets::repository_in`].
    pub(crate) url: String,

    /// The branch the work is on, unqualified — `tobi/steer` rather than
    /// `origin/tobi/steer`.
    pub(crate) head: String,

    /// And the branch it goes into.
    pub(crate) base: String,

    /// Whether the head branch is in another repository — a fork. Such a pull
    /// request cannot be pushed to over `origin`, so a wrap-up over it would
    /// have nowhere to put a fix.
    pub(crate) fork: bool,

    /// Whether GitHub still has it open. A merged or closed pull request is
    /// answered by number exactly as an open one is, and it is nothing to wrap
    /// up: the caller refuses it by the same name a number nothing is open
    /// under is refused by.
    pub(crate) open: bool,
}

/// One check GitHub is running against a pull request's head commit.
///
/// The name is what a human calls it by and what Verkstead counts fix attempts
/// against — see [`store::fix_attempts`]. The link is where the run itself is,
/// which is the one thing a stop over a red check cannot be read without.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    pub(crate) name: String,
    pub(crate) how: Checked,

    /// Where the run is, as GitHub gives it. Empty where it gave none.
    pub(crate) link: String,
}

/// How one check is getting on.
///
/// Three answers rather than GitHub's dozen, because three is what wrap-up
/// decides between: a green check settles, a red one dispatches a fix session,
/// and one still running is nothing to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Checked {
    /// It finished and it is green. A check that was skipped or that reported
    /// nothing either way counts here: neither is something to fix.
    Passed,

    /// It finished and it is not green.
    ///
    /// Cancelled counts here, which is the one reading worth saying out loud: a
    /// cancelled run is not passing and is not going to start passing on its
    /// own, so reading it as *still running* would be a wrap-up that waited for
    /// ever rather than one that eventually asks the human.
    Failed,

    /// It has not finished.
    Running,
}

/// Whether GitHub can merge the pull request into its base as it stands.
///
/// Three answers, and each of them is its own thing to a wrap-up: a conflicting
/// pull request is one nothing can land, and one GitHub has not made its mind up
/// about yet is neither that nor a clean merge — see [`crate::checks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mergeable {
    /// It merges: `MERGEABLE`.
    Cleanly,

    /// It does not: `CONFLICTING`, which is the branch and its base having
    /// changed the same lines since they parted.
    Conflicting,

    /// GitHub has not worked it out — `UNKNOWN`, which is what it answers while
    /// it is still computing, and what an answer carrying no `mergeable` at all
    /// comes to.
    ///
    /// A word this does not know is read as *not known* rather than as a
    /// conflict, which is the way round that matters: not knowing changes
    /// nothing at all, and a conflict holds a wrap-up up. The opposite of
    /// [`crate::github`]'s reading of a check, where green is the reading that has to be earned — the
    /// two are different questions, and the honest answer to each is the one
    /// that concludes nothing from a word nobody here has seen before.
    Unknown,
}

impl Mergeable {
    /// The one GitHub's word names.
    pub(crate) fn read(said: &str) -> Self {
        match said.to_ascii_uppercase().as_str() {
            "MERGEABLE" => Self::Cleanly,
            "CONFLICTING" => Self::Conflicting,
            _ => Self::Unknown,
        }
    }
}

/// Where a pull request has got to, and whether it merges — the two facts one
/// look at a pull request that nobody is wrapping up any more comes back with.
///
/// One question rather than two, GitHub taking a list of fields: a pull request
/// whose base has moved and one somebody has merged are the two things a sweep
/// after Done is looking for, and asking them separately would be two round
/// trips over somebody else's network for one answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Landing {
    /// Whether GitHub can merge it into its base as it stands.
    pub(crate) mergeable: Mergeable,

    /// And where it has got to, which is what says whether there is any point
    /// asking again.
    pub(crate) stands: Stands,
}

/// Where GitHub says a pull request has got to.
///
/// Three answers and a fourth for not knowing, exactly as [`Mergeable`] has: the
/// two endings are final, and *open* and *not known* are each a reason to ask
/// again. Which way an unreadable word falls matters here for [`Mergeable`]'s
/// reason turned around — reading one as an ending would stop the asking for
/// good, so nothing but GitHub's own two words for an ending is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stands {
    /// It is open: `OPEN`.
    Open,

    /// Somebody merged it: `MERGED`.
    Merged,

    /// Somebody closed it without merging it: `CLOSED`.
    Closed,

    /// GitHub said nothing this knows how to read, which includes an answer
    /// carrying no `state` at all. Nothing is concluded from it, the way nothing
    /// is concluded from a Forge that would not answer.
    Unknown,
}

impl Stands {
    /// The one GitHub's word names.
    pub(crate) fn read(said: &str) -> Self {
        match said.to_ascii_uppercase().as_str() {
            "OPEN" => Self::Open,
            "MERGED" => Self::Merged,
            "CLOSED" => Self::Closed,
            _ => Self::Unknown,
        }
    }
}

/// What GitHub says about a pull request's checks, and which commit it is
/// saying it about.
///
/// The two together rather than the checks alone: a rollup is a fact about one
/// commit, and a wrap-up that read it as a fact about the branch would finish
/// over whatever had been pushed since — see [`crate::checks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Suite {
    /// The commit GitHub has the pull request's branch on, which is the commit
    /// the checks below are about.
    ///
    /// Empty where the Forge answered without one, which is a question that came back
    /// short rather than a head of nothing: nothing is concluded from it and the
    /// checks stand on their own, as they did before this was asked for.
    pub(crate) head: String,

    /// Every check GitHub reported against that commit, which is empty for a
    /// commit nothing has run against.
    pub(crate) checks: Vec<Check>,

    /// And whether GitHub can merge the pull request into its base, which is a
    /// fact about the branch rather than about the commit above: a base that has
    /// moved under a branch nobody has touched is what puts a pull request in
    /// conflict.
    ///
    /// Here rather than in an answer of its own because it rides the same call —
    /// see [`Forge::checks`].
    pub(crate) mergeable: Mergeable,
}

/// One thing said on a pull request, as the wrap-up's comment watcher reads it.
///
/// Not [`verkstead_render::Comment`], which is the details pane's and arrives
/// rendered: this one is read by an agent rather than by a browser, so it keeps
/// the markdown it was written in — and it carries the identity, which is the
/// whole of how a comment already dispatched for is told from a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Comment {
    /// What this comment is, for as long as it exists. GitHub's node id where
    /// there is one, its URL where there is not — see [`crate::github`].
    pub(crate) which: String,

    /// Who said it, empty for an account that has since gone.
    pub(crate) author: String,

    /// When, RFC 3339, as GitHub gives it.
    pub(crate) at: String,

    /// Where on the branch they said it: the file and the line, for a comment
    /// left on the diff. Empty for one said about the pull request as a whole,
    /// which is what the conversation and a review's own words are.
    pub(crate) about: String,

    /// And what they said, in the markdown they wrote.
    pub(crate) markdown: String,
}

/// What one reading of a pull request comes back with.
///
/// Two things out of the one question: what the details pane draws, and the
/// checks as [`Forge::checks`] would have read them. The second is not a second copy of
/// the first — it is what the rollup written down beside the Conversation is
/// taken from, and the caller wants the checks in the shape the aggregate is
/// read out of rather than in the shape a page draws.
pub(crate) struct Details {
    pub(crate) pane: verkstead_render::PullRequestDetails,
    pub(crate) checks: Vec<Check>,

    /// And where the pull request has got to and whether it merges, which ride
    /// the same question for the reason the checks do — see [`Landing`]. What
    /// they are for is the recording rather than the pane: opening it is the one
    /// thing that asks GitHub about a pull request nothing is watching or
    /// sweeping, so it freshens every fact that is drawn off a card as well as
    /// the list it came for.
    pub(crate) landing: Landing,
}
