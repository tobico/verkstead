//! The Agent Profiles a session can be run under: which account, and which
//! models.
//!
//! A Profile is a name, an account and a list of models, because that is the
//! whole of what launching a session needs. The account is what keeps one
//! account's sessions out of another's, and it is the paths its harness
//! declares — see [`Account`]; the models are what a session may be run on.
//!
//! The models are a list and not one model, and the list is the Profile's own:
//! different Profiles reach different accounts, so each names what it can
//! actually launch rather than sharing one list nobody's account really has.
//! There is no default and no preferred entry — the list only says what is
//! available, and every pick made from it is explicit.
//!
//! **A Profile is a Replicated record** (ADR-0028) — see [`Profiles`]: its
//! name, its harness, its models and its memory switch, under a uuid that is
//! the same on every device and that everything naming a Profile names — a
//! Pairing, a Repo's memory of what it was last grilled with, a steer and
//! every Conversation. **Its account is a binding**, one per device — see
//! [`Bindings`]: this device's is what a session here is run as, and what
//! [`Profile::account`] reads.
//!
//! An account's paths are stored resolved, as a Repo's is and for the same
//! reason: whoever saved the Profile had `..` and every symlink taken out of
//! them before they arrived, so what is recorded is what the filesystem means
//! rather than what somebody typed. Whether they are of the shape their harness
//! wants is decided above the store, where the reading lives.
//!
//! A Profile's name is optional. A name is what tells two accounts of one
//! harness apart, and a harness with one account has nothing to tell apart — so
//! uniqueness is two rules rather than one, both per harness: at most one
//! unnamed Profile of a harness, and no two of one harness named alike. Both are
//! checked at the press against every live record — see [`clash`].
//!
//! **A Profile this device holds no binding of is still a Profile here**: it is
//! drawn and picked like any other, its account being another device's — see
//! [`Unbound`]. And what is left of the `profiles` table where Profiles were
//! kept before they were records is the **provisional mirrors**: a member's
//! Profile held from before the uuids under a uuid of this device's minting,
//! kept so the Pairings that name it hold until that member's records arrive
//! carrying where it was minted — see [`settle`]. It is in no list.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::replicated::{self, Kind};

/// Which coding agent a Profile runs: the id of its harness.
///
/// **A string rather than a closed list of words** (ADR-0030, *Agent*): a
/// harness is a Plugin of the Agent Category, so what a Profile names is
/// whatever harness the plugin that provides it declares. The five built into
/// this binary each have a fixed uuid — see [`BUILT_IN`] — and **are written
/// down as the word they always were**, `claude`, `codex`, `grok`, `opencode`
/// and `agy`, so that a device of the cluster still on an older release reads
/// a record of one. A plugin harness is written down as its plugin's uuid.
/// Either is read, and a built-in's uuid reads as its word.
///
/// **An id this binary does not know is still an id**: a Profile of a harness
/// that is not here — one installed on another device, or since removed — is
/// read like any other and judged broken above the store, rather than being a
/// row that stops a read.
///
/// Interned, so that it stays a value as cheap to pass around as the enum it
/// replaces: the set of harness ids a server ever meets is a handful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentType(&'static str);

/// One harness built into this binary: the word it is written down as, the
/// uuid its plugin is known by, and the keys of the paths its account is made
/// of, in the order it declares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltIn {
    pub agent_type: AgentType,
    pub uuid: &'static str,
    pub paths: &'static [&'static str],
}

/// The key of Claude Code's directory among its account's paths.
pub const CLAUDE_DIR: &str = "claude_dir";

/// The key of Claude Code's config file among its account's paths.
pub const CONFIG_FILE: &str = "config_file";

/// The key of the one home every built-in harness after Claude keeps its
/// account under.
pub const HOME: &str = "home";

/// The five harnesses built into this binary, in the order they arrived.
///
/// The keys are the binding's own field names from before an account was a
/// map of paths — see [`Binding`] — which is what lets a binding an older
/// release wrote be read as one, and one written now be read by it.
pub const BUILT_IN: [BuiltIn; 5] = [
    BuiltIn {
        agent_type: AgentType::CLAUDE,
        uuid: "985e669b-5cf2-4aee-bbf2-6074626949f4",
        paths: &[CLAUDE_DIR, CONFIG_FILE],
    },
    BuiltIn {
        agent_type: AgentType::CODEX,
        uuid: "49ea40c6-298c-412a-95b5-c89b9f539616",
        paths: &[HOME],
    },
    BuiltIn {
        agent_type: AgentType::GROK,
        uuid: "d40d8f19-7261-4ddf-9a98-539f460577a7",
        paths: &[HOME],
    },
    BuiltIn {
        agent_type: AgentType::OPENCODE,
        uuid: "8c53a219-6c1a-41ca-b127-0f9e75730f4b",
        paths: &[HOME],
    },
    BuiltIn {
        agent_type: AgentType::AGY,
        uuid: "7c2f6b59-0b54-4198-bfd2-b2fae4da2418",
        paths: &[HOME],
    },
];

impl AgentType {
    pub const CLAUDE: Self = Self("claude");
    pub const CODEX: Self = Self("codex");
    pub const GROK: Self = Self("grok");
    pub const OPENCODE: Self = Self("opencode");
    pub const AGY: Self = Self("agy");

    /// The id this harness is written down as: a built-in's word, a plugin's
    /// uuid.
    ///
    /// What the record's `harness` field holds, and the same id a sandbox
    /// carries in its environment so that `verkstead guide` printed inside one
    /// knows which harness it is being printed for. One spelling for both,
    /// because a second vocabulary would be a second thing to keep in step.
    pub fn word(self) -> &'static str {
        self.0
    }

    /// And back again, which is the only way one is ever read: a built-in's
    /// word or its uuid as that built-in, and any other id as itself.
    pub fn read(id: &str) -> Self {
        if let Some(built_in) = BUILT_IN
            .iter()
            .find(|built_in| built_in.agent_type.0 == id || built_in.uuid.eq_ignore_ascii_case(id))
        {
            return built_in.agent_type;
        }

        let mut interned = INTERNED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(held) = interned.get(id) {
            return Self(held);
        }

        let held: &'static str = Box::leak(id.to_owned().into_boxed_str());
        interned.insert(held);

        Self(held)
    }

    /// The id `id` is, where it is one already known to this process — a
    /// built-in's, or one read before — and nothing where it is not.
    ///
    /// **For an id from outside**, which [`AgentType::read`] would keep for
    /// the life of the process: asked by anyone who can reach an open route, a
    /// new id each time, that is memory nothing gives back.
    pub fn known(id: &str) -> Option<Self> {
        if let Some(built_in) = BUILT_IN
            .iter()
            .find(|built_in| built_in.agent_type.0 == id || built_in.uuid.eq_ignore_ascii_case(id))
        {
            return Some(built_in.agent_type);
        }

        INTERNED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .map(|held| Self(held))
    }

    /// The built-in this is, where it is one.
    pub fn built_in(self) -> Option<&'static BuiltIn> {
        BUILT_IN.iter().find(|built_in| built_in.agent_type == self)
    }
}

/// Every id [`AgentType::read`] has kept, each held for the life of the
/// process.
static INTERNED: std::sync::Mutex<std::collections::BTreeSet<&'static str>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

impl std::fmt::Display for AgentType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// How a backend's sessions ask, which is one of two things — which one being
/// the harness's answer, in the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// `verkstead ask` idles until the Response comes back, so the Answers are
    /// in front of the session when it goes on. What a backend that can hold a
    /// shell command open for hours does.
    Blocking,

    /// `verkstead ask` stores the Set and returns at once, the session ends its
    /// turn, and Verkstead types a line into its terminal when the Response
    /// lands — which the session answers by fetching with `verkstead answers`.
    /// What a backend whose shell tool yields after seconds does: an ask held
    /// open there is a paid model turn spent on every poll of it.
    StoreAndNudge,
}

/// The account a Profile runs as: its harness, and the **labelled paths** that
/// harness declares an account to be made of, each resolved.
///
/// Claude Code's is a pair — a directory and a file beside it, bound over
/// `~/.claude` and `~/.claude.json` — under the keys [`CLAUDE_DIR`] and
/// [`CONFIG_FILE`]. Every built-in after it keeps its whole account under one
/// relocatable home, under [`HOME`]: Codex's over `~/.codex`, Grok Build's over
/// `~/.grok`, the Antigravity CLI's over `~/.gemini` (an account only where it
/// holds `antigravity-cli/`), and OpenCode's, the home its XDG directories sit
/// inside. A plugin harness declares its own. What each path is *for* is the
/// harness's to say, in the server; here they are only keys and paths.
///
/// The paths are in the order the harness declares them, so that the first is
/// the directory the account is — see [`Account::directory`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    harness: AgentType,
    paths: Vec<(String, PathBuf)>,
}

impl Account {
    /// An account of `harness` made of `paths`, each under its key, in the
    /// order its harness declares them.
    pub fn new<K: Into<String>, P: Into<PathBuf>>(
        harness: AgentType,
        paths: impl IntoIterator<Item = (K, P)>,
    ) -> Self {
        Self {
            harness,
            paths: paths
                .into_iter()
                .map(|(key, path)| (key.into(), path.into()))
                .collect(),
        }
    }

    /// Claude Code's pair.
    pub fn claude(claude_dir: impl Into<PathBuf>, config_file: impl Into<PathBuf>) -> Self {
        Self::new(
            AgentType::CLAUDE,
            [
                (CLAUDE_DIR, claude_dir.into()),
                (CONFIG_FILE, config_file.into()),
            ],
        )
    }

    /// The one home of a harness that keeps its account under one.
    pub fn home(harness: AgentType, home: impl Into<PathBuf>) -> Self {
        Self::new(harness, [(HOME, home.into())])
    }

    /// The account's paths, in the order its harness declares them: Claude's
    /// directory and then its file, and every other built-in's one home.
    pub fn paths(&self) -> Vec<&Path> {
        self.paths.iter().map(|(_, path)| path.as_path()).collect()
    }

    /// And each under its key.
    pub fn keyed(&self) -> &[(String, PathBuf)] {
        &self.paths
    }

    /// The path under `key`, where the account has one.
    pub fn path(&self, key: &str) -> Option<&Path> {
        self.paths
            .iter()
            .find(|(held, _)| held == key)
            .map(|(_, path)| path.as_path())
    }

    /// And the first of them, which is the directory the account is: what its
    /// login and its memory are said from. The empty path for an account with
    /// none — see [`Unbound`].
    pub fn directory(&self) -> &Path {
        self.paths
            .first()
            .map(|(_, path)| path.as_path())
            .unwrap_or(Path::new(""))
    }

    /// Which harness runs this account.
    pub fn agent_type(&self) -> AgentType {
        self.harness
    }

    /// An account of `harness` with no paths in it: what a Profile carries
    /// that no device this one has heard of is bound on — see [`Unbound`]. A
    /// built-in's keys are each there and empty, which is the shape it has
    /// always had.
    fn nowhere(harness: AgentType) -> Self {
        let keys = harness
            .built_in()
            .map(|built_in| built_in.paths)
            .unwrap_or_default();

        Self::new(harness, keys.iter().map(|key| (*key, PathBuf::new())))
    }
}

/// An Agent Profile as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The **uuid** it is known by, which is what everything that names a
    /// Profile names it by — a Pairing, a Repo's memory, a steer, a transfer,
    /// a peer route (ADR-0028) — and the same on every device that holds it.
    pub id: String,

    /// What the human calls this account, where they have called it anything.
    ///
    /// Unique among the named: a picker with two `work` rows in it is a picker
    /// nobody can use. `None` is a Profile nobody typed a word for — the one
    /// account on its harness, where the harness and the model say the whole of
    /// what it is — and a harness has at most one of those, for the reason a
    /// name is unique.
    pub name: Option<String>,

    /// The account a session under this Profile is run as: the paths its
    /// harness declares.
    pub account: Account,

    /// The models this account can run a session on, in the order they were
    /// written. None of them is the default: the order is the human's typing
    /// kept intact so that editing the list reads back as they left it.
    pub models: Vec<String>,

    /// Whether a session under this Profile is given the account's memory store,
    /// or starts with an empty one of its own.
    ///
    /// On by default, and on for every Profile saved before there was a switch:
    /// the shared store is what the human has always been getting, so the switch
    /// is a way to stop rather than a way to start. Off, the session's store is
    /// its own and empty — fresh memory, and none of the human's transcripts
    /// reachable from inside it.
    pub memory: bool,

    /// Where this Profile is bound, where that is not this device.
    ///
    /// `None` is a Profile this device holds a binding of, whose account is
    /// this device's own. `Some` is one it holds none of: its record replicated
    /// here, and its account somewhere else — see [`Unbound`].
    pub unbound: Option<Unbound>,

    /// Where this Profile was minted and the id it had there, for one of this
    /// device's own that was numbered before Profiles had uuids.
    ///
    /// `None` for one made since. It is the bridge a member holding a mirror of
    /// it from before the switch matches that mirror by: the mirror kept the
    /// device it was at home on and the id it had there, and this is the same
    /// pair said from the minting device's side — see [`settle`].
    pub minted: Option<Minted>,
}

/// Where a Profile numbered before the uuids was minted: the device, and the
/// integer id it had there. See [`Profile::minted`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Minted {
    /// The **Device Id** of the machine it was made on, which is this one.
    pub device: String,

    /// And the id it had there before it was given its uuid.
    pub id: i64,
}

/// Where a Profile this device holds no binding of is bound: the devices whose
/// binding of it this device holds, and which of them hold a login.
///
/// **Its [`Profile::account`] is the first of those devices' paths**, which
/// belong to no filesystem here — two machines set up alike hold the same
/// paths, and the account under them is somebody else's — so nothing here
/// resolves them or launches under them: what a session here is given is the
/// account fetched from a bound device. Where no device's binding has arrived
/// yet it is the harness's shape with nothing in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unbound {
    /// The **Device Ids** of the machines that hold a binding of it, in the
    /// order those bindings were written down here. Empty where none has
    /// arrived.
    pub bound_on: Vec<String>,

    /// And those of them whose binding **holds a login file**, in the same
    /// order: the devices a session here can fetch the account from. A binding
    /// without one is not a source — there is nothing in it to copy.
    pub lending: Vec<String>,
}

impl Profile {
    /// Which agent this Profile runs, which is what its account's shape says.
    pub fn agent_type(&self) -> AgentType {
        self.account.agent_type()
    }

    /// The model to run on where nothing paired one with it.
    ///
    /// The first of the list, which is a Profile's only model in the ordinary
    /// case. Nothing picks this any more — a session runs on the model its
    /// Pairing names — so what is left for it is the Conversation that chose a
    /// Profile before there was a model to choose beside it: see
    /// [`Pairing::runs_on`]. `None` is a Profile with no models at all —
    /// refused above the store, so what it means here is a row somebody edited
    /// by hand.
    pub fn model(&self) -> Option<&str> {
        self.models.first().map(String::as_str)
    }
}

/// What a Conversation has settled about one of its roles: a Profile, and the
/// one of that Profile's models its sessions run on.
///
/// The pair rather than the Profile alone, because a Profile's list says what
/// its account *can* launch and a session runs one thing. Both halves are
/// chosen together, in one press, and both are fixed when grilling starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub profile: Profile,

    /// The model paired with it, where one was paired.
    ///
    /// `None` is a choice made before pairings existed: the Profile was picked
    /// alone and the model was whatever that Profile carried. Left as a state
    /// to be in rather than filled in on the way out, because the two are
    /// different things to a Conversation still drafting — an unpaired choice
    /// is one to make again — and see [`Pairing::runs_on`] for what a
    /// Conversation past drafting runs on instead.
    pub model: Option<String>,
}

impl Pairing {
    /// What a session under this Pairing is launched on.
    ///
    /// The paired model, and the Profile's own where nothing was paired — which
    /// is the model that Profile would have been run on at the time the choice
    /// was made, so a Conversation that chose before pairings existed goes on
    /// exactly as it did.
    pub fn runs_on(&self) -> Option<&str> {
        self.model.as_deref().or_else(|| self.profile.model())
    }
}

/// What a Conversation has settled about one of its roles: the Pairing that
/// role's sessions run under, that the role runs no session at all, or nothing
/// yet.
///
/// Three states rather than an `Option`, because *no review* is a choice the
/// human made and an empty picker is one they have not. Both leave the role
/// without a Pairing and only one of them lets the work start, so a record that
/// could not tell them apart would either refuse a settled Conversation or
/// start an unsettled one.
///
/// One variant carries a Profile and the rest carry nothing, which is what the
/// size lint is about — a Profile got bigger when it gained where it was
/// minted. Left as it is: these are read one role at a time and none is held in
/// a collection worth counting bytes of.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Picked {
    /// The picker is empty: nothing has been chosen for this role.
    ///
    /// Which includes a Profile chosen before pairings existed — see
    /// [`Pairing::model`] — because half a choice is a choice to make again.
    #[default]
    Nothing,

    /// The role runs no session at all, picked from the same flat list the
    /// Pairings are picked from and stored apart from having picked nothing.
    Skipped,

    /// The Profile and model this role's sessions are launched under.
    Under(Pairing),
}

impl Picked {
    /// The Pairing where one was picked, for everything that reads a role as
    /// something to launch a session under.
    pub fn pairing(&self) -> Option<&Pairing> {
        match self {
            Self::Under(pairing) => Some(pairing),
            _ => None,
        }
    }

    /// Whether the human picked the row that runs no session.
    pub fn skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }

    /// Whether anything has been picked at all — a Pairing or the row that says
    /// there is to be none.
    ///
    /// Says nothing about whether a Pairing that was picked is still something
    /// to run: whether its Profile's pair is where it was left is read off the
    /// filesystem, which is above the store.
    pub fn picked(&self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// What a Profile is being saved as — everything but the id, which is the
/// store's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFacts {
    pub name: Option<String>,
    pub account: Account,
    pub models: Vec<String>,
    pub memory: bool,
}

/// What became of writing a Profile down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Saving {
    /// Recorded.
    Saved,

    /// There is no Profile with that id to change.
    NoSuchProfile,

    /// Another Profile is called that already.
    NameTaken,

    /// That harness already has a Profile nobody named.
    DefaultTaken,
}

/// Which of the two uniqueness rules turned a new Profile away.
///
/// Two words rather than [`Saving`]'s four, because there is no rewriting a
/// Profile that is not there to rewrite: what saving a new one comes to is the
/// row or one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    /// Another Profile is called that already.
    NameTaken,

    /// That harness already has a Profile nobody named.
    DefaultTaken,
}

/// What became of removing one.
///
/// Two answers rather than three. A Conversation having chosen the Profile was
/// the third of them and is not a refusal any more: it is nulled out of every
/// Pairing that named it instead — see [`delete_profile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleting {
    Deleted,
    NoSuchProfile,
}

/// The Agent Profiles, as a kind of **Replicated record** (ADR-0028): the
/// Profile as the cluster holds it, the same uuid on every device.
///
/// Declared here rather than by the server, as the Authors are, because what a
/// Profile is is the store's to say — a Pairing, a steer and a Conversation's
/// three roles are read into Profiles inside the store's own transactions.
pub struct Profiles;

impl Kind for Profiles {
    const NAME: &'static str = "profiles";
    type Fields = ProfileRecord;
}

/// What a Profile's record holds: everything about it but where its account
/// is, which is each device's own — see [`Binding`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRecord {
    /// What the human calls it, where they have called it anything — see
    /// [`Profile::name`].
    pub name: Option<String>,

    /// The harness it runs, in [`AgentType::word`]'s spelling — a built-in's
    /// word, a plugin's uuid: which paths every device's binding of it holds.
    pub harness: String,

    /// The models it can run a session on, in the order they were written.
    pub models: Vec<String>,

    /// Whether a session under it shares the account's memory store.
    pub memory: bool,

    /// The **Device Id** it was minted on, for one numbered before Profiles
    /// had uuids — see [`Profile::minted`]. Arrives empty at the switch, which
    /// is made before this device knows its own id, and is said at the serve
    /// — see [`super::stamp_the_profiles`].
    #[serde(default)]
    pub minted_on: Option<String>,

    /// And the integer id it had there.
    #[serde(default)]
    pub minted_id: Option<i64>,
}

impl ProfileRecord {
    /// The record a save of `facts` writes, keeping where `held` was minted.
    fn of(facts: &ProfileFacts, held: Option<&ProfileRecord>) -> Self {
        Self {
            name: facts.name.clone(),
            harness: facts.account.agent_type().word().to_owned(),
            models: facts.models.clone(),
            memory: facts.memory,
            minted_on: held.and_then(|held| held.minted_on.clone()),
            minted_id: held.and_then(|held| held.minted_id),
        }
    }
}

/// The **bindings**: where a device keeps the account of a Profile, one per
/// Profile per device, as a kind of record of its own.
///
/// **Keyed `<profile uuid>/<device id>`** — see [`binding_uuid`] — because the
/// facility keys a record by its kind and uuid alone, and a binding is one of
/// a pair. Only the device it names ever writes it, so two devices never write
/// one binding and the merge rule has nothing to decide between them.
pub struct Bindings;

impl Kind for Bindings {
    const NAME: &'static str = "profile_bindings";
    type Fields = Binding;
}

/// One device's binding of a Profile: the account's paths there, resolved,
/// each under the key its harness declares it by — see [`Account`].
///
/// **The map is the binding**, and for a built-in harness the fields it had
/// before there was one are written beside it: Claude's pair as `claude_dir`
/// and `config_file`, every other built-in's one home as `home`. Those are
/// what a device still on an older release reads, so a binding written here
/// stays one it can run; a plugin harness, which no older release could run
/// anyway, is written as the map alone. A binding an older release wrote has
/// no map, and its fields are read as one — their names being the built-ins'
/// keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_dir: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,

    /// Each of the account's paths under its key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub paths: BTreeMap<String, String>,

    /// Whether the account at those paths holds a **login file**, as the
    /// device bound on it last looked — see [`logins_held`]. A binding without
    /// one is no source for a device running the Profile from a copy.
    ///
    /// Taken to hold one where a binding does not say, which is every binding
    /// written before it was said: what that device's next look says goes over
    /// it.
    #[serde(default = "holding")]
    pub login: bool,
}

/// What a binding that says nothing of its login is taken to hold — see
/// [`Binding::login`].
fn holding() -> bool {
    true
}

impl Binding {
    /// The binding `account` is written down as.
    fn of(account: &Account) -> Result<Self> {
        let paths = account
            .keyed()
            .iter()
            .map(|(key, path)| Ok((key.clone(), text(path)?.to_owned())))
            .collect::<Result<BTreeMap<_, _>>>()?;

        // The fields an older release reads, for a harness it could run.
        let old = |key: &str| match account.agent_type().built_in() {
            Some(_) => paths.get(key).cloned(),
            None => None,
        };

        Ok(Self {
            claude_dir: old(CLAUDE_DIR),
            config_file: old(CONFIG_FILE),
            home: old(HOME),
            paths,
            login: holding(),
        })
    }

    /// And the account it is, for a Profile of `harness` — refused where it
    /// lacks a path a built-in harness declares, which is a record somebody
    /// edited by hand: an account of nowhere is a bind that would land on `/`.
    ///
    /// A plugin harness's paths are taken as the map holds them, in the order
    /// of their keys: which it declares is the plugin's to say, above the
    /// store.
    fn account(&self, harness: AgentType, id: &str) -> Result<Account> {
        let held = |key: &str| {
            self.paths.get(key).cloned().or_else(|| {
                match key {
                    CLAUDE_DIR => &self.claude_dir,
                    CONFIG_FILE => &self.config_file,
                    HOME => &self.home,
                    _ => &None,
                }
                .clone()
            })
        };

        let Some(built_in) = harness.built_in() else {
            return Ok(Account::new(harness, self.paths.clone()));
        };

        let paths = built_in
            .paths
            .iter()
            .map(|key| {
                held(key).map(|path| (*key, PathBuf::from(path))).ok_or_else(|| {
                    anyhow!("Profile {id} runs {harness} and has no {key} bound here to run it under")
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Account::new(harness, paths))
    }
}

/// The uuid `device`'s binding of the Profile `profile` is kept under.
pub fn binding_uuid(profile: &str, device: &str) -> String {
    format!("{profile}/{device}")
}

/// The Device Id of this device, as the serve last said it — see
/// [`super::stamp_the_profiles`] — or the empty string where it has not said
/// one yet.
///
/// **The empty string is a device too**: a record or a binding written before
/// the serve has said which device this is — the switch itself, made as the
/// database is opened, and a database a suite fills before serving it — is
/// written by it, and the serve says the real id over every one of them.
pub(crate) async fn here(connection: &mut sqlx::SqliteConnection) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT device FROM this_device")
        .fetch_optional(&mut *connection)
        .await
        .context("reading which device this is")?;

    Ok(row.map(|(device,)| device).unwrap_or_default())
}

/// The tables the Profiles live in, beside the records.
///
/// **A Profile is a record** — see [`Profiles`] and [`Bindings`] — and
/// `profiles` is what is left of where they were kept before: the **mirrors
/// held from before the uuids**, each a row under a provisional uuid marked
/// with the device it was at home on and the id it had there, kept for their
/// Pairings until they are bridged — see [`settle`] — and, on a database not
/// yet opened by this release, this device's own rows and the other mirrors,
/// which are written down as records as it is opened (see
/// [`super::migrations`]). The two partial indexes over rows with no home
/// device are the uniqueness rules as they were kept before, over rows the
/// switch leaves none of; the rules are checked at the press now, against the
/// whole table of records — see [`clash`].
///
/// And `this_device`, which says which Device Id this database is the
/// device of — see [`here`].
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profiles (
             id          TEXT PRIMARY KEY,
             name        TEXT,
             claude_dir  TEXT NOT NULL,
             config_file TEXT NOT NULL,
             model       TEXT NOT NULL,
             agent_type  TEXT NOT NULL,
             memory      INTEGER NOT NULL DEFAULT 1,
             home_device TEXT,
             home_id     INTEGER,
             home_login  INTEGER,
             minted_on   TEXT,
             minted_id   INTEGER
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profiles table")?;

    mirror_columns(pool).await?;
    home_login_column(pool).await?;

    // Which device this is, said by the serve — see [`here`]. One row at most.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS this_device (
             one    INTEGER PRIMARY KEY CHECK (one = 1),
             device TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the this_device table")?;

    // And what it is called, said by the serve, and whether its own unnamed
    // Profiles have been named after it — see [`named_after_here`]. One row at
    // most, and none until the serve has said a name.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS this_device_called (
             one   INTEGER PRIMARY KEY CHECK (one = 1),
             name  TEXT NOT NULL,
             named INTEGER NOT NULL DEFAULT 0
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the this_device_called table")?;

    // The rule the column's own `UNIQUE` carried: no two Profiles of this
    // device's own called the same thing. Over the local rows alone now — what a
    // member calls its accounts is that machine's business, and a picker tells
    // two rows called `work` apart by the device drawn on them.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS profiles_named_here
         ON profiles (name) WHERE home_device IS NULL",
    )
    .execute(pool)
    .await
    .context("creating the index that keeps this device's own Profiles named apart")?;

    // At most one unnamed Profile per harness. A name is what tells two accounts
    // of one harness apart, so a harness may have one account nobody named — and
    // a second would be two rows a picker draws the same way, which is what the
    // unique name was always for.
    //
    // This device's own again, and under a name of its own for it: the rule
    // moved, so the index that carries it is made afresh rather than left
    // standing as the rule it used to be. The old one is taken away below.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS profiles_one_unnamed_here_per_agent
         ON profiles (agent_type) WHERE name IS NULL AND home_device IS NULL",
    )
    .execute(pool)
    .await
    .context("creating the index that keeps one unnamed Profile per harness")?;

    // And the mirrors held from before the uuids: one row per Profile per
    // device it is at home on, by the integer it had there. A mirror made since
    // has no such integer — it is held under its home's own uuid, which the
    // primary key keeps one of — and a null is in no unique index's way.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS profiles_one_mirror_per_home
         ON profiles (home_device, home_id) WHERE home_device IS NOT NULL",
    )
    .execute(pool)
    .await
    .context("creating the index that keeps one mirror per member Profile")?;

    // And the rule as it was before it was this device's own. Dropped rather
    // than left beside the two above: it names every row in the table, so a
    // member's unnamed Claude account would be refused for this device having
    // one of its own.
    sqlx::query("DROP INDEX IF EXISTS profiles_one_unnamed_per_agent")
        .execute(pool)
        .await
        .context("taking away the index that kept one unnamed Profile per harness everywhere")?;

    // The models each Profile can run, one row apiece. A table of its own for
    // the reason the directions are one: there was no migration machinery to
    // alter `profiles` with when the list arrived, so what was new hung off what
    // was there.
    //
    // `position` is the order they were written in and nothing more — no entry
    // is preferred — kept so that a list read back into the form is the list the
    // human typed.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profile_models (
             profile_id TEXT NOT NULL REFERENCES profiles(id),
             position   INTEGER NOT NULL,
             model      TEXT NOT NULL,
             PRIMARY KEY (profile_id, position)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profile_models table")?;

    // And where a Profile whose account is one relocatable home keeps it —
    // every backend but Claude, whose account is the pair in the row itself. A
    // table rather than a column for the reason `profile_models` is one: there
    // was no migration machinery to add a column with when homes arrived, so the
    // fact became a table hung off `profiles` by id. One home per Profile, so the
    // id is the key.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profile_homes (
             profile_id TEXT PRIMARY KEY REFERENCES profiles(id),
             home       TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profile_homes table")?;

    // And which uuid each Profile numbered before the uuids became, for what
    // outside the database was named by the integer — see
    // [`super::renumbered_profiles`]. Empty on a database that never had one.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profiles_renumbered (
             was  INTEGER PRIMARY KEY,
             uuid TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profiles_renumbered table")?;

    Ok(())
}

/// Where a row that is a mirror is at home, added through `ALTER TABLE` as well
/// as declared above — [`super::members::apply_schema`]'s rule, for its reason:
/// a database made this morning and one written before a Profile could be
/// somebody else's take the same path and end the same shape.
///
/// Both arriving null, which is what was true of every row before them: every
/// Profile a device held was its own.
///
/// Its own function because it is run twice over. [`apply_schema`] runs it
/// before the indexes below, which are written over these two columns — a column
/// that arrived after them would be one they could not name — and the rewrite in
/// [`super::migrations`] that lets a Profile go unnamed runs afterwards and
/// rebuilds this table to the shape it had before either column existed, so the
/// rewrite that follows *it* asks for them again.
pub(crate) async fn mirror_columns(pool: &SqlitePool) -> Result<()> {
    for (column, kind) in [("home_device", "TEXT"), ("home_id", "INTEGER")] {
        added(pool, column, kind).await?;
    }

    Ok(())
}

/// And whether the account a mirror names holds a login file over there, added
/// the same way and asked for in the same two places.
///
/// Its own function rather than a third entry in the loop above only because
/// the rewrite in [`super::migrations`] that lets a Profile go unnamed remakes
/// this table in a shape from before any of the three existed — so the rewrite
/// after it asks for the two above and this one again, and then carries all
/// three across. **Asked for rather than re-added**: a column dropped here and
/// put back afterwards is one the pool can refuse, the pragma that looks for it
/// and the `ALTER` that adds it being handed to whichever connections are free.
///
/// Null is *nobody has said*, which is what every mirror before the column
/// existed said. Read by nothing now that the mirrors are records.
pub(crate) async fn home_login_column(pool: &SqlitePool) -> Result<()> {
    added(pool, "home_login", "INTEGER").await
}

/// One column on the profiles table, added where it is not there already.
async fn added(pool: &SqlitePool, column: &str, kind: &str) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('profiles') WHERE name = ?")
            .bind(column)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the profiles table's {column} column"))?;

    if there.is_none() {
        // Interpolated rather than bound, because a column name is not a value
        // — and every one of them is written out by a caller here rather than
        // taken from anywhere.
        sqlx::query(&format!("ALTER TABLE profiles ADD COLUMN {column} {kind}"))
            .execute(pool)
            .await
            .with_context(|| format!("adding the profiles table's {column} column"))?;
    }

    Ok(())
}

/// Record a Profile, which is expected to have been checked already: that its
/// pair exists and is of its harness's shape is decided above the store.
///
/// **A record and this device's binding**, written in one transaction: the
/// name, the harness, the models and the memory switch are the record, which
/// is the cluster's; the account's paths are this device's binding of it.
///
/// The [`Clash`] is which of the two uniqueness rules turned it away, checked
/// at the press against **every live record of the table** — see [`clash`] —
/// rather than by an index: a record that arrived from a member is in the
/// table beside this device's own, and a pair that arrived alike is kept.
///
/// The uuid is minted in a statement — see [`MINTED_UUID`] — the store having
/// no generator of its own.
pub async fn create_profile(
    pool: &SqlitePool,
    facts: &ProfileFacts,
) -> Result<Result<Profile, Clash>> {
    create_profile_holding(pool, facts, holding()).await
}

/// And the same where the save looked at the account first: `login` is whether
/// it holds a **login file**, said on the binding in the write that makes it —
/// see [`Binding::login`]. Without it the binding is taken to hold one until
/// this device's next look, and a member pulling in between would read an
/// account with none as one to fetch from.
pub async fn create_profile_holding(
    pool: &SqlitePool,
    facts: &ProfileFacts,
    login: bool,
) -> Result<Result<Profile, Clash>> {
    let mut tx = super::writing(pool, "saving a Profile").await?;

    if let Some(clash) = clash(&mut tx, facts, None).await? {
        return Ok(Err(clash));
    }

    let (id,): (String,) = sqlx::query_as(&format!("SELECT {MINTED_UUID}"))
        .fetch_one(&mut *tx)
        .await
        .context("minting a uuid for a new Profile")?;

    let here = here(&mut tx).await?;

    replicated::written_on::<Profiles>(&mut tx, &here, &id, &ProfileRecord::of(facts, None), true)
        .await?;

    replicated::written_on::<Bindings>(
        &mut tx,
        &here,
        &binding_uuid(&id, &here),
        &Binding {
            login,
            ..Binding::of(&facts.account)?
        },
        true,
    )
    .await?;

    tx.commit()
        .await
        .with_context(|| format!("saving the Profile {:?}", facts.name))?;

    Ok(Ok(Profile {
        id,
        name: facts.name.clone(),
        account: facts.account.clone(),
        models: facts.models.clone(),
        memory: facts.memory,

        // Bound here, by the binding written beside it.
        unbound: None,

        // And made since the uuids, so it was never numbered anything else.
        minted: None,
    }))
}

/// Which of the two uniqueness rules `facts` would break, against every live
/// record of the table but `except`'s — or none.
///
/// **Per harness, both of them** (ADR-0028): at most one unnamed Profile of a
/// harness, and no two of one harness named alike. A Claude and a Codex
/// Profile may share a name, the harness's mark telling them apart in a
/// picker. Every live record is asked, whichever device wrote it, because a
/// picker draws them all as one list; a removed one is in nobody's way.
async fn clash(
    connection: &mut sqlx::SqliteConnection,
    facts: &ProfileFacts,
    except: Option<&str>,
) -> Result<Option<Clash>> {
    let harness = facts.account.agent_type().word();

    let taken = replicated::records_on::<Profiles>(connection)
        .await?
        .into_iter()
        .filter(|record| !record.removed() && Some(record.uuid.as_str()) != except)
        .any(|record| record.fields.harness == harness && record.fields.name == facts.name);

    Ok(taken.then_some(match facts.name {
        Some(_) => Clash::NameTaken,
        None => Clash::DefaultTaken,
    }))
}

/// A fresh version-4 uuid, as an SQL expression minted per row it is evaluated
/// for: what a new Profile is keyed by, and what the switch gives every row it
/// renumbers.
///
/// In the statement rather than handed in, because the store has no generator
/// of its own and SQLite's `randomblob` is one — and the rewrite that numbers
/// every saved Profile at once needs one per row, which a bound value cannot
/// be. The version nibble is a literal `4` and the variant one of `89ab`, which
/// is the difference between sixteen random bytes and a uuid something else
/// will take as one.
pub(crate) const MINTED_UUID: &str = "(lower(hex(randomblob(4))) || '-' || \
     lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-' || \
     substr('89ab', 1 + abs(random()) % 4, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || \
     lower(hex(randomblob(6))))";

/// Rewrite a Profile, whole: everything about one is the human's to change, and
/// nothing about it is an artifact that could have been built from it yet.
///
/// The record and this device's binding, each written only where it moved —
/// see [`replicated::Written::Unchanged`]. Where it was minted is kept: it is
/// a fact about the Profile, not about the form.
///
/// **A Profile this device holds no binding of is rewritten as a record
/// alone**, whatever account `facts` carries, and keeps its harness: its
/// account is each bound device's own, and binding this one is a press of its
/// own — never a side effect of a save.
pub async fn update_profile(pool: &SqlitePool, id: &str, facts: &ProfileFacts) -> Result<Saving> {
    rewrite_profile(pool, id, facts, None).await
}

/// And the same where the save looked at the account first: `login` is whether
/// it holds a login file, said on this device's binding whatever it said before
/// — see [`create_profile_holding`]. A Profile with no binding here is still
/// rewritten as a record alone, and `login` goes nowhere.
pub async fn update_profile_holding(
    pool: &SqlitePool,
    id: &str,
    facts: &ProfileFacts,
    login: bool,
) -> Result<Saving> {
    rewrite_profile(pool, id, facts, Some(login)).await
}

/// **Bind this device** to a Profile: write its binding of the Profile `id`,
/// naming `account` here, `login` being whether the account at those paths
/// holds a login file.
///
/// The press of its own [`update_profile`] keeps a save from being. What it is
/// for is a Profile bound only on other devices — one of them switched off,
/// unlinked, or never to be seen again — which from here on is run as this
/// device's own account rather than from a copy of somebody else's. Written
/// over a binding this device already holds, so the press on a Profile bound
/// here is a rewrite of its paths and nothing else.
///
/// `account` is the Profile's harness's shape, which is the caller's to have
/// checked: a record's harness is not this press's to change.
///
/// [`Saving::NoSuchProfile`] for a record that is not there or is removed, and
/// for one whose binding here carries a deletion stamp — a Profile removed
/// here, whose stamp is what every other device is learning from.
pub async fn bind_profile_here(
    pool: &SqlitePool,
    id: &str,
    account: &Account,
    login: bool,
) -> Result<Saving> {
    let mut tx = super::writing(pool, "binding a Profile on this device").await?;

    let Some(held) = replicated::record_on::<Profiles>(&mut tx, id).await? else {
        return Ok(Saving::NoSuchProfile);
    };

    if held.removed() {
        return Ok(Saving::NoSuchProfile);
    }

    let here = here(&mut tx).await?;

    let written = replicated::written_on::<Bindings>(
        &mut tx,
        &here,
        &binding_uuid(id, &here),
        &Binding {
            login,
            ..Binding::of(account)?
        },
        true,
    )
    .await?;

    if written == replicated::Written::Removed {
        return Ok(Saving::NoSuchProfile);
    }

    tx.commit()
        .await
        .with_context(|| format!("binding Profile {id} on this device"))?;

    Ok(Saving::Saved)
}

/// Both rewrites, `login` being what was looked at where anything was.
async fn rewrite_profile(
    pool: &SqlitePool,
    id: &str,
    facts: &ProfileFacts,
    login: Option<bool>,
) -> Result<Saving> {
    let mut tx = super::writing(pool, "rewriting a Profile").await?;

    let Some(held) = replicated::record_on::<Profiles>(&mut tx, id).await? else {
        return Ok(Saving::NoSuchProfile);
    };

    if held.removed() {
        return Ok(Saving::NoSuchProfile);
    }

    let here = here(&mut tx).await?;

    // This device's binding, where it holds a live one: whether the account
    // at its paths held a login is what it last looked, and a save that kept
    // the paths keeps that — see [`logins_held`].
    let binding = replicated::record_on::<Bindings>(&mut tx, &binding_uuid(id, &here))
        .await?
        .filter(|binding| !binding.removed())
        .map(|binding| binding.fields);

    let bound = binding.is_some();

    let facts = match bound {
        true => facts.clone(),
        false => ProfileFacts {
            account: Account::nowhere(AgentType::read(&held.fields.harness)),
            ..facts.clone()
        },
    };

    if let Some(clash) = clash(&mut tx, &facts, Some(id)).await? {
        return Ok(match clash {
            Clash::NameTaken => Saving::NameTaken,
            Clash::DefaultTaken => Saving::DefaultTaken,
        });
    }

    replicated::written_on::<Profiles>(
        &mut tx,
        &here,
        id,
        &ProfileRecord::of(&facts, Some(&held.fields)),
        false,
    )
    .await?;

    if let Some(held) = binding {
        let mut binding = Binding::of(&facts.account)?;

        // The same account whatever shape the binding was written in: one an
        // older release wrote has no map, and is no less the same paths.
        let kept = held
            .account(facts.account.agent_type(), id)
            .is_ok_and(|held| held == facts.account);

        binding.login = match login {
            Some(looked) => looked,
            None if kept => held.login,
            None => binding.login,
        };

        replicated::written_on::<Bindings>(
            &mut tx,
            &here,
            &binding_uuid(id, &here),
            &binding,
            true,
        )
        .await?;
    }

    tx.commit()
        .await
        .with_context(|| format!("rewriting Profile {id}"))?;

    Ok(Saving::Saved)
}

/// Remove a Profile, taking it out of everything that named it.
///
/// **A deletion stamp** on the record and on this device's binding of it,
/// both kept: the stamp is what every other device is to learn from it.
///
/// **Always possible.** A Conversation that had chosen it used to stand in the
/// way; now its choice is nulled out and the removal goes through. What that
/// costs is a session that starts nothing the next time that Conversation is
/// driven, and a steer to pick another account — which is the human's to spend
/// on a Profile they have just decided they are finished with, rather than a
/// refusal that leaves them unable to finish with it at all.
///
/// It costs a session already running nothing: one that has launched holds the
/// account it launched under, and nothing re-reads this record to keep it
/// going.
///
/// Both halves of every Pairing that named it go, not the Profile half alone: a
/// Pairing is an account and a model chosen together, and a role left holding
/// the model by itself would be half a choice nothing can launch. The Repo's
/// memory of what it was last grilled with goes the same way — `repo_pairings`
/// names the Profile in a column that cannot be null — so the next Conversation
/// started there arrives with that picker empty rather than prefilled with an
/// account that is gone. And so does the picker of a steer somebody is part-way
/// through writing, which is the same choice one press before it is settled.
///
/// What is deliberately left alone is the record of what has already run:
/// [`super::session_pairings`] keeps the Profile's *name* rather than its id, so
/// every session ever launched under it goes on saying so, and a recorded
/// steer keeps the uuid.
pub async fn delete_profile(pool: &SqlitePool, id: &str) -> Result<Deleting> {
    let mut tx = super::writing(pool, "removing a Profile").await?;

    let here = here(&mut tx).await?;

    if !replicated::removed_on::<Profiles>(&mut tx, &here, id).await? {
        return Ok(Deleting::NoSuchProfile);
    }

    replicated::removed_on::<Bindings>(&mut tx, &here, &binding_uuid(id, &here)).await?;

    forget_pairings(&mut tx, id).await?;

    tx.commit()
        .await
        .with_context(|| format!("removing Profile {id}"))?;

    Ok(Deleting::Deleted)
}

/// Say whether the account each of this device's bindings names holds a
/// login file: each `(Profile uuid, held)` written over this device's binding
/// of it where that binding says otherwise, and nothing else touched.
///
/// **The bound device is the one that can look**, so it is the one that says:
/// a device running the Profile from a copy fetches the account from a device
/// whose binding holds one, and a binding written here replicates like any
/// other. Asked by the server whenever it reads its Profiles — see
/// `verkstead_server::profiles`. `true` where any binding moved, which is the
/// members' cue to pull.
///
/// Read before anything is written, so the ordinary read — where nothing moved
/// — takes no writer's lock.
pub async fn logins_held(pool: &SqlitePool, held: &[(String, bool)]) -> Result<bool> {
    let moving = {
        let mut tx = super::reading(pool, "reading which accounts hold a login").await?;
        let here = here(&mut tx).await?;
        let mut moving = false;

        for (id, login) in held {
            moving |= replicated::record_on::<Bindings>(&mut tx, &binding_uuid(id, &here))
                .await?
                .is_some_and(|binding| !binding.removed() && binding.fields.login != *login);
        }

        moving
    };

    if !moving {
        return Ok(false);
    }

    let mut tx = super::writing(pool, "saying which accounts hold a login").await?;

    let here = here(&mut tx).await?;
    let mut moved = false;

    for (id, login) in held {
        let uuid = binding_uuid(id, &here);

        let Some(binding) = replicated::record_on::<Bindings>(&mut tx, &uuid).await? else {
            continue;
        };

        if binding.removed() || binding.fields.login == *login {
            continue;
        }

        let binding = Binding {
            login: *login,
            ..binding.fields
        };

        moved |= matches!(
            replicated::written_on::<Bindings>(&mut tx, &here, &uuid, &binding, false).await?,
            replicated::Written::Written
        );
    }

    tx.commit()
        .await
        .context("saying which accounts hold a login")?;

    Ok(moved)
}

/// Write `facts` down as the record and the binding `device` wrote of the
/// Profile `id`, stamped older than anything this device writes, by the merge
/// rule — what a member's records arriving come to.
///
/// Public for the suites that stand a member's Profile up without a member to
/// pull it from.
pub async fn profile_arrived(
    pool: &SqlitePool,
    id: &str,
    device: &str,
    facts: &ProfileFacts,
) -> Result<()> {
    replicated::merge::<Profiles>(
        pool,
        &[replicated::Record {
            uuid: id.to_owned(),
            updated_at: 1,
            deleted_at: None,
            written_by: device.to_owned(),
            fields: ProfileRecord::of(facts, None),
        }],
    )
    .await?;

    replicated::merge::<Bindings>(
        pool,
        &[replicated::Record {
            uuid: binding_uuid(id, device),
            updated_at: 1,
            deleted_at: None,
            written_by: device.to_owned(),
            fields: Binding::of(&facts.account)?,
        }],
    )
    .await?;

    Ok(())
}

/// Settle what records arriving from members say about this device's own
/// references, once they are merged: every provisional mirror bridged onto the
/// record its home published, and every Pairing naming a removed record nulled.
///
/// **The bridge** (ADR-0028, *mirror rows are bridged, not dropped*): a mirror
/// held from before the uuids is a row of `profiles` under a provisional uuid
/// of this device's minting, with the device it was at home on and the id it
/// had there — and a record carrying that same pair as where it was minted is
/// the Profile it stands for. Matched, the provisional uuid is rewritten to the
/// record's in everything that named it — a Conversation's three Pairings and
/// the Profile a signed-out stop waits on, the Repo's pairing memory, the
/// pending and recorded steers — and the row goes. Nothing is nulled. A mirror
/// whose home is too old to publish records is left as it is, Pairings and all,
/// until it does.
///
/// **A removed record** nulls every Pairing that named its uuid, the way
/// removing one here does — see [`delete_profile`]. Asked of every removed
/// record whichever device removed it, which is idempotent: a Pairing nulled
/// once names nothing the next time.
///
/// What comes back is each provisional uuid bridged and the uuid it became, for
/// the caller to move what outside the database is named by it — the account
/// copy under the Data Directory — and the uuids of the removed records.
pub async fn settle(pool: &SqlitePool) -> Result<Settled> {
    let mut tx = super::writing(pool, "settling the Profiles a member's records name").await?;

    let records = replicated::records_on::<Profiles>(&mut tx).await?;

    let provisional: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT id, home_device, home_id FROM profiles
         WHERE home_device IS NOT NULL AND home_id IS NOT NULL",
    )
    .fetch_all(&mut *tx)
    .await
    .context("reading the mirrors held from before the uuids")?;

    let mut settled = Settled::default();

    for (held, home, was) in provisional {
        let Some(record) = records.iter().find(|record| {
            record.fields.minted_on.as_deref() == Some(home.as_str())
                && record.fields.minted_id == Some(was)
        }) else {
            continue;
        };

        rename_references(&mut tx, &held, &record.uuid).await?;

        for statement in [
            "DELETE FROM profile_models WHERE profile_id = ?",
            "DELETE FROM profile_homes WHERE profile_id = ?",
            "DELETE FROM profiles WHERE id = ?",
        ] {
            sqlx::query(statement)
                .bind(&held)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("taking away the provisional mirror {held}"))?;
        }

        settled.bridged.push((held, record.uuid.clone()));
    }

    for record in records.iter().filter(|record| record.removed()) {
        forget_pairings(&mut tx, &record.uuid).await?;
        settled.removed.push(record.uuid.clone());
    }

    tx.commit()
        .await
        .context("settling the Profiles a member's records name")?;

    Ok(settled)
}

/// What [`settle`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settled {
    /// Each provisional mirror bridged, and the uuid it became.
    pub bridged: Vec<(String, String)>,

    /// Every removed record, whose Pairings are nulled.
    pub removed: Vec<String>,
}

/// Every reference to a Profile, moved from `from` to `to`.
async fn rename_references(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    from: &str,
    to: &str,
) -> Result<()> {
    for (table, column) in [
        ("conversations", "grilling_profile_id"),
        ("conversations", "implementation_profile_id"),
        ("conversations", "review_profile_id"),
        ("conversations", "stopped_signed_out"),
        ("repo_pairings", "profile_id"),
        ("pending_steers", "profile_id"),
        ("steers", "profile_id"),
    ] {
        sqlx::query(&format!(
            "UPDATE {table} SET {column} = ? WHERE {column} = ?"
        ))
        .bind(to)
        .bind(from)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("naming Profile {to} for {from} in {table}.{column}"))?;
    }

    Ok(())
}

/// Every Profile, by name: every live record, whether or not this device holds
/// a binding of it.
///
/// Alphabetical like the Repos, and for the same reason: a Profile is not news,
/// it is something to pick out of a short list, and the name is what it is
/// looked for by. The one nobody named sorts first — which is where a
/// harness's default belongs on a list its named accounts are the exceptions
/// on — and two named alike keep the order they were written down in.
pub async fn profiles(pool: &SqlitePool) -> Result<Vec<Profile>> {
    let mut listed = profiles_as_saved(pool).await?;

    listed.sort_by(|one, other| one.name.cmp(&other.name));

    Ok(listed)
}

/// Every Profile, in the order they were written down here.
///
/// What a choice of *the earliest saved* reads, which a uuid cannot be sorted
/// into the way an integer id could.
pub async fn profiles_as_saved(pool: &SqlitePool) -> Result<Vec<Profile>> {
    let mut tx = super::reading(pool, "listing the Profiles").await?;

    let here = here(&mut tx).await?;
    let bindings = bindings_on(&mut tx).await?;

    replicated::records_on::<Profiles>(&mut tx)
        .await?
        .into_iter()
        .filter(|record| !record.removed())
        .map(|record| profile_of(record.uuid, record.fields, &bindings, &here))
        .collect()
}

/// Where every live Profile is bound, by its uuid: each device holding a
/// binding of it — this one among them — with the account that binding names
/// there, in the order those bindings were written down here.
///
/// What the Profile card's line per device is drawn off. A binding not of its
/// Profile's shape, which is one somebody edited by hand, is left out rather
/// than taking the rest with it, and a Profile bound nowhere is not in it.
///
/// **With the id this device's own bindings are written under**, which is
/// what [`Profile::unbound`] is read against — the empty string where the
/// serve has not said one yet (see [`here`]) — so the card's line for this
/// device and the row's own reading of whether it is bound here are the one
/// answer.
pub async fn bound_where(pool: &SqlitePool) -> Result<BoundWhere> {
    let mut tx = super::reading(pool, "reading where the Profiles are bound").await?;

    let here = here(&mut tx).await?;
    let mut bindings = bindings_on(&mut tx).await?;
    let mut bound = HashMap::new();

    for record in replicated::records_on::<Profiles>(&mut tx).await? {
        if record.removed() {
            continue;
        }

        let Some(devices) = bindings.remove(&record.uuid) else {
            continue;
        };

        let harness = AgentType::read(&record.fields.harness);

        let accounts = devices
            .into_iter()
            .filter_map(|(device, binding)| {
                Some((device, binding.account(harness, &record.uuid).ok()?))
            })
            .collect();

        bound.insert(record.uuid, accounts);
    }

    Ok(BoundWhere { here, bound })
}

/// Where every live Profile is bound — see [`bound_where`].
#[derive(Debug, Clone, Default)]
pub struct BoundWhere {
    /// The id this device's own bindings are written under.
    pub here: String,

    /// Each device holding a binding of a Profile, by the Profile's uuid, with
    /// the account it names there.
    pub bound: HashMap<String, Vec<(String, Account)>>,
}

/// Every live binding, by the Profile it binds: each device's, in the order
/// they were written down here.
async fn bindings_on(
    connection: &mut sqlx::SqliteConnection,
) -> Result<HashMap<String, Vec<(String, Binding)>>> {
    let mut bindings: HashMap<String, Vec<(String, Binding)>> = HashMap::new();

    for binding in replicated::records_on::<Bindings>(connection).await? {
        if binding.removed() {
            continue;
        }

        // `<profile uuid>/<device id>`, the device being the last part — see
        // [`binding_uuid`].
        let Some((profile, device)) = binding.uuid.rsplit_once('/') else {
            continue;
        };

        bindings
            .entry(profile.to_owned())
            .or_default()
            .push((device.to_owned(), binding.fields));
    }

    Ok(bindings)
}

/// One record as a [`Profile`]: run as this device's binding of it, where
/// `here` holds one, and otherwise [`Unbound`] — see there.
fn profile_of(
    id: String,
    record: ProfileRecord,
    bindings: &HashMap<String, Vec<(String, Binding)>>,
    here: &str,
) -> Result<Profile> {
    let harness = AgentType::read(&record.harness);
    let bound = bindings.get(&id).map(Vec::as_slice).unwrap_or_default();

    let (account, unbound) = match bound.iter().find(|(device, _)| device == here) {
        Some((_, binding)) => (binding.account(harness, &id)?, None),

        None => {
            let elsewhere = bound.iter().filter(|(device, _)| device != here);

            // The first device's paths, where its binding is of the record's
            // shape — a binding somebody edited by hand being no reason to
            // take the Profile off every list here.
            let account = elsewhere
                .clone()
                .find_map(|(_, binding)| binding.account(harness, &id).ok())
                .unwrap_or_else(|| Account::nowhere(harness));

            let bound_on = elsewhere
                .clone()
                .map(|(device, _)| device.clone())
                .collect();

            let lending = elsewhere
                .filter(|(_, binding)| binding.login)
                .map(|(device, _)| device.clone())
                .collect();

            (account, Some(Unbound { bound_on, lending }))
        }
    };

    // Where it was minted, once the serve has said which device that is — see
    // [`super::stamp_the_profiles`].
    let minted = match (record.minted_on, record.minted_id) {
        (Some(device), Some(id)) if !device.is_empty() => Some(Minted { device, id }),
        _ => None,
    };

    Ok(Profile {
        id,
        name: record.name,
        account,
        models: record.models,
        memory: record.memory,
        unbound,
        minted,
    })
}

/// One Profile, or `None` if there is no such Profile — a removed record
/// being none, and so is a provisional mirror, which is in no list.
pub async fn load_profile(pool: &SqlitePool, id: &str) -> Result<Option<Profile>> {
    let mut tx = super::reading(pool, "loading a Profile").await?;

    profile_on(&mut tx, id).await
}

/// And the same off a connection already held, which is how a read that has to
/// see one moment of the record — the Timeline's — reads the Profile a steer
/// names inside its own transaction rather than off a second connection.
pub(crate) async fn profile_on(
    connection: &mut sqlx::SqliteConnection,
    id: &str,
) -> Result<Option<Profile>> {
    let Some(record) = replicated::record_on::<Profiles>(&mut *connection, id).await? else {
        return Ok(None);
    };

    if record.removed() {
        return Ok(None);
    }

    let here = here(&mut *connection).await?;
    let bindings = bindings_on(&mut *connection).await?;

    profile_of(record.uuid, record.fields, &bindings, &here).map(Some)
}

/// Whether a Profile is there to be picked under `id`: a live record. What a
/// Pairing being settled is checked against, there being no foreign key to
/// check it — a record is nobody's to cascade from.
pub(crate) async fn pickable(connection: &mut sqlx::SqliteConnection, id: &str) -> Result<bool> {
    let there: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM replicated WHERE kind = ? AND uuid = ? AND deleted_at IS NULL",
    )
    .bind(Profiles::NAME)
    .bind(id)
    .fetch_optional(&mut *connection)
    .await
    .with_context(|| format!("looking for Profile {id}"))?;

    Ok(there.is_some())
}

/// A row of the profiles table as a [`Profile`].
type Row = (
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<i64>,
    Option<bool>,
    Option<String>,
    Option<i64>,
);

/// The columns every read of the table takes, in [`Row`]'s order.
///
/// Written once rather than in each of the two statements that select them: the
/// tuple they are read into is one shape, and two lists a column apart would be
/// a mirror read as somebody else's Profile.
const COLUMNS: &str = "id, name, claude_dir, config_file, model, agent_type, memory, \
                       home_device, home_id, home_login, minted_on, minted_id";

/// One row, whatever `profile_models` holds for it, and the home in
/// `profile_homes` where its type keeps one.
///
/// An empty list is a Profile written before the list existed: what it holds is
/// the one model in the old column, which becomes the sole entry of its list
/// without anybody having to retype it. A Profile saved since always has its
/// rows, so the old column is never read for one.
///
/// A type whose account is one home and no home row is a Profile something
/// edited by hand: refused rather than read as a home of the empty string,
/// because an account of nowhere is a bind that would land on `/`.
fn read_row(row: Row, listed: Vec<String>, home: Option<String>) -> Result<Profile> {
    let (
        id,
        name,
        claude_dir,
        config_file,
        model,
        agent_type,
        memory,
        _home_device,
        _home_id,
        _home_login,
        minted_on,
        minted_id,
    ) = row;

    let models = match (listed.is_empty(), model.is_empty()) {
        (true, false) => vec![model],
        _ => listed,
    };

    let agent_type = AgentType::read(&agent_type);

    // The pair in the row itself for Claude, and the one home in
    // `profile_homes` for every harness after it: the only shapes a row was
    // ever written in, the table being from before a harness could be a
    // plugin's.
    let account = match agent_type == AgentType::CLAUDE {
        true => Account::claude(claude_dir, config_file),
        false => Account::home(agent_type, kept(agent_type, &id, home)?),
    };

    // Where it was minted, once the serve has said which device that is — see
    // [`super::stamp_the_profiles`]. A mirror never has one.
    let minted = match (minted_on, minted_id) {
        (Some(device), Some(id)) => Some(Minted { device, id }),
        _ => None,
    };

    // Read as the row says it and nothing of where it is bound: what reads a
    // row now is the switch, which writes the account down as a binding.
    Ok(Profile {
        id,
        name,
        account,
        models,
        memory,
        unbound: None,
        minted,
    })
}

/// The home a Profile of a type that keeps one was recorded with.
fn kept(agent_type: AgentType, id: &str, home: Option<String>) -> Result<String> {
    home.ok_or_else(|| {
        anyhow!(
            "Profile {id} runs {} and has no home recorded to run it under",
            agent_type.word()
        )
    })
}

/// Take a Profile out of every Pairing that named it, on its way to being
/// removed.
///
/// Four places: a Conversation names one per role, and a Repo remembers the one
/// it was last grilled with in each of the three. Nothing is refused and nothing
/// is reported — a Profile nobody has ever picked passes through here without
/// touching a row, which is what the ordinary removal is.
///
/// The model rows go before the columns they are found by, because that is what
/// finds them: a role's model is looked up through the Conversation whose column
/// still names the Profile, so nulling first would leave the model half behind
/// with nothing left pointing at it.
///
/// Nobody is put on the row that runs no session by this. A role picked away is
/// something the human chose and *no Profile any more* is not: what the removal
/// leaves is a picker with nothing in it, which is the state a Conversation
/// starts in.
async fn forget_pairings(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: &str) -> Result<()> {
    for role in super::conversations::Role::ALL {
        sqlx::query(&format!(
            "DELETE FROM pairing_models
             WHERE role = ?
               AND conversation_id IN (SELECT id FROM conversations WHERE {} = ?)",
            role.column(),
        ))
        .bind(role.stored())
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("clearing the models paired with Profile {id} in the {role:?} role")
        })?;

        sqlx::query(&format!(
            "UPDATE conversations SET {} = NULL WHERE {} = ?",
            role.column(),
            role.column(),
        ))
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("clearing Profile {id} off the Conversations that chose it to {role:?}")
        })?;
    }

    sqlx::query("DELETE FROM repo_pairings WHERE profile_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("forgetting that a Repo was last grilled under Profile {id}"))?;

    // And the picker of a steer somebody is part-way through writing, which is
    // a Pairing chosen and not yet settled — see [`super::pending_steers`].
    // Both halves, by the rule above: a form holding the model of an account
    // that has gone is half a choice, and the pane reads a row with one half in
    // it as nothing picked anyway. The rest of the form is left exactly as it
    // was — the Profile going is no reason to throw away an afternoon's
    // writing, and picking another is a control the pane already has.
    sqlx::query("UPDATE pending_steers SET profile_id = NULL, model = NULL WHERE profile_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("clearing Profile {id} off the steers being written under it"))?;

    Ok(())
}

/// A path as SQLite can hold it, which is UTF-8 or nothing.
///
/// Refused outright rather than written lossily, as a Repo's path is: a stored
/// path that is not the one on disk is a boundary check that will pass for the
/// wrong directory later.
fn text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("the path {} is not valid UTF-8", path.display()))
}

/// **The switch**: every one of this device's own Profiles still kept as a row
/// of `profiles` taken up into a record under the uuid it already carries, its
/// account's paths into **this device's binding** of it — and the row, its
/// models and its home taken away. Run as the database is opened, inside
/// [`super::migrations`]'s transaction.
///
/// **Written by whichever device this database says it is**, which at the
/// first open is none yet — see [`here`]: the serve says the real id over
/// every record and binding written before it knew, in
/// [`super::stamp_the_profiles`]. Where each was minted comes across as it
/// stood on the row, for the same serve to finish saying.
///
/// In the order the rows were written down, which is the order the records are
/// read back in. A uuid already held as a record is left as it is.
pub(crate) async fn take_up(connection: &mut sqlx::SqliteConnection) -> Result<()> {
    let here = here(&mut *connection).await?;

    written_down(
        connection,
        "home_device IS NULL",
        Some(&here),
        replicated::now(),
    )
    .await?;

    sqlx::query("INSERT INTO replicated_taken_up (kind) VALUES (?) ON CONFLICT (kind) DO NOTHING")
        .bind(Profiles::NAME)
        .execute(&mut *connection)
        .await
        .context("saying the Profiles have been taken up")?;

    Ok(())
}

/// **The mirrors retired** (ADR-0028): every mirror held under the uuid its
/// home knows the Profile by written down as a record of that uuid, and its
/// account's paths as **its home's binding** of it — both stamped at the epoch
/// and written by the home device, so that the home's own record and binding
/// win the first pull of it, and until then the Profile is on every list here
/// under the uuid the Pairings name. The row, its models and its home are
/// taken away. Run as the database is opened, inside [`super::migrations`]'s
/// transaction, every time: a database with no such row has nothing to do.
///
/// **A mirror held from before the uuids is left as it is**, under its
/// provisional uuid: its home has published no record yet that it could be the
/// same Profile as, so it is kept for its Pairings and is in no list until one
/// arrives — see [`settle`].
pub(crate) async fn mirrors_written_down(connection: &mut sqlx::SqliteConnection) -> Result<()> {
    written_down(
        connection,
        "home_device IS NOT NULL AND home_id IS NULL",
        None,
        0,
    )
    .await
}

/// Every row of `profiles` that `which` picks, written down as a record and a
/// binding under the uuid it carries — by `writer`, or by the device each row
/// is at home on where that is `None` — and taken out of the table.
///
/// In the order the rows were written down, which is the order the records are
/// read back in. A uuid already held as a record is left as it is.
async fn written_down(
    connection: &mut sqlx::SqliteConnection,
    which: &str,
    writer: Option<&str>,
    stamp: i64,
) -> Result<()> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM profiles WHERE {which} ORDER BY rowid"
    ))
    .fetch_all(&mut *connection)
    .await
    .context("reading the Profiles kept as rows to write them down as records")?;

    for row in rows {
        let id = row.0.clone();
        let writer = match (writer, &row.7) {
            (Some(writer), _) => writer.to_owned(),
            (None, Some(home)) => home.clone(),
            (None, None) => bail!("Profile {id} is a mirror with no device it is at home on"),
        };

        let listed: Vec<(String,)> = sqlx::query_as(
            "SELECT model FROM profile_models WHERE profile_id = ? ORDER BY position",
        )
        .bind(&id)
        .fetch_all(&mut *connection)
        .await
        .with_context(|| format!("reading the models Profile {id} runs"))?;

        let home: Option<(String,)> =
            sqlx::query_as("SELECT home FROM profile_homes WHERE profile_id = ?")
                .bind(&id)
                .fetch_optional(&mut *connection)
                .await
                .with_context(|| format!("reading the home Profile {id} is kept under"))?;

        let (minted_on, minted_id) = (row.10.clone(), row.11);

        let profile = read_row(
            row,
            listed.into_iter().map(|(model,)| model).collect(),
            home.map(|(home,)| home),
        )?;

        let record = ProfileRecord {
            name: profile.name.clone(),
            harness: profile.agent_type().word().to_owned(),
            models: profile.models.clone(),
            memory: profile.memory,
            minted_on,
            minted_id,
        };

        for (kind, uuid, body) in [
            (Profiles::NAME, id.clone(), serde_json::to_string(&record)),
            (
                Bindings::NAME,
                binding_uuid(&id, &writer),
                serde_json::to_string(&Binding::of(&profile.account)?),
            ),
        ] {
            let body = body.with_context(|| format!("writing down Profile {id} as a record"))?;

            sqlx::query(
                "INSERT INTO replicated (kind, uuid, updated_at, deleted_at, written_by, body)
                 VALUES (?, ?, ?, NULL, ?, ?)
                 ON CONFLICT (kind, uuid) DO NOTHING",
            )
            .bind(kind)
            .bind(&uuid)
            .bind(stamp)
            .bind(&writer)
            .bind(body)
            .execute(&mut *connection)
            .await
            .with_context(|| format!("writing down Profile {id} as a record"))?;
        }

        for statement in [
            "DELETE FROM profile_models WHERE profile_id = ?",
            "DELETE FROM profile_homes WHERE profile_id = ?",
            "DELETE FROM profiles WHERE id = ?",
        ] {
            sqlx::query(statement)
                .bind(&id)
                .execute(&mut *connection)
                .await
                .with_context(|| format!("taking Profile {id} out of the profiles table"))?;
        }
    }

    Ok(())
}

/// Say which device this database is the device of — see [`here`] — and say
/// it over every record and binding written before it was known, or under an
/// id it has had since: the switch's, a suite's, and those of a device whose
/// id was issued again.
///
/// **This device's bindings follow it**: a binding is where *this machine*
/// keeps the account, so one kept under an id this device no longer goes by
/// is moved to the one it does.
pub(crate) async fn this_device_is(
    connection: &mut sqlx::SqliteConnection,
    device: &str,
) -> Result<()> {
    let was = here(&mut *connection).await?;

    if was != device {
        let suffix = format!("/{was}");

        let held: Vec<(String,)> = sqlx::query_as(
            "SELECT uuid FROM replicated
             WHERE kind = ? AND substr(uuid, -length(?)) = ?",
        )
        .bind(Bindings::NAME)
        .bind(&suffix)
        .bind(&suffix)
        .fetch_all(&mut *connection)
        .await
        .context("reading the bindings kept under this device's former id")?;

        for (uuid,) in held {
            let profile = &uuid[..uuid.len() - suffix.len()];

            sqlx::query(
                "UPDATE OR IGNORE replicated SET uuid = ?, written_by = ?
                 WHERE kind = ? AND uuid = ?",
            )
            .bind(binding_uuid(profile, device))
            .bind(device)
            .bind(Bindings::NAME)
            .bind(&uuid)
            .execute(&mut *connection)
            .await
            .with_context(|| format!("moving the binding {uuid} to this device's id"))?;
        }

        sqlx::query("UPDATE replicated SET written_by = ? WHERE kind = ? AND written_by = ?")
            .bind(device)
            .bind(Profiles::NAME)
            .bind(&was)
            .execute(&mut *connection)
            .await
            .context("saying which device wrote the Profiles written before it was known")?;

        sqlx::query(
            "INSERT INTO this_device (one, device) VALUES (1, ?)
             ON CONFLICT (one) DO UPDATE SET device = excluded.device",
        )
        .bind(device)
        .execute(&mut *connection)
        .await
        .context("saying which device this is")?;
    }

    // And where the Profiles numbered here before the uuids were minted, which
    // the switch could not say.
    sqlx::query(
        "UPDATE replicated SET body = json_set(body, '$.minted_on', ?)
         WHERE kind = ?
           AND json_extract(body, '$.minted_id') IS NOT NULL
           AND coalesce(json_extract(body, '$.minted_on'), '') = ''",
    )
    .bind(device)
    .bind(Profiles::NAME)
    .execute(&mut *connection)
    .await
    .context("saying which device the Profiles numbered before their uuids were minted on")?;

    Ok(())
}

/// Say what this device is called — its name as the Devices pane draws it —
/// and name its own unnamed Profiles after it where it has members and has
/// not yet: what the serve says once it knows both which device this is and
/// what it is called. **The switch on a device that has members** is this:
/// its first serve on this release, with the members it already had.
///
/// The uuids of the Profiles named, for the members to be told.
pub async fn this_device_called(pool: &SqlitePool, name: &str) -> Result<Vec<String>> {
    let mut tx = super::writing(pool, "saying what this device is called").await?;

    sqlx::query(
        "INSERT INTO this_device_called (one, name) VALUES (1, ?)
         ON CONFLICT (one) DO UPDATE SET name = excluded.name",
    )
    .bind(name)
    .execute(&mut *tx)
    .await
    .context("saying what this device is called")?;

    let named = named_after_here(&mut tx).await?;

    tx.commit()
        .await
        .context("naming this device's unnamed Profiles after it")?;

    Ok(named)
}

/// **Name this device's own unnamed Profiles after it, once** (ADR-0028): two
/// unnamed Claude Profiles are two records nothing merges, so a cluster whose
/// every device brought one would draw *Default* once per device in every
/// picker. Asked as the serve says the name — see [`this_device_called`] —
/// and as a member is recorded, inside that write: a device going from no
/// members to one is a device first linking, whichever side of the link it is
/// on, and an Annex kept is linked by the same write.
///
/// **Nothing at all** on a device with no members, which leaves its unnamed
/// Profiles as they are; on one the serve has not said a name for yet, which
/// it does once it has; and on one that has been through this already — so a
/// name the human cleared afterwards stays cleared across a restart and every
/// later link. That one goes through once the device has members, whether or
/// not anything was unnamed.
///
/// **Its own** is a live record this device holds a live binding of: one that
/// arrived by replication is bound on the device that made it, and is that
/// device's to name. Every unnamed one takes the one name whatever its harness,
/// the name rule being per harness; and a name the table already holds is not
/// in its way, the rules being the press's — two alike are kept and drawn so,
/// as a pair arriving by replication is.
///
/// An ordinary record write, so it replicates. The uuids named come back.
pub(crate) async fn named_after_here(
    connection: &mut sqlx::SqliteConnection,
) -> Result<Vec<String>> {
    let called: Option<(String, bool)> =
        sqlx::query_as("SELECT name, named FROM this_device_called")
            .fetch_optional(&mut *connection)
            .await
            .context("reading what this device is called")?;

    let Some((name, false)) = called else {
        return Ok(Vec::new());
    };

    let (members,): (i64,) = sqlx::query_as("SELECT count(*) FROM members")
        .fetch_one(&mut *connection)
        .await
        .context("counting the devices this one is linked to")?;

    if members == 0 {
        return Ok(Vec::new());
    }

    let here = here(&mut *connection).await?;
    let mut named = Vec::new();

    for record in replicated::records_on::<Profiles>(&mut *connection).await? {
        if record.removed() || record.fields.name.is_some() {
            continue;
        }

        let bound =
            replicated::record_on::<Bindings>(&mut *connection, &binding_uuid(&record.uuid, &here))
                .await?
                .is_some_and(|binding| !binding.removed());

        if !bound {
            continue;
        }

        let fields = ProfileRecord {
            name: Some(name.clone()),
            ..record.fields
        };

        replicated::written_on::<Profiles>(&mut *connection, &here, &record.uuid, &fields, false)
            .await?;

        named.push(record.uuid);
    }

    sqlx::query("UPDATE this_device_called SET named = 1")
        .execute(&mut *connection)
        .await
        .context("saying this device's unnamed Profiles have been named after it")?;

    Ok(named)
}
