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

use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Which agent a session runs: the id of its harness, as the server's harness
/// list gives it — see [`HarnessEntry`].
///
/// **A string rather than a closed list** (ADR-0030, *Agent*): a harness is a
/// Plugin of the Agent Category, so the ids are whatever this server can run
/// and whatever a Profile written elsewhere names. The five built-ins are
/// spelled as they always were on the wire — `Claude`, `Codex`, `Grok`,
/// `OpenCode`, `Agy` — so that a device still on an older release reads what
/// this one sends it; a plugin harness is its plugin's uuid.
///
/// Its own type because the account is not always there to carry it — a
/// Timeline Event says which agent ran a session and holds no account at all,
/// the Profile it was launched from being a thing that can since have gone.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AgentType(pub Cow<'static, str>);

impl AgentType {
    pub const CLAUDE: Self = Self(Cow::Borrowed("Claude"));
    pub const CODEX: Self = Self(Cow::Borrowed("Codex"));
    pub const GROK: Self = Self(Cow::Borrowed("Grok"));
    pub const OPENCODE: Self = Self(Cow::Borrowed("OpenCode"));
    pub const AGY: Self = Self(Cow::Borrowed("Agy"));

    /// The id, as the wire spells it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The account a Profile names: which harness runs it, and each of the paths
/// that harness declares an account to be made of, under its key.
///
/// **One shape for every harness**, the form drawing a field per path the
/// harness list declares — see [`HarnessPath`] — rather than a shape per type:
/// Claude Code's pair is two declared paths, `claude_dir` and `config_file`,
/// and every built-in after it one, `home`.
///
/// One shape for both directions. A Profile as the viewer receives it carries
/// the resolved paths the server recorded and a Profile as the human has just
/// written it carries what they typed, but they are the same fields either way,
/// and two types for one shape would be two opinions about what an account is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileAccount {
    /// Which harness runs it.
    pub agent_type: AgentType,

    /// Each path under the key its harness declares it by. Optional per key
    /// to the viewer, which reads them by the keys a harness declares and may
    /// hold an account of a harness that declares others.
    #[cfg_attr(feature = "typescript", ts(type = "{ [key in string]?: string }"))]
    pub paths: BTreeMap<String, String>,
}

impl ProfileAccount {
    /// Claude Code's pair.
    pub fn claude(claude_dir: impl Into<String>, config_file: impl Into<String>) -> Self {
        Self {
            agent_type: AgentType::CLAUDE,
            paths: BTreeMap::from([
                ("claude_dir".to_owned(), claude_dir.into()),
                ("config_file".to_owned(), config_file.into()),
            ]),
        }
    }

    /// The one home of a built-in harness that keeps its account under one.
    pub fn home(agent_type: AgentType, home: impl Into<String>) -> Self {
        Self {
            agent_type,
            paths: BTreeMap::from([("home".to_owned(), home.into())]),
        }
    }

    /// Which agent runs this account.
    pub fn agent_type(&self) -> AgentType {
        self.agent_type.clone()
    }
}

/// One harness this server can run a session on, as the Profile form, the
/// cards and the pickers draw it: everything about it the viewer used to keep
/// a table of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct HarnessEntry {
    /// The id a Profile of it names.
    pub id: AgentType,

    /// What it is called where a sentence names it: *Claude Code*.
    pub name: String,

    /// Its mark, as the SVG document it ships. Drawn as an image rather than
    /// put into the page, so that nothing in it runs.
    pub mark: String,

    /// The paths an account of it is made of, in the order the form draws
    /// them.
    pub paths: Vec<HarnessPath>,

    /// What its login calls it — *Claude*, *Antigravity* — where Verkstead
    /// runs its login, so that a Profile card offers one; `null` where it does
    /// not.
    pub login: Option<String>,

    /// Where an account of it is kept in a home, said from `~` as the human
    /// would recognise it: what onboarding draws under an account it found.
    pub found_at: Vec<String>,

    /// What to run in a terminal to make an account of it.
    pub first_run: String,

    /// The models a Profile of it is offered, in the order the form offers
    /// them.
    pub models: Vec<HarnessModel>,

    /// The short names it answers to beyond those — `opus` — each with what
    /// it reads as: recognised where a Profile holds one, never offered.
    pub aliases: Vec<HarnessModel>,
}

/// One model a harness declares: what it travels as, and what a human reads
/// instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct HarnessModel {
    /// The id the harness is launched with, and the string a Profile carries.
    pub id: String,

    /// What it reads as.
    pub name: String,
}

/// One path an account of a harness is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct HarnessPath {
    /// The key it is kept under in a [`ProfileAccount`].
    pub key: String,

    /// What the form labels its field.
    pub label: String,

    /// What it is for, said under the field, with what is to be read as code
    /// between backticks.
    pub purpose: String,

    /// What the field shows before anything is typed into it.
    pub placeholder: String,

    /// Whether it is a file rather than a directory, which is what the field's
    /// picker offers.
    pub file: bool,
}

/// Why a saved Profile cannot be run under as things stand.
///
/// Not a way of being saved: every Profile here passed the same checks when it
/// was written down. This is what has become of its account since — the pair
/// for a Claude Profile, and the one home for every type that keeps one.
///
/// **And two of them are a Profile's not bound here**, whose account is on
/// other machines. None of their paths is judged here — they belong to no
/// filesystem this device can read — so what is asked instead is the two things
/// that would stop a launch from a copy, in the order a launch would meet them:
/// a bound device holding a login answering at all, and the harness to run it
/// on this machine. **Per device**: every device answers them for itself.
///
/// **Refused rather than hidden.** A row that says why it cannot be run is
/// something to go and put right, and a row quietly missing is a human looking
/// for a Profile they know they saved. So none of them takes a row out of
/// a picker, and a Pairing made against one goes on reading as a Pairing — with
/// a broken Profile in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Broken {
    /// The claude directory is not there any more.
    DirMissing,

    /// The config file is not there any more.
    ConfigMissing,

    /// The home the account was kept under is not there any more.
    HomeMissing,

    /// Not bound on this device, and no device it is bound on whose binding
    /// holds a login is answering — so there is nowhere to fetch the account
    /// from, and no root here can be built out of it.
    ///
    /// Names no device: what is missing is *a* source, and with more than one
    /// binding any of them would do. A device bound on it whose account holds
    /// no login file is no source either — there is nothing in it to copy.
    NoBoundDevice,

    /// The harness this Profile runs is not one this server has: a plugin
    /// installed on the device that saved it and not here, or since removed.
    /// Not offered on a picker — there is nothing here to launch it with, and
    /// nothing to name it by but its id.
    HarnessUnknown,

    /// The harness this Profile runs is not on *this* machine, whatever is true
    /// of it on the devices it is bound on.
    ///
    /// The onboarding probe's own finding, asked of the program a session of
    /// this type is launched as — so a row reads it in that step's own word
    /// rather than in a second vocabulary for one fact.
    HarnessMissing,

    /// The account bound here has no login: `claude auth status` or `agy
    /// models` said so when a press was about to start a session under it.
    ///
    /// Remembered from that press rather than asked at every read, the asking
    /// being a run of the harness. It goes when a login from the card lands,
    /// or when the next press finds the account logged in after all.
    SignedOut,
}

/// Why a press that would have started a session was refused over a Profile:
/// what is wrong with it, and what it runs.
///
/// **Enough to say the sentence the row says.** Every one of the readings above
/// is drawn on the Profile's own row, and a refusal saying only *a chosen
/// profile is broken* would be a second, vaguer account of something the human
/// is already being told precisely. So what travels is the two facts that
/// sentence is composed out of, and the viewer composes it once for both. No
/// device is among them: a Profile is the cluster's, and no finding names one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileTrouble {
    /// What is wrong with it.
    pub broken: Broken,

    /// Which harness it runs, for the trouble that is about the harness.
    pub agent_type: AgentType,
}

/// One row of the Profile list.
///
/// The account's paths are the resolved ones the server recorded rather than
/// whatever was typed to save them: those are what will be bind-mounted, so
/// those are what is worth showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEntry {
    /// The uuid it is known by, the same on every device that holds it.
    pub id: String,

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

    /// `null` while the account is where it was left and there is nothing in
    /// the way of running a session under it here, which is the ordinary case.
    pub broken: Option<Broken>,

    /// Whether the account this row names holds a **login file**, which is what
    /// says the Profile can be used from a device it is not bound on.
    ///
    /// Answered by the devices the account is on: one bound here is looked at
    /// here, and one bound elsewhere carries what its bindings last said. A
    /// login kept somewhere that is not a file — the macOS Keychain — leaves
    /// nothing for another device to copy, and so does a sign-out; either way
    /// the account is still perfectly runnable on the machine it is on, which
    /// is why this is a fact beside [`Self::broken`] rather than a way of being
    /// it. One not bound here with no login on any bound device it can reach is
    /// broken, and that is [`Broken::NoBoundDevice`].
    pub login: bool,

    /// Whether a session under this Profile shares the account's memory store,
    /// or starts with an empty one of its own. On unless the human switched it
    /// off.
    pub memory: bool,

    /// Whether this device holds a **binding** of it: its own account's paths,
    /// which [`Self::account`] then is and a session here is run as. `false`
    /// is a Profile bound on other devices alone, whose paths on the row are
    /// one of theirs and belong to no filesystem here.
    ///
    /// No device is drawn on the row's own line either way: a Profile is the
    /// cluster's, and has no home to mark. Where it is bound is
    /// [`Self::devices`].
    pub bound: bool,

    /// How each device of the cluster runs it, a line apiece: this device
    /// first, then every member in the order the Devices section lists them —
    /// see [`ProfileDevice`].
    ///
    /// **Empty on a Verkstead in no cluster**, whose one device is the only
    /// place anything could be bound and says nothing a line would. A member
    /// not answering keeps its line, drawn from the last binding of it this
    /// device holds; one unlinked loses it, whatever binding it left behind.
    pub devices: Vec<ProfileDevice>,
}

/// One device's line on a Profile card: the device, and whether it holds a
/// **binding** of the Profile.
///
/// Every device draws every device's line off what it holds, the bindings
/// replicating like the records they bind (ADR-0028). A binding is only ever
/// written by the device it names, so the paths on another device's line are
/// that device's to change and nobody else's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileDevice {
    /// Which device, drawn in full: its name, the mark for its OS and whether
    /// it is answering. The `id` is `null` on this device's own line.
    pub device: crate::RowDevice,

    /// The account its binding names, in the harness's own shape, or `null`
    /// for a device holding no binding — one that **runs from a copy** of the
    /// account under its own Data Directory.
    pub bound: Option<ProfileAccount>,
}

/// One file of a Profile's account as a member across the link is handed it: a
/// file a **Built Root** is made out of, and what is in it.
///
/// **The allowlist and nothing beside it.** A session that runs from a copy is
/// given a root built out of a **mirror** of its account, so what travels is
/// what a root is made from — the login, and what the written configuration is
/// composed from — and nothing else of the account: no plugins, hooks, rules,
/// skills, global instructions file, history, or any other repository's
/// transcripts. The memory store travels on its own terms and is not on this
/// list.
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
    /// The harness's own shape rather than wherever the Profile's binding on the
    /// answering device points at: a mirror is a home the device that fetched it
    /// made, and what a root is built out of there is the account as that harness
    /// keeps one.
    pub inside: String,

    /// What is in it, or nothing where the account has no such file — a Claude
    /// login kept in the macOS Keychain, an account nothing has logged in to
    /// yet.
    pub text: Option<String>,
}

/// And the **login** going the other way: what a session that ran from a copy
/// left in it, on its way into the account it belongs to.
///
/// **The one file a session genuinely changes.** A harness refreshes its OAuth
/// pair as it works, so an account lent to another device and never written back
/// would be an account signing itself out a session at a time — and everything
/// else a root holds is either Verkstead's own or the human's own and travels in
/// one direction only. So this is the whole of what comes back: the login,
/// whole, as the harness left it.
///
/// **Sent only where it changed.** A login the session wrote through or replaced
/// comes back; one that is still exactly what came down is not sent at all; and
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
/// The account says which harness it is, because the paths beside it are the
/// ones that harness declares. The form picks one from the harnesses the server
/// lists, and a harness it has not got is refused as
/// [`ProfileSaved::NoSuchHarness`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEdit {
    /// What to call it, or `null` to leave it unnamed — which is what the form
    /// sends for an empty box. A harness takes one unnamed Profile.
    pub name: Option<String>,

    /// The absolute paths this Profile's account is, under the keys its
    /// harness declares.
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

    /// The harness it names is not one this server has.
    NoSuchHarness,
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
/// the size lint is about — a Profile's own fields, its account's paths and its
/// list of models among them, are what make it big. Left as it is: these are
/// made one at a time on the way to being serialized, none of them is held in a
/// collection, and a `Box` in a view type would be a pointer written into the
/// shape the viewer is generated from for a few bytes nobody is counting.
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
    /// The Profile, by its uuid.
    pub profile_id: String,

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

/// What a device a Profile is not bound on asks a device it is bound on for, of
/// that Profile's **memory store** (ADR-0020, *Shared Profiles*).
///
/// **Facts about this device's side, rather than paths for that one to use.** A
/// memory store is keyed by the path a session ran in wherever it is keyed by
/// anything, and every one of those paths is a different string on every
/// machine — so what travels is what each end needs to name *its own*: which of
/// its Repos this repository is, what this Worktree is called, and what
/// Verkstead named the sessions of this Conversation. The device that answers
/// joins each onto its own directories, and the device that asked joins them
/// onto its own; neither ever writes a path the other chose. See
/// [`MemoryFile`], which is what comes back.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryWanted {
    /// Which of the answering device's Repos is this repository, by the id it
    /// has **there** — the answer of the match across devices.
    ///
    /// `None` is *no match*, which is an answer rather than a failure: that
    /// device holds nothing that is this repository, so there is nothing of the
    /// Repo's memory to carry either way. The Worktree's half still travels,
    /// being named off the Data Directory rather than off the Repo.
    pub repo: Option<i64>,

    /// And what this Conversation's Worktree directory is called, which is the
    /// stem both machines name theirs with — the Repo and the branch, as
    /// Verkstead spells a directory name.
    ///
    /// The answering device joins it onto its own worktrees directory, which is
    /// the path it *would* have used for this work: a Worktree lives under the
    /// Data Directory rather than under the Repo, so there is nothing to match
    /// it by, and naming it that machine's way is what leaves the transcript
    /// findable when the work comes home.
    pub worktree: Option<String>,

    /// And every session id Verkstead has given this Conversation, for the one
    /// harness whose store files a session's directory under it.
    pub sessions: Vec<String>,
}

/// One file of a memory store on its way across the link.
///
/// **Under a label rather than under a path.** The part of the store it belongs
/// to is named by a word both machines hold to — the Repo's entry, the
/// Worktree's, the sessions, the memory files, the data store — and each of
/// them joins that word onto a path of its own. `inside` is where the file sits
/// under that part, which is the one piece of a path that is the same on both
/// machines.
///
/// **Bytes rather than text**, base64 as this travels: three of the four stores
/// hold a SQLite database, and a store carried as text is a store that arrives
/// broken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryFile {
    /// Which part of the store it is in.
    pub part: String,

    /// And where it is under that part, with forward slashes whichever machine
    /// composed it — `index.sqlite`, `2026/09/29/rollout-….jsonl`.
    pub inside: String,

    /// What is in it, base64 with padding, as the standard alphabet spells it.
    pub bytes: String,
}

/// And what a session that ran from a copy left in its memory store, on its way
/// into the account it belongs to.
///
/// **The question comes back with the answer.** The device it is bound on names
/// its own path for each part of the store, and it names them off exactly what
/// it was asked the first time — which of its Repos this repository is, what
/// this Worktree is called — so a write-back carrying a different question would
/// be writing into a different part of the store than the one it read.
///
/// **Only what changed is in it.** A file still exactly as it came down is not
/// here, and neither is one the session took away: nothing is deleted where it
/// is bound, the rest of that account's store being somebody else's work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryLeft {
    /// The same question the store was read with.
    pub wanted: MemoryWanted,

    /// And the files, each under the part it belongs to.
    pub files: Vec<MemoryFile>,
}
