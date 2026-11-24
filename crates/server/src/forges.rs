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
//! **A plugin's component answers it** — see [`plugged`], which is what every
//! Forge is: the operations are its component's exports, asked as the Repo's
//! Author with the token set in the component's environment for the one call.
//! GitHub's is the built-in plugin's, embedded in this binary.
//!
//! **And the words are read inside the Forge.** What an API or a tool of the
//! Forge's own answers is read into these shapes there and nowhere else, so a
//! caller never reads anybody's text: it branches on [`Trouble`]'s kinds.
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

pub(crate) mod plugged;

pub(crate) use crate::plugins::manifest::forge::Capability;
use crate::settings::Settings;
use crate::store;
use crate::targets::NamedPullRequest;

/// A pull request to open: the branch it is from and the one it goes into, and
/// what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Whether there is a token to ask as at all. An Author with none for this
    /// Forge leaves it unasked (Set 1878, Q3): nothing falls back to the
    /// host's own login.
    fn tokened(&self) -> bool;

    /// Who `token` authenticates as, and what the Forge will let it do — the
    /// scopes it found, which [`Account::short_of`] reads against the ones
    /// the Forge's manifest declares it needs.
    fn verify(&self, token: &str) -> Result<Account, Trouble>;

    /// The pull request open on `branch` of `repo`.
    fn pull_request(&self, repo: &Path, branch: &str) -> Result<store::PullRequest, Trouble>;

    /// The pull request numbered `number` of `repo`, open or not.
    fn pull_request_numbered(&self, repo: &Path, number: i64) -> Result<Numbered, Trouble>;

    /// Open `opening` as a draft, and answer it.
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
    /// there — `Ok(())` where it is.
    ///
    /// Read off the directory it would be in under the home `environment`
    /// names, rather than by running the tool: an extension lives inside that
    /// home, and the server runs none of a Forge's tools itself (Set 1878,
    /// Q4).
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
    /// Whatever the host is logged in as — a Forge with no settings behind it
    /// to read a token out of, which is a stand-in's.
    Host,

    /// The Repo's Author holds no token for this Forge, or there is no
    /// Author at all: the Forge is not asked (Set 1878, Q3), and a git of
    /// Verkstead's own is left to the machine's own configuration — an SSH
    /// key, most likely — exactly as before there were tokens.
    Untokened,

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
            AsWhom::Host | AsWhom::Untokened => {}
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
            AsWhom::Untokened => f.write_str("Credential(the machine's, no token held)"),
            AsWhom::Awaited => f.write_str("Credential(nobody's, a token awaited)"),
        }
    }
}

/// Who a call about a Repo is to authenticate as, read now: the Author's token
/// for the Forge `forge`, or — where they hold none, or there is no Author —
/// nobody, and the Forge is not asked (Set 1878, Q3). The host's own login for
/// a tool is never read in its place.
///
/// One reading for every implementation of [`Forge`], because it is one
/// question about this installation's files rather than anything a Forge
/// answers: which Author a Repo recorded, whether this device has their copy
/// of the token, and what it is. `settings` is `None` on a Forge with no Data
/// Directory behind it — a stand-in's — which is left to whatever it is.
pub(crate) fn whom(settings: Option<&Settings>, author: Option<&str>, forge: &str) -> AsWhom {
    let Some(settings) = settings else {
        return AsWhom::Host;
    };

    let config = settings.config();

    let Some(author) = config.author_of(author) else {
        return AsWhom::Untokened;
    };

    if config.token_awaited_on(author.id(), forge) {
        return AsWhom::Awaited;
    }

    match config.token_of_on(author, &settings.secrets(), forge) {
        Some(token) => AsWhom::Token(token.to_owned()),
        None => AsWhom::Untokened,
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
    /// Nobody is logged in to the Forge: the Repo's Author holds no token for
    /// it, which leaves it unasked, or the Forge turned the token down.
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

    /// The Repo is on no Forge: its remote names `host`, which no enabled
    /// Forge answers for — see [`matched`]. The host's too, and asked before
    /// any Forge is: there is nobody to ask. A Repo on none has no wrap-up, so
    /// what meets this lands the work Done rather than stopping it.
    NoForge(Option<String>),
}

impl Trouble {
    /// The sentence the Timeline carries, and the one the viewer draws as plain
    /// text beside a press that asked the Forge something — which is why
    /// nothing in it is set as `code`: a backtick there would be drawn as a
    /// backtick.
    pub(crate) fn why(&self) -> String {
        match self {
            Trouble::NotLoggedIn => {
                "This repo's author has no GitHub token that works, so Verkstead can't ask \
                 GitHub anything. Set one in Settings, under Git."
                    .to_owned()
            }
            Trouble::NoRemote => {
                "This repo has no remote GitHub could be asked about.".to_owned()
            }
            Trouble::NoPullRequest => {
                "GitHub has no pull request on this branch, so the session that finished didn't open one."
                    .to_owned()
            }
            Trouble::Refused(said) => format!("GitHub said: {}", crate::stopping::ended(said)),
            Trouble::Ours(said) => said.clone(),
            Trouble::NoForge(host) => unmatched(host.as_deref()),
        }
    }
}

/// Who a token is, and what it may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Account {
    /// The login GitHub answered with.
    pub(crate) login: String,

    /// And the scopes it says the token carries.
    pub(crate) scopes: Scopes,
}

impl Account {
    /// The scopes of `needed` — the Forge manifest's, in its order — this
    /// token is known not to carry. Empty on one that carries them all, and
    /// on one the Forge said nothing about — see [`Scopes::known_to_lack`].
    pub(crate) fn short_of(&self, needed: &[String]) -> Vec<String> {
        needed
            .iter()
            .filter(|scope| self.scopes.known_to_lack(scope))
            .cloned()
            .collect()
    }
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
    /// the Forge's reading of a check, where green is the reading that has to be earned — the
    /// two are different questions, and the honest answer to each is the one
    /// that concludes nothing from a word nobody here has seen before.
    Unknown,
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
    /// there is one, its URL where there is not — see the GitHub plugin's `api`
    /// module.
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

/// A Forge enabled on this cluster, as a Repo's remote is matched against it:
/// which plugin it is, and every host it answers for — the ones its manifest
/// declares and the ones the human added in its settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Enabled {
    pub(crate) uuid: String,
    pub(crate) name: String,
    pub(crate) hosts: Vec<String>,

    /// What it calls a pull request, as its manifest declares: the word the
    /// screen says for one of a Repo on it (ADR-0030, *Forge*).
    pub(crate) noun: String,

    /// What it can do, as its manifest declares. One it does not declare is
    /// settled rather than failed — see [`Enabled::has`].
    pub(crate) capabilities: Vec<Capability>,
}

impl Enabled {
    /// Whether it declares `capability`.
    pub(crate) fn has(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// The built-in GitHub as it is when nothing could be read about it: every
    /// capability, its own noun, and no hosts to match by.
    fn github() -> Enabled {
        Enabled {
            uuid: crate::plugins::built_in::GITHUB.to_owned(),
            name: "GitHub".to_owned(),
            hosts: Vec::new(),
            noun: PULL_REQUEST.to_owned(),
            capabilities: Capability::ALL.to_vec(),
        }
    }
}

/// What a Forge calls a pull request where nothing better is known: GitHub's
/// word, and the documents'.
pub(crate) const PULL_REQUEST: &str = "pull request";

impl Matched {
    /// Whether the Repo's Forge declares `capability`. A Repo on none has
    /// nothing to have it of.
    pub(crate) fn has(&self, capability: Capability) -> bool {
        match self {
            Matched::On(forge) => forge.has(capability),
            Matched::Unmatched { .. } => false,
        }
    }

    /// The noun the Repo's Forge calls a pull request, or the documents' own
    /// word for a Repo on none — which has no pull request to call anything.
    pub(crate) fn noun(&self) -> &str {
        match self {
            Matched::On(forge) => &forge.noun,
            Matched::Unmatched { .. } => PULL_REQUEST,
        }
    }
}

/// The key of the `list` setting a Forge's manifest declares for the hosts the
/// human adds to it: a self-hosted instance is a host added, not a plugin
/// written (ADR-0030, *Forge*).
pub(crate) const HOSTS_SETTING: &str = "hosts";

/// The Forges a token is asked for under each Author: the built-in GitHub
/// where it is enabled, then every Forge an installed plugin that loaded here
/// declares, in the order they were installed (ADR-0030, *Forge*). Each with
/// the scopes its manifest says a token needs.
///
/// Wider than [`enabled`], which is the Forges a Repo is matched against: a
/// token can be set for a Forge before its component can be asked anything.
pub(crate) async fn taking_tokens(state: &crate::AppState) -> Vec<verkstead_render::TokenForge> {
    let records =
        match crate::store::replicated::live_records::<crate::replicating::plugins::Plugins>(
            &state.pool,
        )
        .await
        {
            Ok(records) => records,
            Err(error) => {
                tracing::error!(error = ?error, "reading which forges are enabled failed");
                Vec::new()
            }
        };

    let built_in = crate::plugins::built_in::listed()
        .iter()
        .filter(|manifest| {
            !records
                .iter()
                .any(|record| record.uuid == manifest.uuid && !record.fields.enabled)
        })
        .filter_map(|manifest| {
            let forge = manifest
                .provides
                .iter()
                .find_map(|provision| provision.forge())?;

            Some(verkstead_render::TokenForge {
                uuid: manifest.uuid.clone(),
                name: manifest.name.clone(),
                scopes: forge.scopes.clone(),
            })
        });

    built_in.chain(state.plugins.forges()).collect()
}

/// Which Forge a Repo is on, read off its remote at the moment it is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Matched {
    /// The enabled Forge whose hosts the remote names.
    On(Enabled),

    /// None: the remote names `host`, and no enabled Forge answers for it —
    /// or, with no host, there is no enabled Forge at all to leave it to.
    Unmatched { host: Option<String> },
}

impl Matched {
    /// What the viewer is handed of it, beside the `origin` the match was read
    /// off — the one reading answering both, which is why the URL comes in
    /// rather than being asked for again here.
    pub(crate) fn drawn(&self, origin: Option<String>) -> verkstead_render::RepoForge {
        verkstead_render::RepoForge {
            origin,
            on: match self {
                Matched::On(forge) => verkstead_render::ForgeMatch::On {
                    uuid: forge.uuid.clone(),
                    name: forge.name.clone(),
                    noun: forge.noun.clone(),
                    capabilities: forge
                        .capabilities
                        .iter()
                        .map(|capability| drawn(*capability))
                        .collect(),
                },
                Matched::Unmatched { host } => {
                    verkstead_render::ForgeMatch::Unmatched { host: host.clone() }
                }
            },
        }
    }
}

/// A capability as the viewer is handed it.
fn drawn(capability: Capability) -> verkstead_render::ForgeCapability {
    match capability {
        Capability::Checks => verkstead_render::ForgeCapability::Checks,
        Capability::Review => verkstead_render::ForgeCapability::Review,
        Capability::Publish => verkstead_render::ForgeCapability::Publish,
        Capability::Stacks => verkstead_render::ForgeCapability::Stacks,
        Capability::Create => verkstead_render::ForgeCapability::Create,
    }
}

/// The Forges enabled on this cluster: every one registered in-process on
/// this server first — see [`crate::forge::Registered`] — then the built-ins,
/// in the order they are listed.
///
/// **Of the built-ins, only GitHub for now**: it is the one Forge the server
/// has an implementation of, answered in-process. An installed plugin
/// declaring a Forge is matched once its component can answer — until then a
/// Repo on its host would be a Repo on a Forge nothing can ask.
///
/// Read off the Plugins records each time: a tick or a host added on any
/// device reaches the next question without a restart.
pub(crate) async fn enabled(state: &crate::AppState) -> anyhow::Result<Vec<Enabled>> {
    let mut enabled = state.forge.registered();
    enabled.extend(built_in(&state.pool).await?);

    Ok(enabled)
}

/// The same, where reading the records failed left as the built-in GitHub
/// alone, after whatever is registered: what [`of`] matches against.
pub(crate) async fn enabled_now(state: &crate::AppState) -> Vec<Enabled> {
    match enabled(state).await {
        Ok(enabled) => enabled,
        Err(error) => {
            tracing::error!(error = ?error, "reading which forges are enabled failed");

            let mut enabled = state.forge.registered();
            enabled.push(Enabled::github());
            enabled
        }
    }
}

/// The built-in Forges the Plugins records leave enabled.
pub(crate) async fn built_in(pool: &sqlx::SqlitePool) -> anyhow::Result<Vec<Enabled>> {
    use crate::replicating::plugins::{Plugins, SettingValue};

    let records = crate::store::replicated::live_records::<Plugins>(pool).await?;

    Ok(crate::plugins::built_in::listed()
        .iter()
        .filter(|manifest| manifest.uuid == crate::plugins::built_in::GITHUB)
        .filter_map(|manifest| {
            let declared = manifest
                .provides
                .iter()
                .find_map(|provision| provision.forge())?;
            let kept = records.iter().find(|record| record.uuid == manifest.uuid);
            if kept.is_some_and(|record| !record.fields.enabled) {
                return None;
            }

            let added = kept
                .and_then(|record| record.fields.values.get(HOSTS_SETTING))
                .map(|value| match value {
                    SettingValue::Lines(lines) => lines.clone(),
                    SettingValue::Line(line) => vec![line.clone()],
                })
                .unwrap_or_default();

            let mut hosts: Vec<String> = Vec::new();
            for host in declared.hosts.iter().chain(&added) {
                let host = host.trim().to_ascii_lowercase();
                if !host.is_empty() && !hosts.contains(&host) {
                    hosts.push(host);
                }
            }

            Some(Enabled {
                uuid: manifest.uuid.clone(),
                name: manifest.name.clone(),
                hosts,
                noun: declared.noun.clone(),
                capabilities: declared.capabilities.clone(),
            })
        })
        .collect())
}

/// What the viewer is handed of the Repo at `repo`: the URL of its `origin`
/// remote, and the Forge that remote's host matches among `enabled`.
///
/// **One `git remote get-url` answers both**, which is the whole reason this
/// exists beside [`among`]: a row that carried the two separately asked git
/// for the same URL twice.
pub(crate) async fn drawn_for(enabled: &[Enabled], repo: &Path) -> verkstead_render::RepoForge {
    let repo = repo.to_owned();
    let origin = tokio::task::spawn_blocking(move || crate::repos::origin(&repo))
        .await
        .unwrap_or_default();

    matched(enabled, origin.as_deref()).drawn(origin)
}

/// The same, against the Forges enabled at this moment — see [`enabled_now`].
pub(crate) async fn drawn_of(state: &crate::AppState, repo: &Path) -> verkstead_render::RepoForge {
    drawn_for(&enabled_now(state).await, repo).await
}

/// The Forge of the Repo whose `origin` remote is this, among `enabled`.
///
/// **A remote with no host is left to the first enabled Forge**, as every
/// Repo was before there was more than one: a path on this machine, a
/// `file://` URL or no `origin` at all names nothing to match, and what the
/// Forge then says about it — no remote, most likely — is what it always said.
/// With nothing enabled it is on none.
pub(crate) fn matched(enabled: &[Enabled], origin: Option<&str>) -> Matched {
    let Some(host) = origin.and_then(crate::matching::host_of) else {
        return match enabled.first() {
            Some(forge) => Matched::On(forge.clone()),
            None => Matched::Unmatched { host: None },
        };
    };

    match enabled.iter().find(|forge| forge.hosts.contains(&host)) {
        Some(forge) => Matched::On(forge.clone()),
        None => Matched::Unmatched { host: Some(host) },
    }
}

/// The Forge of the Repo at `repo`, read now: its `origin` asked of git and
/// matched against the Forges enabled at this moment.
///
/// A store that cannot be read leaves the Repo to the built-in GitHub, rather
/// than to none: a Repo read as on no Forge lands its work Done with nothing
/// watching, which is not a thing to do over a read that failed. See
/// [`enabled_now`].
pub(crate) async fn of(state: &crate::AppState, repo: &Path) -> Matched {
    among(&enabled_now(state).await, repo).await
}

/// The same, matched against `enabled` as already read.
pub(crate) async fn among(enabled: &[Enabled], repo: &Path) -> Matched {
    let repo = repo.to_owned();
    let origin = tokio::task::spawn_blocking(move || crate::repos::origin(&repo))
        .await
        .unwrap_or_default();

    matched(enabled, origin.as_deref())
}

/// Why there is nothing for a Forge to say about a Repo on none, in a
/// sentence for the Timeline.
pub(crate) fn unmatched(host: Option<&str>) -> String {
    match host {
        Some(host) => format!(
            "No forge for {host}, so there's no pull request to wrap up. Add {host} to a \
             forge's hosts in Plugins to have it watched."
        ),
        None => "No forge is enabled, so there's no pull request to wrap up.".to_owned(),
    }
}

/// Why a wrap-up does not wait on what a Forge lacking `capability` — checks,
/// or review — would have said about pull request `number`, in a sentence for
/// the Timeline. `None` for a capability no wrap-up watcher waits on.
pub(crate) fn unwatched(
    forge: &str,
    noun: &str,
    capability: Capability,
    number: i64,
) -> Option<String> {
    match capability {
        Capability::Checks => Some(format!(
            "{forge} has no checks, so the wrap-up doesn't wait on any for {noun} #{number}."
        )),
        Capability::Review => Some(format!(
            "{forge} has no reviews, so the wrap-up doesn't wait on anything said on {noun} \
             #{number}."
        )),
        Capability::Publish | Capability::Stacks | Capability::Create => None,
    }
}

/// Why a Share on a Repo whose Forge can't publish has no Publish, in a
/// sentence.
pub(crate) fn publishless(forge: &str) -> String {
    format!("{forge} can't publish shares, so there's nowhere to publish this one.")
}

/// Why a repository made with a remote asked for was made here only, its
/// Forge being one that can't create one.
pub(crate) fn createless(forge: Option<&str>) -> String {
    match forge {
        Some(forge) => format!("{forge} can't create repositories, so no remote was made."),
        None => "No forge is enabled, so no remote was made.".to_owned(),
    }
}

/// Why a Share on a Repo on no Forge has no Publish, in a sentence.
pub(crate) fn unpublishable(host: Option<&str>) -> String {
    match host {
        Some(host) => format!("No forge for {host}, so there's nowhere to publish the share."),
        None => "No forge is enabled, so there's nowhere to publish the share.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github(hosts: &[&str]) -> Enabled {
        Enabled {
            uuid: crate::plugins::built_in::GITHUB.to_owned(),
            name: "GitHub".to_owned(),
            hosts: hosts.iter().map(|host| (*host).to_owned()).collect(),
            noun: PULL_REQUEST.to_owned(),
            capabilities: Capability::ALL.to_vec(),
        }
    }

    /// A remote on a host a Forge answers for is on it, whichever spelling
    /// names the host; one on any other host is on none, naming it.
    #[test]
    fn a_remote_is_on_the_forge_its_host_names() {
        let enabled = [github(&["github.com", "github.example.com"])];

        for origin in [
            "https://github.com/tobico/verkstead.git",
            "git@github.example.com:tobico/verkstead.git",
        ] {
            assert_eq!(
                matched(&enabled, Some(origin)),
                Matched::On(enabled[0].clone()),
                "{origin}"
            );
        }

        assert_eq!(
            matched(&enabled, Some("https://gitlab.com/tobico/verkstead")),
            Matched::Unmatched {
                host: Some("gitlab.com".to_owned())
            }
        );
    }

    /// A remote naming no host — a path, or none at all — is left to the first
    /// enabled Forge, as every Repo was; with none enabled it is on none.
    #[test]
    fn a_remote_with_no_host_is_left_to_the_first_forge() {
        let enabled = [github(&["github.com"])];

        assert_eq!(
            matched(&enabled, Some("/srv/upstream.git")),
            Matched::On(enabled[0].clone())
        );
        assert_eq!(matched(&enabled, None), Matched::On(enabled[0].clone()));
        assert_eq!(matched(&[], None), Matched::Unmatched { host: None });
    }
}
