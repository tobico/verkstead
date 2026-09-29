//! What a device lacks before a Conversation can be moved onto it, and the one
//! question a device answers about itself for it (ADR-0020, *Transfer*).
//!
//! **A reading of the far end, drawn under the device select in the Transfer
//! dialog.** Nothing here is a press: the preflight is what the dialog shows
//! the moment a device is picked, and what refuses **Go** while anything is in
//! the way — by name, because every one of these findings is something to go
//! and put right somewhere rather than a count to report.
//!
//! **Asked of the far end, with one exception.** Whether one of that device's
//! Repos *is* this repository is settled by [`RepoAcross`](crate::RepoAcross)
//! and the rule over it, which runs on the end that is going to act on the
//! answer — the far end sends its registry and the device holding the work
//! applies it. Everything else only the far end can say, so it answers about
//! itself: which harnesses are on its `PATH`, and whether it is answering at
//! all.
//!
//! **And *unreachable* is never *no match*.** They are two different things to
//! tell a human — no match sends somebody to **Open repo** on a machine that
//! may already have the repository, and a machine that is asleep will answer
//! perfectly well tomorrow — so a device that did not answer is the whole of
//! its own preflight, named, rather than a list of everything it failed to
//! confirm.

use serde::{Deserialize, Serialize};

use crate::AgentType;

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Which of a Conversation's three roles a Pairing is for.
///
/// Here rather than read off a Profile, because what a missing harness has to be
/// named against is the Pairing that wants it: two roles can want two harnesses,
/// and a sentence that said only *Claude Code is not over there* would leave the
/// human to work out which picker to go and change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum PairingRole {
    Grilling,
    Implementation,
    Review,
}

/// One thing the far end has not got.
///
/// Each names the *somewhere* it is to be put right, because each is a different
/// errand: a repository to open over there, a harness to install over there, or
/// a machine to go and wake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Lacking {
    /// The device did not answer, so it said nothing about itself at all.
    ///
    /// The whole of a preflight where it is in one: a machine that is asleep has
    /// not *failed* any of the questions below, and listing them under it would
    /// be this device inventing findings out of a silence.
    Unreachable,

    /// No Repo on that device is this repository — see
    /// [`RepoAcross`](crate::RepoAcross) for what *is* means.
    Repo {
        /// What this device calls the repository, which is its directory's own
        /// name and the only name there is to point at.
        name: String,

        /// Whether it is one of the Conversation's Companions rather than the
        /// repository the work itself is in. Both are refusals; they are not
        /// the same sentence.
        companion: bool,
    },

    /// The harness a Pairing names is not on that device's `PATH`.
    Harness {
        role: PairingRole,

        /// The Agent Profile's name as it reads here, or nothing for the one
        /// account on its harness that nobody named.
        profile: Option<String>,

        agent_type: AgentType,
    },
}

/// What one device lacks, as the device holding the work found it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Preflight {
    /// The device it was asked of, by the name the human gave that machine —
    /// which is what every one of these findings has to be said against, rather
    /// than sixteen bytes of hex.
    pub device: String,

    /// Everything in the way, in the order a move would meet it: the machine,
    /// then the repositories, then the harnesses. Empty is a device the work can
    /// go to.
    pub lacks: Vec<Lacking>,
}

/// Whether one harness is on a device's `PATH`, as that device answers for
/// itself.
///
/// **Every harness rather than the ones asked after**, which is what makes this
/// a reading of a machine rather than a question with an answer shaped to it:
/// four rows is the whole list, the device that asked picks out the ones its
/// Pairings name, and nothing has to be sent over before anything can be read
/// back.
///
/// Not a viewer type: no browser draws this, and what a browser is drawn is the
/// [`Preflight`] composed out of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessThere {
    pub agent_type: AgentType,

    /// Whether a session of that type could be launched there at all.
    pub there: bool,
}

/// What became of pressing **Go**.
///
/// Named the way the stops' and the close's answers are, and for their reason: a
/// press that quietly did nothing would leave the human waiting for work to move
/// that was never going anywhere.
///
/// **Nothing here says the work has arrived**, because nothing could: the press
/// writes down that the Conversation is going, and the move runs once the turn
/// the session is part way through has ended. What says it landed is the record
/// itself — the copy over there, and the mark on the copy here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Transferring {
    /// It is going: the request is written, whatever is running runs to its own
    /// end, nothing is started after it, and the move follows.
    Transferring,

    /// The far end is missing something after all — the reading the dialog drew
    /// a moment ago, taken again as the press arrived.
    ///
    /// **The whole reading rather than a word**, so the dialog says what is in
    /// the way in the same sentences it was already saying it in. A device that
    /// went to sleep between the drawing and the press is the ordinary way here.
    Lacking(Preflight),

    /// It is drafting or closed, so there is nothing of the kind this moves. A
    /// Draft is moved by the device select on its own composer; a Closed
    /// Conversation has no work left to move.
    NotTransferable,

    /// This copy is not the live record: the work has already been handed to
    /// another device, and what this device holds is the tombstone. What moves
    /// it now is a press over there.
    Elsewhere,

    NoSuchConversation,
}

/// A Conversation as it crosses the link: everything the far end writes its own
/// row from, and nothing about the machine it came off.
///
/// **Not a viewer type.** No browser is ever handed one of these — what a
/// browser sees of a move is the Timeline it leaves on both sides — and the two
/// ends of it are two Verksteads.
///
/// Ids are each device's own and collide by construction, so nothing here is one
/// of the sender's: the Repo is the one *this* device's registry answered for the
/// match, and each Pairing's Profile is named by the device it is at home on
/// rather than by a row number. What does travel unchanged is the pair that is
/// cluster-wide by construction — the **Rank**, which carries the device that
/// issued it, and the **birth key**, which says where the work was born.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationAcross {
    /// The key the work was born under, which every copy of it says the same.
    pub born: BirthKey,

    /// The Repo **on the receiving device** the work is in, as the sending
    /// device's matching settled it — see [`RepoAcross`](crate::RepoAcross).
    pub repo: i64,

    /// The branch the work is on, and whether the name is one somebody settled
    /// on or one Verkstead invented and a session may still replace.
    pub branch: String,
    pub branch_named: bool,
    pub naming: bool,

    /// The state it is in, which is what the far end's Resume asks its question
    /// of.
    pub state: crate::Lifecycle,

    /// Its **Rank**, verbatim.
    pub rank: String,

    /// And what each of the three roles has settled.
    pub grilling: PickedAcross,
    pub implementation: PickedAcross,
    pub review: PickedAcross,
}

/// What names a Conversation across a whole cluster: the device it was drafted
/// on, and the id it was given there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BirthKey {
    pub device: String,
    pub id: i64,
}

/// What one role has settled, as it crosses.
///
/// The three states rather than an option, for the reason the record keeps
/// three: a role picked away is a choice somebody made and an empty picker is
/// one they have not, and a copy that could not tell them apart would arrive
/// either unable to start or starting a session the human said there was to be
/// none of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickedAcross {
    Nothing,
    Skipped,
    Under(PairingAcross),
}

/// The Profile and model a role's sessions run under, as they cross.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingAcross {
    pub profile: ProfileAcross,

    /// The model paired with it, where one was — `None` being a choice made
    /// before pairings had models, which runs on the Profile's own first one
    /// wherever it is launched.
    pub model: Option<String>,
}

/// An Agent Profile named the way a cluster names one: the device it is at home
/// on, and the id it has **there**.
///
/// **Never the sender's local id.** A Profile on one machine is a row on every
/// other that has heard of it — a mirror, with a local id of its own — so the
/// pair is the one name for it that means the same thing on both ends. The
/// receiving device turns it back into an id of its own, which is its own row
/// where the account is at home there and its mirror where it is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileAcross {
    pub device: String,
    pub id: i64,
}

/// The word that the move is over, and which machine the work came off.
///
/// **Sent last of all, after the sending device has marked its own copy**, which
/// is what makes it the one call the far end may act on. Every leg before it
/// stands in front of the commit point: a move that falls over there is swept,
/// and a session started in a checkout that was about to be taken back would be
/// an agent working in a directory nobody could account for. By the time this
/// arrives nothing is going to be swept, so what the far end does about it is
/// press Resume for itself and say on its Timeline where the work came from.
///
/// **The Device Id in the body rather than read off the certificate**, which is
/// how every other call over this link names a device — see [`ProfileAcross`].
/// The gate has already settled that the caller is a member; this says *which*
/// member, and what it is for is a sentence with a machine's name in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameFrom {
    /// The **Device Id** of the machine the work was moved off.
    pub device: String,
}

/// What the receiving device answers: the id it numbered its copy.
///
/// Which is what the sending device writes on its own copy as the mark saying
/// where the live record now is — and what makes that answer the **commit
/// point** of the whole move. Until it arrives nothing has changed on the
/// sending side, and the work is still being done there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrived {
    pub id: i64,
}

/// What a device already holds of one repository: the commit ids at its refs.
///
/// **The negotiation the bundle is packed against.** A machine that cloned the
/// same repository yesterday holds nearly everything the branch stands on, so
/// what has to cross is the branch and not the history under it — and the only
/// end that can say what it holds is the far end. See
/// [`CheckoutAcross::bundle`].
///
/// **The tips rather than the refs.** What a bundle's prerequisites are is
/// commits, and a ref's *name* on the far end is that machine's own business —
/// a branch of somebody else's work there is as good a prerequisite as any, and
/// naming it would be this device reading somebody's branch list for nothing.
///
/// **And the most recently moved first**, bounded rather than whole: a
/// repository worked in for years holds thousands of refs, nearly all of them at
/// commits some other tip already reaches, and the ones a transfer is packed
/// against are the ones somebody has touched. Past the bound the bundle carries
/// a little more history than it strictly had to, which costs bytes rather than
/// correctness.
///
/// Not a viewer type: no browser draws this, and the device that asks for it
/// hands it to git rather than to a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TipsThere {
    pub tips: Vec<String>,
}

/// A Conversation's checkout as it crosses the link: the branch, and whatever
/// was uncommitted in the Worktree it was being worked in.
///
/// **Not a viewer type**, for [`ConversationAcross`]'s reason: the two ends of
/// it are two Verksteads, and what a browser sees of a move is the Timeline it
/// leaves on both sides.
///
/// **And no path of the sending machine is in it.** The far end names its own
/// Worktree under its own Data Directory — the way the memory sync names its
/// own parts — so what travels is a branch, some commits and some bytes, and
/// nothing that could be joined onto a directory over there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutAcross {
    /// The branch the checkout is **actually** on, which is not always the one
    /// the record was written with: a session renames the branch it was given
    /// where the name was Verkstead's own, and what goes over is the name it
    /// now answers to.
    pub branch: String,

    /// Where that branch stands, so the far end can say whether the bundle put
    /// it where it was — and so a bundle that was not needed at all still
    /// leaves a branch behind.
    pub commit: String,

    /// What the branch was cut from, carried because it is a fact about the
    /// work rather than about the machine: the commit, and the branch it was
    /// resolved off where the record kept one.
    ///
    /// What reads it is the far end's commit sweep, which leaves out everything
    /// the base already holds. A copy that arrived without it would report the
    /// history under the branch as the Conversation's own work.
    pub base_commit: Option<String>,
    pub base_ref: Option<String>,

    /// The branch as a git bundle, base64 — packed against exactly the tips the
    /// far end said it held, so a machine that has the history under the work
    /// is sent the work.
    ///
    /// `None` where the far end already holds every commit of the branch, which
    /// is a bundle git refuses to make rather than an empty one: what is left to
    /// do over there is the ref, and [`Self::commit`] is what it is written at.
    pub bundle: Option<String>,

    /// The tracked changes, as `git diff --binary` against `HEAD`, base64.
    ///
    /// **Binary because a Worktree holds binaries**: a changed image or a
    /// compiled fixture is a change somebody made, and the prose diff a Question
    /// Set carries leaves it out by design. `None` is a tree with nothing
    /// uncommitted in it.
    ///
    /// The line endings need nothing done to them: a binary patch is written in
    /// index form and `git apply` re-applies whatever working-tree convention
    /// the far end keeps, so a tree with `core.autocrlf` on and one with it off
    /// exchange patches in both directions.
    pub patch: Option<String>,

    /// And every untracked file git does not ignore, by its path relative to
    /// the Worktree.
    ///
    /// Ignored files stay behind and the far end builds its own — a `target/`
    /// is the far end's to make, and carrying one would be carrying a build for
    /// a machine that may not even be the same operating system.
    pub untracked: Vec<UntrackedFile>,

    /// And the same leg again for each **Companion Repo**, where a
    /// Conversation's work is in more than one repository.
    ///
    /// **In the one message rather than a call apiece**, because a Conversation
    /// whose own checkout landed and whose Companion's did not is one no session
    /// could be launched in: the far end makes every checkout or none, which is
    /// what a grill start on one machine does.
    ///
    /// Empty for the ordinary Conversation, which has no Companions.
    pub companions: Vec<CompanionCheckoutAcross>,
}

/// One Companion's checkout on the way across.
///
/// **Which Companion it is, is said as a Repo of the receiving device's** — the
/// match the whole cluster runs on, settled by the end that is going to act on
/// it, exactly as the Conversation's own Repo crosses. The far end reads the
/// *mode* off its own `companions` row, which arrived with the record a leg
/// earlier, and a mode is what decides how a sandbox binds the directory.
///
/// **And the two kinds carry different things.** A read-write Companion is a
/// repository a session commits in and leaves uncommitted work in, so it carries
/// the whole of what the Conversation's own does: a branch, a bundle, a patch and
/// its untracked files. A read-only one is checked out detached and bound
/// read-only — nothing to commit and so nothing uncommitted — so it carries its
/// commit and nothing else. Carrying a patch to one would be carrying changes a
/// session was never able to make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionCheckoutAcross {
    /// The Repo **on the receiving device** this Companion is, as the sending
    /// device's matching settled it — see [`RepoAcross`](crate::RepoAcross).
    pub repo: i64,

    /// The branch its checkout is on, or `None` for a read-only Companion,
    /// which holds no branch at all.
    ///
    /// Read off the checkout rather than off the row, which is
    /// [`CheckoutAcross::branch`]'s rule and the same fact by it: a Companion on
    /// the empty *mirroring* setting takes its name from the Conversation's, so a
    /// session's rename in the last turn is a record one name behind at
    /// precisely the moment the work goes.
    pub branch: Option<String>,

    /// Where it stands: the branch's tip for a read-write Companion, and the
    /// commit a read-only one is detached at.
    pub commit: String,

    /// What its checkout was cut from, where the record knows — a Companion's
    /// base is a *name* on its row and a name moves, so the commit that name came
    /// to when the checkout was made is the only thing that ever recorded it.
    pub base_commit: Option<String>,

    /// Its branch as a git bundle, base64, packed against the tips the far end
    /// holds of *that* repository — [`CheckoutAcross::bundle`]'s rule, asked of
    /// a Companion. `None` for a read-only one, and for a far end that holds
    /// every commit of the branch already.
    pub bundle: Option<String>,

    /// Its tracked changes as a binary patch, base64. `None` for a read-only
    /// Companion, which has none to have.
    pub patch: Option<String>,

    /// And its untracked unignored files. Empty for a read-only Companion, for
    /// that reason again.
    pub untracked: Vec<UntrackedFile>,
}

/// One untracked file on the way across.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntrackedFile {
    /// Its path relative to the Worktree, spelled git's way — `/` between the
    /// segments on every platform, which is what makes one written on Windows
    /// land on a Unix and back.
    pub path: String,

    /// Its bytes, base64 — the way an attachment crosses, and for the same
    /// reason: the envelope is JSON and an untracked file is as likely to be a
    /// screenshot as a note.
    ///
    /// Converted by neither end, which is right for a file git is not tracking:
    /// there is no attribute to read and no index form to write, so what arrives
    /// is what was there.
    pub bytes: String,
}
