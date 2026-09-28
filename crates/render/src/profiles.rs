//! Agent Profiles as the viewer receives them, and what it sends to manage one.
//!
//! A row carries whether the Profile is **broken**, which is a fact about the
//! filesystem now rather than about what was saved: the pair was there when it
//! was checked at save time, and a directory can be moved afterwards. Answered
//! by the server on every read, because the server is the side that can look —
//! and because a session launched under a Profile whose pair has gone is a
//! failure with nobody watching, which is what this whole stage is arranged to
//! move forward in time.
//!
//! Every way of being refused a save is a named outcome rather than a status
//! code, as registering a Repo is: each is a different sentence to put in front
//! of the human, and each names which of the two paths it is about — pointing
//! the config field at a directory is an easy mistake, and "that path is wrong"
//! would not say which one.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Which agent a session runs, on its own rather than as an account's shape.
///
/// The same four words [`ProfileAccount`]'s discriminator is written in, so a
/// viewer narrowing on one narrows on the other: what a Profile runs and what a
/// finished session ran are the same fact about the same set of backends, and
/// two spellings of it would be two things for the viewer to keep in step.
///
/// Its own type because the account is not always there to carry it — a Timeline
/// Event says which agent ran a session and holds no account at all, the
/// Profile it was launched from being a thing that can since have gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum AgentType {
    Claude,
    Codex,
    Grok,
    OpenCode,
}

/// The account a Profile names, in the shape the agent type running it keeps
/// one.
///
/// Flat on the wire — `{"agent_type": "Claude", "claude_dir": "…",
/// "config_file": "…"}` — so the type is a field the viewer can read and narrow
/// on rather than a name it has to unwrap the account out of. Which is what the
/// form draws its fields off: a shape per type, and adding a backend adds a
/// variant here and the fields beside it.
///
/// One shape for both directions. A Profile as the viewer receives it carries
/// the resolved paths the server recorded and a Profile as the human has just
/// written it carries what they typed, but they are the same fields either way,
/// and two types for one shape would be two opinions about what an account is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "agent_type")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileAccount {
    /// Claude Code's pair: the directory a session's `~/.claude` is built from,
    /// and the file its `~/.claude.json` is copied from.
    Claude {
        claude_dir: String,
        config_file: String,
    },

    /// Codex's one home: what is bind-mounted over `~/.codex`, and the whole of
    /// what a Codex Profile names.
    Codex { home: String },

    /// And Grok Build's, bind-mounted over `~/.grok`: the same one-home shape
    /// Codex's is, under the directory grok keeps an account in.
    Grok { home: String },

    /// And OpenCode's, which is one home again but not one mount: opencode
    /// keeps no dot-directory of its own, so what this names is a home holding
    /// `.config/opencode` and `.local/share/opencode`, each bound where
    /// opencode's XDG defaults look for it inside the sandbox.
    OpenCode { home: String },
}

/// Why a saved Profile cannot be run under as things stand.
///
/// Not a way of being saved: every Profile here passed the same checks when it
/// was written down. This is what has become of its account since — the pair
/// for a Claude Profile, and the one home for every type that keeps one.
///
/// **Nothing at all for a mirror.** A Profile at home on another device names
/// paths on that machine and is judged against none of them here: what a session
/// away from home is given is a mirror of the account, fetched from the home
/// device before each launch, so the row is one to run under and there is nothing
/// about this filesystem to say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Broken {
    /// The claude directory is not there any more.
    DirMissing,

    /// The config file is not there any more.
    ConfigMissing,

    /// The home the account was kept under is not there any more.
    HomeMissing,
}

/// One row of the Profile list.
///
/// The account's paths are the resolved ones the server recorded rather than
/// whatever was typed to save them: those are what will be bind-mounted, so
/// those are what is worth showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEntry {
    pub id: i64,

    /// What the human calls this account, and `null` where they have called it
    /// nothing.
    ///
    /// A name tells two accounts of one harness apart, which is the rare case;
    /// the ordinary one is an account the harness's mark and the model already
    /// say the whole of. So the viewer says nothing where nothing has to be
    /// said, and *Default* where a name has to be shown.
    pub name: Option<String>,

    /// Which agent this Profile runs, and the account it runs as — one field,
    /// because the type is what says which fields the account has.
    pub account: ProfileAccount,

    /// Every model this account can run a session on. At least one, and none of
    /// them preferred over the others: the list says what is available and
    /// nothing more.
    pub models: Vec<String>,

    /// `null` while the account is where it was left, which is the ordinary
    /// case.
    pub broken: Option<Broken>,

    /// Whether a session under this Profile shares the account's memory store,
    /// or starts with an empty one of its own. On unless the human switched it
    /// off.
    pub memory: bool,

    /// Which device this Profile is at home on, and `null` for this one's own —
    /// which is every Profile on a Verkstead that is linked to nothing.
    ///
    /// The same shape a Conversation's row carries whose work it is, and for the
    /// same reason: the name and the mark for the operating system are this
    /// device's own reading of its membership rather than anything the far end
    /// said, and one shape means one way of drawing a device wherever a list has
    /// several machines' rows in it. See [`crate::RowDevice`].
    ///
    /// One list rather than a section per device: a Profile is a Profile, and
    /// which machine its account sits on is a fact on the row.
    pub device: Option<crate::RowDevice>,
}

/// One file of a Profile's account as a member across the link is handed it: a
/// file a **Built Root** is made out of, and what is in it.
///
/// **The allowlist and nothing beside it.** A session away from home is given a
/// root built out of a **mirror** of its account, so what travels is what a root
/// is made from — the login, and what the written configuration is composed from
/// — and nothing else of the account: no plugins, hooks, rules, skills, global
/// instructions file, history, or any other repository's transcripts. The memory
/// store travels on its own terms and is not on this list.
///
/// **Every path a mirror can hold is answered**, whether or not the account has
/// that file: an account with no login says so with no text at all, which is what
/// tells a mirror to take away the login it was holding rather than to leave a
/// stale one standing.
///
/// **Text rather than bytes**, every file here being one a harness writes as JSON
/// or as TOML. What a file no harness wrote holds is not a login, and it travels
/// as nothing rather than as something a mirror would write in a login's place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountFile {
    /// Where the file sits inside the home an account of this harness is kept
    /// in, with forward slashes: `.claude/.credentials.json`,
    /// `.codex/config.toml`, `.local/share/opencode/auth.json`.
    ///
    /// The harness's own shape rather than wherever the home device's Profile
    /// points at: a mirror is a home the device that fetched it made, and what a
    /// root is built out of there is the account as that harness keeps one.
    pub inside: String,

    /// What is in it, or nothing where the account has no such file — a Claude
    /// login kept in the macOS Keychain, an account nothing has logged in to
    /// yet.
    pub text: Option<String>,
}

/// And the **login** going the other way: what a session away from home left in
/// it, on its way into the account it belongs to.
///
/// **The one file a session genuinely changes.** A harness refreshes its OAuth
/// pair as it works, so an account lent to another device and never written back
/// would be an account signing itself out a session at a time — and everything
/// else a root holds is either Verkstead's own or the human's own and travels in
/// one direction only. So this is the whole of what comes home: the login, whole,
/// as the harness left it.
///
/// **Sent only where it changed.** A login the session wrote through or replaced
/// comes home; one that is still exactly what came down is not sent at all; and
/// one made where the account had none is handed over. Which is the same three
/// cases a local session's ending already tells apart, and the reason this is a
/// press rather than a file kept in step.
///
/// **Last write wins.** Two machines refreshing one login may sign one of them
/// out; nothing is merged and nothing is locked, and the write that arrives later
/// is the one the account keeps (ADR-0020, *Shared Profiles*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLogin {
    /// What the harness left in the login file, whole and as it wrote it.
    pub text: String,
}

/// A Profile as the human has just written it, for saving or for rewriting.
///
/// The account says which type it is, because the fields beside it are that
/// type's. The form picks one from the types whose stage has landed — a type
/// that cannot launch the real binary yet would be a lie in a picker — so a
/// type this knows about may still be one nothing arrives as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEdit {
    /// What to call it, or `null` to leave it unnamed — which is what the form
    /// sends for an empty box. A harness takes one unnamed Profile.
    pub name: Option<String>,

    /// The absolute paths this Profile's account is, in its type's shape.
    pub account: ProfileAccount,

    /// The models this account can run a session on, in the order they were
    /// typed. The form takes them a line apiece; blank lines and repeated
    /// whitespace are the server's to drop, and a list that comes to nothing is
    /// refused.
    pub models: Vec<String>,

    /// Whether a session under this Profile shares the account's memory store:
    /// `true` for the store the human's own sessions keep, `false` for an empty
    /// one of the session's own. Left out, it is on — the default the form
    /// draws, and what every Profile had before there was a switch.
    #[serde(default = "memory_on")]
    pub memory: bool,
}

/// What [`ProfileEdit::memory`] is when a request leaves it out.
fn memory_on() -> bool {
    true
}

/// What became of saving a Profile.
///
/// The refusals are the server's and not the form's: a check the browser made
/// is a courtesy, and every request reaching the endpoint is decided there
/// whether or not a form was involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileSaved {
    /// Recorded. It is on the list, and it is there after a restart.
    Saved,

    /// There is no Profile with that id to rewrite.
    NoSuchProfile,

    /// It was given no models. A session has to know what it runs on, and a
    /// Profile naming none is one nothing could be launched under.
    Modelless,

    /// Another Profile is called that already.
    NameTaken,

    /// That harness already has a Profile nobody named. A name is what tells two
    /// accounts of one harness apart, so the second of them has to have one.
    DefaultTaken,

    /// The claude directory was named relatively. There is nothing to resolve it
    /// against that would mean the same thing twice.
    DirNotAbsolute,

    /// Nothing is at the claude directory's path.
    DirMissing,

    /// Something is there and it is not a directory — `~/.claude` is a directory
    /// bind-mounted over, so a file cannot stand in for it.
    NotADirectory,

    /// The config file was named relatively.
    ConfigNotAbsolute,

    /// Nothing is at the config file's path.
    ConfigMissing,

    /// Something is there and it is not a file — the pair is a directory and a
    /// file, and this is the file half.
    NotAFile,

    /// The home was named relatively. There is nothing to resolve it against
    /// that would mean the same thing twice.
    HomeNotAbsolute,

    /// Nothing is at the home's path.
    HomeMissing,

    /// Something is there and it is not a directory — a home is a directory
    /// bind-mounted over, so a file cannot stand in for it.
    HomeNotADirectory,
}

/// What became of removing a Profile.
///
/// Two answers, and neither of them is a refusal about what is using it: a
/// Profile is always the human's to take away, and the Conversations that had
/// chosen it are nulled out rather than standing in the way. The only thing
/// that can go wrong is asking about one that has already gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileDeleted {
    Removed,
    NoSuchProfile,
}

/// One of a Conversation's Pairings, as the page shows it: the Profile
/// whole, and the model paired with it.
///
/// The Profile whole rather than by id because the pane says what it is and
/// whether it is still runnable — and the model beside it because a Pairing is
/// both halves, and either half alone is not something to launch a session
/// with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PairingView {
    pub profile: ProfileEntry,

    /// The model of that Profile's list this Conversation's sessions run on.
    ///
    /// `null` is a Profile chosen before pairings existed, which is not a
    /// Pairing: the page draws it as nothing chosen, because while the
    /// Conversation is drafting that is a choice to make again. One past
    /// drafting keeps running on the model its Profile carried, which nothing
    /// here has to say — its Pairings are fixed and there is no picking left.
    pub model: Option<String>,
}

/// What a Conversation has settled about one of its roles, as the page shows
/// it: the Pairing its sessions run under, that the role runs none, or nothing
/// picked yet.
///
/// Three rather than a nullable Pairing, because the review picker offers *no
/// review* as a row of its own: a Conversation that picked it is as ready to
/// start as one that picked a Pairing, and a page that could not tell it from an
/// empty picker would draw the placeholder over a settled choice.
///
/// One variant carries a Profile and the other two carry nothing, which is what
/// the size lint is about — a Profile got bigger when it gained the device it is
/// at home on. Left as it is: these are made one at a time on the way to being
/// serialized, none of them is held in a collection, and a `Box` in a view type
/// would be a pointer written into the shape the viewer is generated from for a
/// few bytes nobody is counting.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum PickedView {
    /// Nothing picked — which includes a Profile chosen before pairings
    /// existed, that being half a choice and so a choice to make again.
    Nothing,

    /// The row that runs no session at all.
    Skipped,

    /// The Profile and model this role's sessions run under.
    Under(PairingView),
}

impl PickedView {
    /// The Pairing where one was picked, for the readers that want the account
    /// rather than which of the three this is.
    pub fn pairing(&self) -> Option<&PairingView> {
        match self {
            Self::Under(pairing) => Some(pairing),
            _ => None,
        }
    }

    /// Whether the human picked the row that runs no session, which is what the
    /// presses that behave differently for it turn on.
    pub fn skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// What a Repo was last grilled with, as a Conversation started on it would
/// arrive showing it — the prefill, read before there is a Conversation for it
/// to have been applied to.
///
/// The same three fields a [`crate::ConversationView`] carries and in the same
/// shapes, because that is what this is: what the pickers of a freshly created
/// draft on this Repo would say. A page filling its own pickers from this and a
/// page reading them off a draft are drawing the same answer, and one shape is
/// what keeps the two from wording it differently.
///
/// Each of them is judged before it is handed over, exactly as creation judges
/// it: a remembered Pairing whose Profile has broken, or whose Profile no
/// longer lists the model it was remembered with, comes back as nothing picked.
///
/// A Repo with no memory at all is not handed back empty: it is prefilled, role
/// by role, off the last Conversation to start work anywhere and the platform
/// default under that, each candidate judged the same way — so nothing picked
/// is what a role gets only where neither of those survives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RepoPairingsView {
    /// A Pairing, or nothing: the grilling role has no row that runs no session,
    /// so a skip some Repo remembers from before *No grilling* retired is read
    /// as nothing remembered and never handed over as a choice.
    pub grilling: Option<PairingView>,

    /// And the other role with no such row.
    pub implementation: Option<PairingView>,

    /// The one whose memory can hold it, so this says which of three rather than
    /// whether anything is remembered.
    pub review: PickedView,
}

/// Which Pairing one of a Conversation's roles runs under, or that it runs none.
///
/// `null` is the row that runs no session: a picker that offers one offers it
/// beside the Pairings, so the one press that picks either sends the same body
/// — see [`PickedView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RoleChoice {
    pub pairing: Option<ProfileChoice>,
}

/// Which Profile and model a Conversation is pairing for one of its roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileChoice {
    pub profile_id: i64,

    /// One of that Profile's models. Never absent: there is no default model
    /// anywhere, so a Pairing is picked whole or not at all.
    pub model: String,
}

/// What became of choosing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileChosen {
    Chosen,
    NoSuchConversation,

    /// There is no Profile with that id — it was removed between the list this
    /// page read and the choice it made from it.
    NoSuchProfile,

    /// That Profile does not list that model, for the same reason: its list was
    /// edited between the read and the pick.
    NoSuchModel,

    /// The Conversation is past drafting, so both its Pairings are fixed.
    NotDrafting,
}
