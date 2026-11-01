//! Managing Agent Profiles: everything between a workbench form and a row in
//! the store, and the check that says whether a saved one can still be run
//! under.
//!
//! The account is judged against the filesystem, and that judging is the
//! server's: a form that checked it would be a courtesy, since this endpoint is
//! reachable without one. Anywhere the server can read is somewhere an account
//! can be kept — the human's own login under `~` included, which is the account a
//! Profile most naturally names. What an account *is* — Claude's pair, and the
//! single home each backend after it keeps one under — is the agent type's, and
//! every judgement here is made per type for that reason.
//!
//! **And a Profile may be bound on other devices alone**, which is the one
//! row here nothing is judged about against this filesystem: every device
//! holds the one record (ADR-0028), and the account paths on a row this device
//! holds no binding of are another machine's — see [`store::Unbound`]. So it
//! is answered out of the record and out of this device.
//!
//! **Which does not make such a Profile unbreakable**, it makes it break over
//! different things: the two that would stop a launch here, asked in the order
//! a launch would meet them. Whether a device it is bound on, whose account
//! there holds a login, is answering at all, and whether the harness is on
//! *this* machine. Each is named on the row and each refuses a Start by name,
//! before a session comes up logged out with nothing saying why — see
//! [`broken`], where the two are decided, and [`verkstead_render::Broken`],
//! which is the vocabulary they are said in. **Per device**: every device
//! answers them for itself.
//!
//! **Whether a bound account holds a login is said by the device it is bound
//! on**, the one that can look: every read here looks at this device's own
//! bound accounts and writes what it saw onto their bindings where that moved,
//! which replicate — see [`looked`].
//!
//! Nothing here mounts anything. A Profile is a record of an account a session
//! will later be run under; the bind-mounting arrives with the stage that runs
//! one.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{
    AgentType, Broken, PairingView, PickedView, ProfileAccount, ProfileChoice, ProfileChosen,
    ProfileDeleted, ProfileDevice, ProfileEdit, ProfileEntry, ProfileSaved, RoleChoice, RowDevice,
};

use crate::resolved::{Resolved, resolve};
use crate::{AppState, store};

/// Save a new Profile, or say why not.
pub(crate) async fn create(pool: &SqlitePool, edit: &ProfileEdit) -> Result<ProfileSaved> {
    let (facts, login) = match checked(edit).await? {
        Ok(checked) => checked,
        Err(refusal) => return Ok(refusal),
    };

    Ok(
        match store::create_profile_holding(pool, &facts, login).await? {
            Ok(_) => ProfileSaved::Saved,
            Err(store::Clash::NameTaken) => ProfileSaved::NameTaken,
            Err(store::Clash::DefaultTaken) => ProfileSaved::DefaultTaken,
        },
    )
}

/// Rewrite one, whole. Everything about a Profile is the human's to change:
/// nothing has been built from it that a change would contradict.
///
/// **A Profile this device holds no binding of is rewritten as a record
/// alone**: its paths are other devices', so what the form carries of them is
/// neither judged against this filesystem nor written, and binding this
/// device is a press of its own.
pub(crate) async fn edit(pool: &SqlitePool, id: &str, edit: &ProfileEdit) -> Result<ProfileSaved> {
    let Some(held) = store::load_profile(pool, id).await? else {
        return Ok(ProfileSaved::NoSuchProfile);
    };

    let saving = match held.unbound {
        None => match checked(edit).await? {
            Ok((facts, login)) => store::update_profile_holding(pool, id, &facts, login).await?,
            Err(refusal) => return Ok(refusal),
        },

        Some(_) => match named(edit) {
            Ok((name, models)) => {
                let facts = store::ProfileFacts {
                    name,
                    account: held.account,
                    models,
                    memory: edit.memory,
                };

                store::update_profile(pool, id, &facts).await?
            }
            Err(refusal) => return Ok(refusal),
        },
    };

    Ok(match saving {
        store::Saving::Saved => ProfileSaved::Saved,
        store::Saving::NoSuchProfile => ProfileSaved::NoSuchProfile,
        store::Saving::NameTaken => ProfileSaved::NameTaken,
        store::Saving::DefaultTaken => ProfileSaved::DefaultTaken,
    })
}

/// Remove one, whoever had chosen it.
///
/// Nothing is judged here, which is why there is so little of it: the store
/// takes the Profile out of every Pairing that named it in the one transaction
/// that removes it, and a Conversation left with an empty picker is read as one
/// that has not chosen — see [`picked`].
pub(crate) async fn remove(pool: &SqlitePool, id: &str) -> Result<ProfileDeleted> {
    Ok(match store::delete_profile(pool, id).await? {
        store::Deleting::Deleted => ProfileDeleted::Removed,
        store::Deleting::NoSuchProfile => ProfileDeleted::NoSuchProfile,
    })
}

/// Every Profile, each with whether its account is still where it was left —
/// **every live record, bound here or not**.
///
/// One list rather than a section per device (ADR-0028): a Profile is the
/// cluster's, and no device is its home.
pub(crate) async fn listed(state: &AppState) -> Result<Vec<ProfileEntry>> {
    entries(state, store::profiles(&state.pool).await?).await
}

/// The same reading for one Pairing, for the panes that show a Conversation's
/// own choices rather than the whole list.
pub(crate) async fn pairing(
    state: &AppState,
    pairing: Option<store::Pairing>,
) -> Result<Option<PairingView>> {
    let Some(pairing) = pairing else {
        return Ok(None);
    };

    let model = pairing.model;

    Ok(entries(state, vec![pairing.profile])
        .await?
        .pop()
        .map(|profile| PairingView { profile, model }))
}

/// And the same reading for a batch of Pairings that each belong to something,
/// by whatever that something is keyed by.
///
/// One hop off the runtime for the lot of them, which is the whole of why this
/// is not [`pairing`] in a loop: every Profile read looks at the filesystem, and
/// a Timeline whose human has steered a dozen times would otherwise be a dozen
/// hops. What it is for is the steers on a Timeline — see `crate::ui`, which
/// reads them all before it draws any.
pub(crate) async fn keyed(
    state: &AppState,
    pairings: Vec<(i64, store::Pairing)>,
) -> Result<HashMap<i64, PairingView>> {
    let (keys, profiles): (Vec<i64>, Vec<store::Profile>) = pairings
        .iter()
        .map(|(key, pairing)| (*key, pairing.profile.clone()))
        .unzip();

    let models = pairings.into_iter().map(|(_, pairing)| pairing.model);

    Ok(keys
        .into_iter()
        .zip(entries(state, profiles).await?)
        .zip(models)
        .map(|((key, profile), model)| (key, PairingView { profile, model }))
        .collect())
}

/// And the same reading for a whole choice, for the one role that can be picked
/// away as well as paired.
///
/// The three states come across unchanged: a Pairing is read as one, the row
/// that runs no session stays what it is, and a Profile whose row has gone
/// reads as nothing picked — which is what it is, since there is no account
/// left to launch under.
pub(crate) async fn picked(state: &AppState, picked: store::Picked) -> Result<PickedView> {
    Ok(match picked {
        store::Picked::Nothing => PickedView::Nothing,
        store::Picked::Skipped => PickedView::Skipped,
        store::Picked::Under(under) => match pairing(state, Some(under)).await? {
            Some(pairing) => PickedView::Under(pairing),
            None => PickedView::Nothing,
        },
    })
}

/// Look at whether the account of each Profile bound here holds a login file,
/// and say so on this device's binding of it where that moved — see
/// [`store::logins_held`] — telling the members to pull where it did.
///
/// **The device bound on an account is the only one that can look**, and a
/// device running the Profile from a copy fetches only from a binding that
/// holds one. So it is asked on every read of this device's Profiles, at the
/// start, and as a login written back by a member lands; a read where nothing
/// moved writes nothing.
pub(crate) async fn looked(state: &AppState, profiles: &[store::Profile]) {
    let bound: Vec<store::Profile> = profiles
        .iter()
        .filter(|profile| profile.unbound.is_none())
        .cloned()
        .collect();

    if bound.is_empty() {
        return;
    }

    let held = tokio::task::spawn_blocking(move || {
        bound
            .iter()
            .map(|profile| (profile.id.clone(), lends(profile)))
            .collect::<Vec<_>>()
    })
    .await;

    let Ok(held) = held else {
        return;
    };

    match store::logins_held(&state.pool, &held).await {
        Ok(true) => state.nudges.announce(verkstead_schema::Nudge::Profiles),
        Ok(false) => {}
        Err(why) => tracing::warn!(
            error = ?why,
            "whether this device's accounts hold a login could not be written onto their \
             bindings, so a member may fetch from one that has none",
        ),
    }
}

/// And the same for every Profile this device holds, read for it.
pub(crate) async fn looked_at_all(state: &AppState) {
    match store::profiles(&state.pool).await {
        Ok(profiles) => looked(state, &profiles).await,
        Err(why) => tracing::warn!(
            error = ?why,
            "the Profiles could not be read to look at whether their accounts hold a login",
        ),
    }
}

/// Read a batch of Profiles into rows, looking at the filesystem once for the
/// lot of them.
///
/// Off the runtime, because every one of those looks is a blocking read — and
/// together rather than one call per Profile, since the whole point of the batch
/// is that it is one hop off the runtime instead of a list's worth.
///
/// **And the membership, read once beside them**, for whether a device a
/// launch of one not bound here would fetch from is answering — see
/// [`broken`] — and for the card's line per device: see [`Lines`].
///
/// **And the machine itself goes in with them**, for the one thing such a
/// Profile is judged against here: whether the harness it runs is installed on
/// this box. One probe for the batch and one `PATH` walk per harness in it,
/// inside the same blocking hop the paths are resolved in.
async fn entries(state: &AppState, profiles: Vec<store::Profile>) -> Result<Vec<ProfileEntry>> {
    looked(state, &profiles).await;

    let members = store::members(&state.pool).await?;
    let lines = Lines::read(state, &members).await?;
    let members: HashMap<String, store::Member> = members
        .into_iter()
        .map(|member| (member.device.clone(), member))
        .collect();
    let machine = state.onboarding.machine().clone();
    let signed_out = state.logins.signed_out();

    Ok(tokio::task::spawn_blocking(move || {
        // Whether each harness a Profile not bound here runs is on this
        // machine, asked once apiece: a cluster's Profiles section holds every
        // device's accounts, and four walks are not one per row.
        let mut harnesses: HashMap<store::AgentType, bool> = HashMap::new();

        for profile in &profiles {
            if profile.unbound.is_some() {
                let agent_type = profile.agent_type();

                harnesses
                    .entry(agent_type)
                    .or_insert_with(|| machine.harness(agent_type));
            }
        }

        profiles
            .into_iter()
            .map(|profile| {
                // Whether a device a launch could fetch from is answering, for
                // one not bound here: one bound with a login, and none being
                // none.
                let reachable = profile.unbound.as_ref().is_none_or(|unbound| {
                    unbound
                        .lending
                        .iter()
                        .any(|device| whose(&members, device).reachable)
                });

                ProfileEntry {
                    id: profile.id.clone(),
                    broken: broken(&profile, reachable, &harnesses, &signed_out),
                    login: lends(&profile),
                    bound: profile.unbound.is_none(),
                    name: profile.name,
                    account: account(&profile.account),
                    models: profile.models,
                    memory: profile.memory,
                    devices: lines.of(&profile.id),
                }
            })
            .collect()
    })
    .await?)
}

/// What a Profile card's line per device is drawn off: every device of the
/// cluster, this one first and then the members in the order the Devices
/// section lists them, and where each Profile is bound.
///
/// **Nothing at all on a Verkstead in no cluster**, and on a router with no
/// identity: one device is the only place anything could be bound, and a line
/// would say what the card already does. The bindings are read only where
/// there is a line to draw.
struct Lines {
    /// What each device is drawn as, by the Device Id its bindings are
    /// written under — this device's own first, with no id.
    devices: Vec<(String, RowDevice)>,
    bound: store::BoundWhere,
}

impl Lines {
    async fn read(state: &AppState, members: &[store::Member]) -> Result<Self> {
        let (Some(here), false) = (state.devices.as_ref(), members.is_empty()) else {
            return Ok(Self {
                devices: Vec::new(),
                bound: store::BoundWhere::default(),
            });
        };

        let bound = store::bound_where(&state.pool).await?;

        // This device drawn as the Devices section draws it, and each member
        // as it last answered.
        let this = RowDevice {
            id: None,
            name: here.name(),
            os: Some(here.os()),
            reachable: true,
        };

        let devices = std::iter::once((bound.here.clone(), this))
            .chain(members.iter().map(|member| {
                (
                    member.device.clone(),
                    RowDevice {
                        id: Some(member.device.clone()),
                        name: member.name.clone(),
                        os: Some(member.os.clone()),
                        reachable: member.reachable,
                    },
                )
            }))
            .collect();

        Ok(Self { devices, bound })
    }

    /// The lines of the Profile `id`: a binding's account on every device
    /// holding one, and *runs from a copy* on every other.
    fn of(&self, id: &str) -> Vec<ProfileDevice> {
        let bound = self
            .bound
            .bound
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_default();

        self.devices
            .iter()
            .map(|(device, row)| ProfileDevice {
                device: row.clone(),
                bound: bound
                    .iter()
                    .find(|(on, _)| on == device)
                    .map(|(_, held)| account(held)),
            })
            .collect()
    }
}

/// Whether the account this row names holds a **login file** — the fact that
/// says whether it can be used on a device it is not bound on at all.
///
/// One bound here is looked at here, at the one path the harness keeps a login
/// under. One bound elsewhere holds one where any device's binding of it says
/// so — see [`store::Unbound::lending`].
///
/// A login kept somewhere that is not a file — a Claude login in the macOS
/// Keychain — reads the same as a sign-out, which is what it is from here: there
/// is nothing to hand another machine. Neither stops a session here, which is
/// why this is a fact beside [`ProfileEntry::broken`] rather than a way of being
/// it.
fn lends(profile: &store::Profile) -> bool {
    match &profile.unbound {
        Some(unbound) => !unbound.lending.is_empty(),
        None => holds_login(&profile.account),
    }
}

/// Whether an account at these paths holds its login file, looked at here.
fn holds_login(account: &store::Account) -> bool {
    matches!(
        resolve(&crate::sandbox::root::login_at(account)),
        Resolved::At(_)
    )
}

/// How a device a Profile is bound on is read, off the membership.
///
/// **A device the membership does not hold still reads**, under the id itself
/// and as not answering: a binding outlives the link to the device that wrote
/// it, and a reading with no name at all would be worse than one naming a
/// machine by its id.
pub(crate) fn whose(members: &HashMap<String, store::Member>, device: &str) -> RowDevice {
    match members.get(device) {
        Some(member) => RowDevice {
            id: Some(member.device.clone()),
            name: member.name.clone(),
            os: Some(member.os.clone()),
            reachable: member.reachable,
        },

        None => RowDevice {
            id: Some(device.to_owned()),
            name: device.to_owned(),
            os: Some(String::new()),
            reachable: false,
        },
    }
}

/// Whether everything the next stage needs before it will start is settled.
///
/// Every Pairing complete — a Profile *and* a model — and no Profile broken.
/// Broken as well as chosen because a pair that has gone is a session that
/// fails to start, and the whole reason this is answered here is to say so
/// while the human is looking.
///
/// A Profile chosen before pairings existed has no model and so is not a
/// Pairing: while a Conversation is drafting that is a choice to make again,
/// which is what this says. Nothing asks it of a Conversation past drafting —
/// there is no start left to be ready for — so the carried model this would
/// refuse is never the one that runs anything.
///
/// **A role picked away is settled**, and settles this: there is no session to
/// fail to start, so a Conversation that will not be reviewed is as ready as one
/// that will. What it is not is an empty picker — see
/// [`verkstead_render::PickedView`]. Which leaves the review the one role read
/// that way: the grilling picker's own such row retired with *No grilling*, so
/// that role is answered with an account or not answered.
pub(crate) fn ready_to_grill(
    grilling: Option<&PairingView>,
    implementation: Option<&PairingView>,
    review: &PickedView,
) -> bool {
    runnable(grilling) && ready_to_wrap(implementation, review)
}

/// And the same with the grilling left out of it, which is what a Process that
/// is never interviewed waits on: a **Tinker**, whose composer draws no Grilling
/// picker at all.
///
/// The reading `unready_to_wrap` in [`crate::conversations`] takes for the
/// press, said the way the pane wants it — one of them is what decides whether
/// to offer the button and the other is what was wrong when it was pressed, and
/// they have to be the one question.
pub(crate) fn ready_to_wrap(implementation: Option<&PairingView>, review: &PickedView) -> bool {
    runnable(implementation) && settled(review)
}

/// And the same with the review left out of it as well, which is what a Process
/// run under the Implementation role alone waits on: an **Investigate**, whose
/// composer draws the one **Agent** dropdown and no other picker.
///
/// The one role every Process uses and no Process can pick away — there is no
/// work without something doing it — so this is [`runnable`] under the name of
/// the question it answers. Said here beside the other two rather than called
/// directly, because what a press asks and what the pane asks have to be the one
/// reading: `unready_to_investigate` in [`crate::conversations`] is the other
/// half of it.
pub(crate) fn ready_to_investigate(implementation: Option<&PairingView>) -> bool {
    runnable(implementation)
}

/// Whether the one role that can be picked away is settled: a Pairing something
/// could be launched under, or the row that launches nothing.
fn settled(picked: &PickedView) -> bool {
    match picked {
        PickedView::Nothing => false,
        PickedView::Skipped => true,
        PickedView::Under(pairing) => runnable(Some(pairing)),
    }
}

/// Whether one picked Pairing is something a session could be launched under.
fn runnable(pairing: Option<&PairingView>) -> bool {
    pairing.is_some_and(|pairing| pairing.model.is_some() && pairing.profile.broken.is_none())
}

/// Why this Profile cannot be run under as things stand, or `None` while its
/// account is where it was left.
///
/// Each of an account's paths is named by what it is, for the reason the
/// refusals are: a Profile whose config file has gone and one whose directory
/// has gone are two different things to go and put right.
///
/// Asked by resolving rather than by asking whether the path exists, so that
/// what is checked here and what was checked at the save are the one question:
/// a dangling symlink where the account was is a Profile that cannot be run
/// under.
///
/// **A Profile not bound here is answered before any of that and asks two
/// other questions.** The paths on it are another machine's, so resolving them
/// here would be asking the wrong filesystem a question it may well answer
/// *yes* to — two machines set up alike hold the same paths, and the account
/// under them is somebody else's. What a session here is given is not those
/// paths either: the login and the configuration a Built Root is made from are
/// fetched from a device it is bound on into a copy under this device's Data
/// Directory before each launch. See [`crate::mirroring::account`].
///
/// So what is asked instead is the two things that would stop *that*, **in the
/// order a launch would meet them**: a bound device is dialled and the account
/// fetched out of it, and the harness is run here.
///
/// 1. **No device it is bound on whose binding holds a login is answering** —
///    [`Broken::NoBoundDevice`], naming none. Nothing can be fetched, so
///    nothing can be built. This device's own reading of its membership is
///    what says so, which is the same reading the sidebar draws a member
///    unreachable off, and `reachable` is it. A binding with no login is no
///    source — see [`lends`].
/// 2. **The harness is not on this machine** — [`Broken::HarnessMissing`]. The
///    onboarding probe's own finding, in that step's own word.
///
/// **And that is the whole of it** for such a Profile: a row that passes both
/// is a row to run under, and there is nothing about this filesystem to go and
/// put right.
///
/// **One bound here may also have signed out** — [`Broken::SignedOut`] — where
/// a press found its account with no login. Asked after its paths, which a
/// login could not put right. `signed_out` is what [`crate::logins::asked`]
/// found.
fn broken(
    profile: &store::Profile,
    reachable: bool,
    harnesses: &HashMap<store::AgentType, bool>,
    signed_out: &HashSet<String>,
) -> Option<Broken> {
    if profile.unbound.is_some() {
        if !reachable {
            return Some(Broken::NoBoundDevice);
        }

        // Absent from the map is a batch nothing probed, which cannot happen
        // for a row not bound here — [`entries`] fills it off the same rows.
        // Read as *there* rather than as broken, a Profile refused for a probe
        // nobody made being worse than one offered and found wanting.
        if !harnesses
            .get(&profile.agent_type())
            .copied()
            .unwrap_or(true)
        {
            return Some(Broken::HarnessMissing);
        }

        return None;
    }

    let paths: Vec<(PathBuf, Broken)> = match &profile.account {
        store::Account::Claude {
            claude_dir,
            config_file,
        } => vec![
            (claude_dir.clone(), Broken::DirMissing),
            (config_file.clone(), Broken::ConfigMissing),
        ],
        store::Account::Codex { home } | store::Account::Grok { home } => {
            vec![(home.clone(), Broken::HomeMissing)]
        }

        // An OpenCode home is judged by the two directories opencode keeps an
        // account in rather than by the directory holding them: those are what
        // is mounted, and a home that has never had opencode run in it has
        // neither. See [`crate::sandbox::OPENCODE_CONFIG_INSIDE_HOME`].
        store::Account::OpenCode { home } => {
            xdg(home).map(|path| (path, Broken::HomeMissing)).collect()
        }
    };

    for (path, missing) in paths {
        if !matches!(resolve(&path), Resolved::At(_)) {
            return Some(missing);
        }
    }

    signed_out
        .contains(&profile.id)
        .then_some(Broken::SignedOut)
}

/// What the human typed, checked — or the reason it is not going to be saved.
///
/// The filesystem half runs off the runtime for the reason a registration's
/// does: resolving a path is blocking, and a save is rare enough that the thread
/// it borrows costs nothing.
///
/// **And whether the account holds a login file**, looked at in the same hop,
/// for the save to say on the binding it writes — see
/// [`store::create_profile_holding`]. Said by [`looked`] on the next read
/// instead, a member could pull the binding in between and read an account
/// with no login as one to fetch from.
async fn checked(edit: &ProfileEdit) -> Result<Result<(store::ProfileFacts, bool), ProfileSaved>> {
    let (name, models) = match named(edit) {
        Ok(named) => named,
        Err(refusal) => return Ok(Err(refusal)),
    };

    let account = edit.account.clone();
    let memory = edit.memory;

    Ok(tokio::task::spawn_blocking(move || {
        inspect(&account).map(|account| {
            let login = holds_login(&account);

            (account, login)
        })
    })
    .await?
    .map(|(account, login)| {
        (
            store::ProfileFacts {
                name,
                account,
                models,
                memory,
            },
            login,
        )
    }))
}

/// The name and the models the human typed, tidied — or the reason they are
/// not going to be saved. Everything of a save but the account.
fn named(edit: &ProfileEdit) -> Result<(Option<String>, Vec<String>), ProfileSaved> {
    // Trimmed, and a box with nothing but space in it is no name at all: a
    // Profile may go unnamed, and one called `" "` would be a name nobody could
    // tell from none.
    let name = edit
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned);

    // Trimmed and the blanks dropped: the form hands these over a line apiece,
    // and a trailing newline is not a model. Their order is kept, because it is
    // the order the human typed and reading the list back should say so.
    let models: Vec<String> = edit
        .models
        .iter()
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty())
        .collect();

    if models.is_empty() {
        return Err(ProfileSaved::Modelless);
    }

    Ok((name, models))
}

/// The account, resolved — or the reason it is refused.
///
/// One arm per agent type, because what a Profile names is that type's own
/// shape: Claude's pair here, and the single home each backend after it keeps
/// its whole account under when its stage lands.
///
/// Every path is judged the same way and refused by its own name: pointing the
/// config field at the directory is an easy mistake, and "that path is wrong"
/// would not say which one.
fn inspect(account: &ProfileAccount) -> Result<store::Account, ProfileSaved> {
    match account {
        ProfileAccount::Claude {
            claude_dir,
            config_file,
        } => {
            let dir = match resolve(Path::new(claude_dir.trim())) {
                Resolved::At(path) => path,
                Resolved::NotAbsolute => return Err(ProfileSaved::DirNotAbsolute),
                Resolved::Missing => return Err(ProfileSaved::DirMissing),
            };

            if !dir.is_dir() {
                return Err(ProfileSaved::NotADirectory);
            }

            let config = match resolve(Path::new(config_file.trim())) {
                Resolved::At(path) => path,
                Resolved::NotAbsolute => return Err(ProfileSaved::ConfigNotAbsolute),
                Resolved::Missing => return Err(ProfileSaved::ConfigMissing),
            };

            if !config.is_file() {
                return Err(ProfileSaved::NotAFile);
            }

            Ok(store::Account::Claude {
                claude_dir: dir,
                config_file: config,
            })
        }

        ProfileAccount::Codex { home } => Ok(store::Account::Codex { home: kept(home)? }),

        ProfileAccount::Grok { home } => Ok(store::Account::Grok { home: kept(home)? }),

        // An OpenCode home is the directory the two opencode keeps an account
        // in sit under, so both of those are judged as well as the home itself
        // — they are what a session mounts, and a directory that has never had
        // opencode run in it holds neither. Each is judged by the same rule and
        // refused in the same words, because for this type they *are* the home
        // the account is kept under.
        ProfileAccount::OpenCode { home } => {
            let home = kept(home)?;

            for path in xdg(&home) {
                directory(&path)?;
            }

            Ok(store::Account::OpenCode { home })
        }
    }
}

/// The two directories an OpenCode account is kept in, under the home a Profile
/// names.
///
/// opencode reads the XDG base directories rather than keeping a dot-directory
/// of its own, and a Profile's home is a home to resolve them inside — see
/// [`crate::sandbox::OPENCODE_CONFIG_INSIDE_HOME`], which is where the same two
/// relative paths are bound from and to.
fn xdg(home: &Path) -> impl Iterator<Item = PathBuf> {
    [
        crate::sandbox::OPENCODE_CONFIG_INSIDE_HOME,
        crate::sandbox::OPENCODE_DATA_INSIDE_HOME,
    ]
    .map(|inside| home.join(inside))
    .into_iter()
}

/// The one directory a type whose whole account is one home named, judged the
/// way each of Claude's pair is.
///
/// One check for every such type rather than one apiece: what differs between
/// them is which account the directory is, and that is the arm above, not this.
fn kept(home: &str) -> Result<PathBuf, ProfileSaved> {
    directory(Path::new(home.trim()))
}

/// The same, of a path rather than of something typed into the form.
///
/// What an OpenCode home holds is judged this way: the two directories under it
/// are derived rather than typed, so there is nothing to trim off either.
fn directory(home: &Path) -> Result<PathBuf, ProfileSaved> {
    let home = match resolve(home) {
        Resolved::At(path) => path,
        Resolved::NotAbsolute => return Err(ProfileSaved::HomeNotAbsolute),
        Resolved::Missing => return Err(ProfileSaved::HomeMissing),
    };

    if !home.is_dir() {
        return Err(ProfileSaved::HomeNotADirectory);
    }

    Ok(home)
}

/// Choose the Pairing a Conversation's grilling session will run under.
///
/// An account and nothing else, the picker having no row that is not one: *No
/// grilling* is retired, and a Brief that wants no interview is a **Tinker**
/// Conversation.
pub(crate) async fn choose_grilling(
    pool: &SqlitePool,
    id: i64,
    choice: &ProfileChoice,
) -> Result<ProfileChosen> {
    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refused(refusal));
    }

    Ok(chosen(
        store::set_grilling_pairing(pool, id, &choice.profile_id, Some(&choice.model)).await?,
    ))
}

/// And the one its implementation will run under.
pub(crate) async fn choose_implementation(
    pool: &SqlitePool,
    id: i64,
    choice: &ProfileChoice,
) -> Result<ProfileChosen> {
    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refused(refusal));
    }

    Ok(chosen(
        store::set_implementation_pairing(pool, id, &choice.profile_id, Some(&choice.model))
            .await?,
    ))
}

/// And the one the wrap-up's review session will run under — or the row that
/// says there is to be no review session at all.
///
/// One press either way, because the picker offers them as one list: nothing is
/// judged about a review that runs nothing, there being no Profile to have gone
/// and no model to have been retyped.
pub(crate) async fn choose_review(
    pool: &SqlitePool,
    id: i64,
    choice: &RoleChoice,
) -> Result<ProfileChosen> {
    let Some(choice) = &choice.pairing else {
        return Ok(chosen(store::skip_review(pool, id).await?));
    };

    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refused(refusal));
    }

    Ok(chosen(
        store::set_review_pairing(pool, id, &choice.profile_id, Some(&choice.model)).await?,
    ))
}

/// What is wrong with a picked Pairing, where anything is.
///
/// Its own two words rather than one of the outcome types around it, because two
/// presses ask this and each answers in a vocabulary of its own: the drafting
/// pickers answer as [`ProfileChosen`], and a steer answers as
/// [`verkstead_render::ConversationSteered`]. The question is the same either
/// way — is this still a Profile, and does it still list that model?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unlisted {
    /// There is no Profile with that id: it was removed between the list the
    /// page read and the pick it made from it.
    NoSuchProfile,

    /// That Profile does not list that model, for the same reason.
    NoSuchModel,
}

/// Why this is not a Pairing to record, or `None` where the Profile lists the
/// model it was picked with.
///
/// Asked here rather than in the store, because it is a question about a Profile
/// read as a row: a Profile written before the model list existed carries its
/// one model in the old column, and a statement that checked `profile_models`
/// alone would refuse the only model that Profile has.
pub(crate) async fn unlisted(
    pool: &SqlitePool,
    choice: &ProfileChoice,
) -> Result<Option<Unlisted>> {
    let Some(profile) = store::load_profile(pool, &choice.profile_id).await? else {
        return Ok(Some(Unlisted::NoSuchProfile));
    };

    Ok((!profile.models.contains(&choice.model)).then_some(Unlisted::NoSuchModel))
}

/// What is wrong with a pick, as the drafting pickers say it.
fn refused(unlisted: Unlisted) -> ProfileChosen {
    match unlisted {
        Unlisted::NoSuchProfile => ProfileChosen::NoSuchProfile,
        Unlisted::NoSuchModel => ProfileChosen::NoSuchModel,
    }
}

/// The store's outcome as the viewer receives it. One word either side, and this
/// is where the two vocabularies are held to each other.
fn chosen(chosen: store::Chosen) -> ProfileChosen {
    match chosen {
        store::Chosen::Chosen => ProfileChosen::Chosen,
        store::Chosen::NoSuchConversation => ProfileChosen::NoSuchConversation,
        store::Chosen::NoSuchProfile => ProfileChosen::NoSuchProfile,
        store::Chosen::NotDrafting => ProfileChosen::NotDrafting,
    }
}

/// The store's account as the viewer receives it, held to the wire's the same
/// way: one arm per agent type, and the paths that type has.
///
/// The paths are stored as UTF-8 in the first place — one that is not cannot be
/// saved — so nothing is lost putting them back on the wire.
pub(crate) fn account(account: &store::Account) -> ProfileAccount {
    match account {
        store::Account::Claude {
            claude_dir,
            config_file,
        } => ProfileAccount::Claude {
            claude_dir: claude_dir.to_string_lossy().into_owned(),
            config_file: config_file.to_string_lossy().into_owned(),
        },
        store::Account::Codex { home } => ProfileAccount::Codex {
            home: home.to_string_lossy().into_owned(),
        },
        store::Account::Grok { home } => ProfileAccount::Grok {
            home: home.to_string_lossy().into_owned(),
        },
        store::Account::OpenCode { home } => ProfileAccount::OpenCode {
            home: home.to_string_lossy().into_owned(),
        },
    }
}

/// What a Profile is called where a name has to be said and it has none.
///
/// A Profile may go unnamed, and most readings of one say nothing at all where
/// the harness's mark and the model already say the whole of what it is. This is
/// for the readings that cannot: prose about the account a run was spending has
/// a hole in it where a name was supposed to be, and *the account **Default***
/// is the sentence that is left. The viewer says the same word in the same
/// places — see `web/src/agents.ts`.
pub(crate) fn shown(name: Option<&str>) -> &str {
    name.unwrap_or("Default")
}

/// And the agent type on its own, for the Timeline: what a session ran under is
/// recorded without an account, the Profile it came from being a thing that can
/// since have gone.
pub(crate) fn agent_type(agent_type: store::AgentType) -> AgentType {
    match agent_type {
        store::AgentType::Claude => AgentType::Claude,
        store::AgentType::Codex => AgentType::Codex,
        store::AgentType::Grok => AgentType::Grok,
        store::AgentType::OpenCode => AgentType::OpenCode,
    }
}
