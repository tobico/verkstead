//! The **account mirror**: the login and the configuration of a member's
//! account, fetched from a device it is bound on into a directory of this
//! device's own under the Data Directory, so that a session launched here under
//! a Profile not bound here is given a Built Root made out of it (ADR-0020,
//! *Shared Profiles*; ADR-0028).
//!
//! **The record replicates and the account does not.** A Profile's record carries
//! what it is drawn and picked by — see [`super`] — and none of it is an account
//! a session could run as. What a session that runs from a copy gets is a copy of
//! the little of an account a root is made from: the login, and what the written
//! configuration is composed from. The answering half is
//! [`crate::peer::account`], a member's own route on the Peer Listener behind the
//! Member Gate, and the one list of what a root is made from is
//! [`crate::sandbox::root::mirrored`].
//!
//! **Per Profile, under the Data Directory**, which is what makes Windows' rule
//! hold by construction: the profile a session's root is built in is under that
//! same directory, so the hard link joining the login into it never crosses a
//! volume and the check that refuses an account elsewhere cannot fire for a
//! mirror — see [`crate::sandbox::Homes::account_mirror`].
//!
//! **Fetched before each launch**, rather than once and kept. A login refreshed
//! on the device it is bound on since the last session is the one this session
//! has to be given, and a mirror left to go stale would sign a session out for
//! no reason anybody could see. What that device says is written over what is
//! there, and every file of the allowlist it did *not* send is taken away — a
//! login that has gone there is a login gone here.
//!
//! **Written rather than emptied and made again.** Two Conversations may be
//! running under the one member's account, and on Linux the login is a bind into
//! each of their sessions: a directory swept away underneath them would be a
//! session logged out mid-run. So each file is written beside and renamed over,
//! which is how the harnesses themselves save one, and a running session keeps
//! the file it started with.
//!
//! **And downstream of the launch nothing is special.** The mirror directory *is*
//! the account as far as the root is concerned — its four parts, the trust seeded
//! into the copied config, the launch line and the model it runs are what they
//! already are for an account on this machine. See
//! [`crate::sessions::Sessions::start`], which is where a launch asks for this,
//! and [`crate::sandbox::kept_in`], which is the shape the mirror is written in.
//!
//! **And the login goes back as the session ends.** It is the one file of a root a
//! session genuinely changes — the harness refreshes its OAuth pair as it works —
//! so an account lent out and never written back would be an account signing
//! itself out a session at a time. What the session left is read off the mirror
//! once the ending has written everything back into it, and put to the device it
//! was fetched from where it is not still what came down: replaced or made where
//! there was none goes back whole, untouched goes nowhere. **And only over the
//! login it replaces** — the device it lands on writes it where its own login is
//! still the one this device last knew it to hold, and keeps a login made or
//! refreshed there since: a session that signed out on a token rotated away under
//! it would otherwise put the dead token over the live one. Nothing here merges or
//! locks anything. See [`Lending::written_home`], [`known`], and
//! [`crate::peer::account::LOGIN`], which is where it lands.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use verkstead_render::{AccountFile, AccountLogin, AccountLoginTaken, AccountSignedOut, LoginOver};

use crate::device::Devices;
use crate::peer::account::{ACCOUNT, LOGIN, SIGNED_OUT};
use crate::relaying::{self, Call, Refusal, Streamed, as_json};
use crate::sandbox::{Homes, kept_in, root};
use crate::store;

/// The most an account may be before the answer is dropped rather than written
/// down: **one megabyte**.
///
/// An account here is a login, a settings file and a configuration file — some
/// kilobytes between them, an OAuth pair being a few hundred bytes. What a bound
/// is for is the case every bound across a link is for: a machine on the far end
/// that answers and then writes without stopping. The mirrors' own bound, for its
/// reason — see [`super::MOST_THE_PROFILES_ARE`].
const MOST_AN_ACCOUNT_IS: usize = 1024 * 1024;

/// What a launch from a copy is refused with where no device it is bound on
/// holds a login to fetch: the gap, naming no device — the row's own reading of
/// it, see [`verkstead_render::Broken::NoBoundDevice`].
const NO_BOUND_DEVICE: &str =
    "No device this profile is bound on has a login to copy. Log in on one of them.";

/// A member's account mirrored here for a launch to be made out of: the Profile
/// the launch reads in the record's place, and what its ending has to write
/// back to the device it was fetched from.
#[derive(Debug, Clone)]
pub struct Mirrored {
    /// The same Profile by the same uuid, naming this device's mirror of the
    /// account rather than the paths on a machine it is bound on. So the root,
    /// the Tail that follows the session's log and everything the ending writes
    /// back are what they are for an account on this machine.
    pub profile: store::Profile,

    /// And what the ending puts to the device it was fetched from — see [`Lent`] and
    /// [`Lending`].
    pub lent: Lent,
}

/// What a session that runs from a copy carries to its ending: which machine the
/// account was fetched from, the uuid the Profile has on every device, where the
/// mirror keeps its login, and how to tell whether the session changed it.
///
/// **Held for as long as the session runs**, by whoever holds the [`Closing`] that
/// sees to the rest of the ending: the two are the same moment, and the login is
/// read only once that one has written what the session left into the mirror.
///
/// [`Closing`]: crate::sandbox::Closing
#[derive(Debug, Clone)]
pub struct Lent {
    /// The device the account was fetched from: one the Profile is bound on.
    at: String,

    /// And the Profile's uuid, which is the one it has there too — what the
    /// write-back is addressed by, the way every press over a mirror is.
    profile: String,

    /// Where the mirror keeps its login, on this device.
    login: PathBuf,

    /// And what came down in it, or nothing where the account it was fetched from
    /// had no login file at all — which is the case a session that logs in for the
    /// first time starts in.
    ///
    /// **A fingerprint rather than the login.** What the ending asks is one
    /// question — *is this still what came down* — and the bytes it answers about
    /// are read off the mirror at the time. So a Profile lent out does not cost a
    /// login held in this process's memory for the hours a session runs.
    came_down: Option<Fingerprint>,
}

impl Lent {
    /// The **Device Id** the account was fetched from: one the Profile is bound
    /// on, and the one the memory store is synced with and a sign-out names.
    pub fn at(&self) -> &str {
        &self.at
    }
}

/// What a login is remembered by across a session: the SHA-256 of what came down
/// in it.
///
/// Nothing is proved against it and nothing is signed with it. What it is for is
/// the one comparison the ending makes, and a hash of the bytes is what cannot
/// miss a change the way a length or a modification time can — the same reading
/// the Code pane's version is, see [`crate::files`].
type Fingerprint = [u8; 32];

/// The login each mirror on this device last knew its account to hold, by the
/// path of the mirror's login: what came down at the last fetch, or what the last
/// ending wrote into the account since. `None` beside a path is an account that
/// had no login.
///
/// **What a login sent back goes over** — see [`AccountLogin::over`]. Per mirror
/// rather than per session, because two sessions under the one member's Profile
/// share the one mirror and the one login in it: the first to end writes what they
/// both refreshed into the account, and the second's own fetch is then a login the
/// account no longer holds. Measured against that, the second's write would be
/// kept out of an account that holds nothing newer than what this device just put
/// there — so the second asks after what this device last knew instead.
///
/// **Keyed by the path**, which is this device's own: a Data Directory has one
/// mirror per Profile, and two Verksteads in one process — every suite that links
/// two — have a Data Directory each. **Held in the process**, so a restart forgets
/// it and an ending after one goes over what its own session fetched, which is the
/// reading this improves on rather than a worse one.
static KNOWN: LazyLock<Mutex<HashMap<PathBuf, Option<Fingerprint>>>> =
    LazyLock::new(Mutex::default);

/// What the mirror whose login is at `login` last knew its account to hold, or
/// `None` where this process has neither fetched it nor written it back.
fn known(login: &Path) -> Option<Option<Fingerprint>> {
    KNOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(login)
        .copied()
}

/// And that it now holds `held`: fetched, or written into it.
fn knows(login: &Path, held: Option<Fingerprint>) {
    KNOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(login.to_owned(), held);
}

/// The Profile a session on this device is launched under, where the Profile is
/// **not bound here**: its account fetched from a device it is bound on into
/// this device's own mirror of it, and the Profile as the launch below reads it
/// — the same record, naming that directory as its account.
///
/// `Ok(None)` is a Profile bound here, which is every Profile on a Verkstead
/// that is linked to nothing: there is nothing to fetch and nothing to rewrite,
/// and the launch goes on with the Profile it already had.
///
/// `Err` is *the account was never fetched* — no device it is bound on answered,
/// or this device could not write the mirror down — and it names the machine. A
/// session is not started on it: what would come up is a session logged out, with
/// nothing saying why, which is the whole failure this is in front of.
pub async fn fetched(
    devices: Option<&Devices>,
    homes: &Homes,
    profile: &store::Profile,
) -> Result<Option<Mirrored>, Refusal> {
    let Some(unbound) = &profile.unbound else {
        return Ok(None);
    };

    // The first device bound on it, whose binding holds a login, that
    // answers: one not answering is passed over for the next, and where none
    // answered the first one's refusal is the one said, it naming a machine.
    let mut refused = None;
    let mut fetched = None;

    for at in &lending(devices, unbound).await {
        match relaying::word_from::<Vec<AccountFile>>(
            devices,
            at,
            relaying::asking(ACCOUNT.replace("{profile}", &profile.id)),
            MOST_AN_ACCOUNT_IS,
        )
        .await
        {
            Ok(said) => {
                fetched = Some((at.clone(), said));
                break;
            }

            Err(why) => {
                refused.get_or_insert(why);
            }
        }
    }

    let Some((at, said)) = fetched else {
        return Err(refused.unwrap_or_else(|| Refusal::ours(NO_BOUND_DEVICE.to_owned())));
    };

    let under = homes.account_mirror(&profile.id);
    let agent_type = profile.agent_type();

    // What came down in the login, taken before the answer is written down and off
    // the answer rather than off the file: this is the one thing the ending has to
    // hold, and a session running beside this one may write the mirror's own file
    // between here and there.
    let (Some(inside), Some(account)) =
        (root::login_inside(agent_type), kept_in(agent_type, &under))
    else {
        return Err(Refusal::ours(format!(
            "There's no harness {agent_type} on this device to run it on."
        )));
    };

    let came_down = said
        .iter()
        .find(|file| named(&file.inside).as_deref() == Some(inside.as_path()))
        .and_then(|file| file.text.as_deref())
        .map(|text| fingerprint(text.as_bytes()));

    let written = tokio::task::spawn_blocking({
        let under = under.clone();

        move || written(&under, agent_type, &said)
    })
    .await;

    match written {
        Ok(Ok(())) => {}

        Ok(Err(why)) => {
            return Err(Refusal::ours(format!(
                "The account couldn't be written into {}: {why:#}.",
                under.display(),
            )));
        }

        Err(why) => {
            return Err(Refusal::ours(format!(
                "The account couldn't be written down: {why:#}.",
            )));
        }
    }

    let login = under.join(inside);

    knows(&login, came_down);

    Ok(Some(Mirrored {
        profile: store::Profile {
            account,
            ..profile.clone()
        },
        lent: Lent {
            at,
            profile: profile.id.clone(),
            login,
            came_down,
        },
    }))
}

/// The devices a launch here may fetch the account from: those of
/// [`store::Unbound::lending`] that are still members.
///
/// **A binding outlives the link to the device that wrote it.** Only the device
/// a binding names ever writes it, so one written by a device since unlinked
/// stays in every database of the cluster for good — and dialling it is a
/// refusal naming a machine nobody can reach any more, said in front of the
/// gap that is actually there. So a device that has left is passed over before
/// anything is asked of it, and a Profile bound on nothing but departed devices
/// reads as bound on none.
///
/// A membership that cannot be read leaves the list as it is: the dial below
/// refuses a departed device by name all the same, and a launch refused for a
/// read that failed would be one refused for nothing about the Profile.
async fn lending(devices: Option<&Devices>, unbound: &store::Unbound) -> Vec<String> {
    let Some(devices) = devices else {
        return unbound.lending.clone();
    };

    match devices.membership().rows().await {
        Ok(members) => unbound
            .lending
            .iter()
            .filter(|device| members.iter().any(|member| &member.device == *device))
            .cloned()
            .collect(),

        Err(why) => {
            tracing::warn!(
                error = ?why,
                "the membership could not be read, so every device a Profile is bound on is \
                 tried for its account",
            );

            unbound.lending.clone()
        }
    }
}

/// Move each account mirror kept under a Profile's integer id to the uuid the
/// switch gave that Profile — see [`store::renumbered_profiles`].
///
/// **The mirror is a member's login on this disk**, and what takes one away is
/// the row that names it going — see [`super::swept`], which finds it by the
/// uuid. So a directory left under the integer would be a login nothing will
/// ever take away again: it is moved, at the first start, before any route is
/// answered.
///
/// A directory that is not there is a Profile nothing was ever launched under
/// here, which is let go of the same as one moved. One that will not move is a
/// line in the log, and asked about again at the next start.
pub(crate) async fn renumbered(pool: &SqlitePool, data_dir: &Path) -> anyhow::Result<()> {
    // A router with no Data Directory has nowhere to have kept a mirror, and
    // the empty path would resolve to the working directory — see
    // [`crate::nowhere`].
    if data_dir.as_os_str().is_empty() {
        return Ok(());
    }

    let accounts = crate::sandbox::accounts(data_dir);

    for (was, uuid) in store::renumbered_profiles(pool).await? {
        let from = accounts.join(was.to_string());
        let to = accounts.join(&uuid);

        match tokio::fs::rename(&from, &to).await {
            Ok(()) => tracing::info!(
                from = %from.display(),
                to = %to.display(),
                "the mirror of a member's account moved to the uuid its Profile is named by now",
            ),

            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}

            Err(why) => {
                tracing::warn!(
                    error = ?why,
                    from = %from.display(),
                    to = %to.display(),
                    "the mirror of a member's account could not be moved to the uuid its Profile \
                     is named by now",
                );

                continue;
            }
        }

        store::renumbering_done(pool, was).await?;
    }

    Ok(())
}

/// Move the account mirror kept under a bridged mirror's provisional uuid to
/// the uuid the device it was minted on publishes — see [`store::settle`].
///
/// For [`renumbered`]'s reason: the row that names a mirror is what takes it
/// away, and a directory left under the provisional uuid is a member's login
/// nothing names any more. And the next session under the Profile fetches into
/// the one named by the real uuid, so the move is what keeps that one fetch
/// rather than a fresh mirror beside a forgotten one.
///
/// Where one is already under the real uuid, the provisional one is taken away
/// instead: a mirror is fetched fresh before every launch, so the one already
/// in place loses nothing. Anything that will not move is a line in the log.
pub(crate) async fn bridged(data_dir: &Path, from: &str, to: &str) {
    // A router with no Data Directory has nowhere to have kept a mirror — see
    // [`renumbered`].
    if data_dir.as_os_str().is_empty() {
        return;
    }

    let accounts = crate::sandbox::accounts(data_dir);
    let from = accounts.join(from);
    let to = accounts.join(to);

    let moved = if tokio::fs::try_exists(&to).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&from).await
    } else {
        tokio::fs::rename(&from, &to).await
    };

    match moved {
        Ok(()) => tracing::info!(
            from = %from.display(),
            to = %to.display(),
            "the mirror of a member's account moved to the uuid the Profile was minted with",
        ),

        // Never fetched, which is a Profile nothing was launched under here.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}

        Err(why) => tracing::warn!(
            error = ?why,
            from = %from.display(),
            to = %to.display(),
            "the mirror of a member's account could not be moved to the uuid the Profile was \
             minted with",
        ),
    }
}

/// Take away the copy of each removed Profile's account kept under the Data
/// Directory.
///
/// **The record is the last thing on this device that says whose login that
/// was**, and a removed one is in no list: a directory left behind would be a
/// member's login on this disk with nothing pointing at it. Asked of every
/// removed record at every pull, so a directory that would not go is tried
/// again, and a line in the log meanwhile rather than anything louder.
pub(crate) async fn swept(data_dir: &Path, removed: &[String]) {
    // A router with no Data Directory has nowhere to have kept a copy.
    if data_dir.as_os_str().is_empty() {
        return;
    }

    let accounts = crate::sandbox::accounts(data_dir);

    for uuid in removed {
        let under = accounts.join(uuid);

        match tokio::fs::remove_dir_all(&under).await {
            Ok(()) => tracing::info!(
                copy = %under.display(),
                "the copy of a removed Profile's account is gone with its record",
            ),

            // Never fetched, which is a Profile nothing was launched under here.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}

            Err(why) => tracing::warn!(
                error = ?why,
                copy = %under.display(),
                "the copy of a removed Profile's account could not be taken away, so a login \
                 of it is still on this device",
            ),
        }
    }
}

/// What a session or a terminal that runs from a copy holds until its ending, in
/// one value: the login to write back, the store the sentence goes in where the
/// device it was fetched from has gone away, and the cluster handle it travels
/// over.
///
/// **One value rather than three arguments beside the launch's own**, because both
/// callers hold it across a spawn — a session's relay and a terminal's follow loop
/// — and what either of them wants of it is one thing: *see to this now the process
/// has gone*.
#[derive(Debug, Clone)]
pub struct Lending {
    pool: SqlitePool,
    devices: Option<Devices>,
    lent: Lent,
}

impl Lending {
    /// What a launch that fetched a mirror leaves for its ending to see to.
    pub fn of(pool: &SqlitePool, devices: Option<&Devices>, lent: Lent) -> Lending {
        Lending {
            pool: pool.clone(),
            devices: devices.cloned(),
            lent,
        }
    }

    /// The **Device Id** the account was fetched from — see [`Lent::at`].
    pub fn at(&self) -> &str {
        self.lent.at()
    }

    /// The login a session that ran from a copy left, written back into the
    /// account it belongs to on the device it was fetched from.
    ///
    /// **Called once the ending has seen to the profile** — see
    /// [`crate::sandbox::Closing`], which is what puts a login the session replaced
    /// rather than wrote through back over the mirror's own file. So what is read here
    /// is the login as the session finished with it, whichever way the harness saved
    /// it and whichever platform's rendering gave it to them.
    ///
    /// **Only what changed travels**, which is the three cases a local ending already
    /// tells apart, asked the one way a mirror can be asked: a login that is no longer
    /// what the account holds goes back whole, one made where the account had none is
    /// handed over, and one still exactly what the account holds is not sent at all.
    /// *What the account holds* is what this device last knew of it — see [`known`] —
    /// rather than what this session's own fetch brought, which another session on
    /// the same mirror may have written over since.
    ///
    /// **And only over the login it replaces.** What is sent names that login, and
    /// the device it lands on keeps a login of its own made or refreshed since — see
    /// [`crate::peer::account::written`]. A refresh token is spent as it is used, so
    /// whichever machine refreshed first holds the one live login: a session here
    /// whose token was rotated away by a refresh at home has nothing worth sending,
    /// and putting it over the account anyway is how one session signing out became
    /// every session after it signing out too (ADR-0020, *Shared Profiles*). Nothing
    /// is merged and nothing is locked.
    ///
    /// **A device that has gone away by then is said rather than swallowed.** The
    /// mirror is left exactly where it is and nothing retries: what is put on the
    /// Conversation's Timeline is a sentence saying the account was not written, and
    /// the next session on that device may find itself signed out. Which is worth a
    /// line — the human is the one who can put it right, with a login on that device —
    /// where a silent loss would be an account signing itself out with nothing
    /// anywhere saying so.
    pub async fn written_home(self, conversation_id: i64) {
        let Lending {
            pool,
            devices,
            lent:
                Lent {
                    at,
                    profile,
                    login,
                    came_down,
                },
        } = self;

        let devices = devices.as_ref();

        let over = known(&login).unwrap_or(came_down);

        let left = tokio::task::spawn_blocking({
            let login = login.clone();

            move || left_behind(&login, over)
        })
        .await;

        let text = match left {
            Ok(Some(text)) => text,

            // The login is still what the account holds, or there is none there at
            // all: a session that wrote nothing to it, and one that took away the
            // file it was given, which is not a session asking for the account's to
            // go.
            Ok(None) => return,

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    conversation_id,
                    "reading what a session run from a copy left in its login ended badly, so the \
                     account it belongs to was not written",
                );

                return;
            }
        };

        let sent = fingerprint(text.as_bytes());

        let body = match serde_json::to_vec(&AccountLogin {
            text,
            over: Some(match over {
                Some(over) => LoginOver::Login(hex(&over)),
                None => LoginOver::Nothing,
            }),
        }) {
            Ok(body) => body,

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    conversation_id,
                    "the login a session run from a copy left could not be composed for the \
                     device its account was fetched from",
                );

                return;
            }
        };

        let call = Call {
            method: reqwest::Method::POST,
            onwards: LOGIN.replace("{profile}", &profile),
            headers: as_json(),
            body: Streamed::saying(body),
        };

        match relaying::put_to(devices, &at, call).await {
            // An empty answer is a Verkstead from before the answer said, which wrote
            // every login it was sent — see [`AccountLoginTaken`].
            Ok(answer) => match answer.json::<AccountLoginTaken>().await {
                Ok(AccountLoginTaken { written: false }) => tracing::info!(
                    conversation_id,
                    device = %at,
                    "the login this session left was not written back, the account on the \
                     device it was fetched from holding a login made or refreshed there since",
                ),

                _ => {
                    knows(&login, Some(sent));

                    tracing::debug!(
                        conversation_id,
                        device = %at,
                        "the login this session left was written into the account on the \
                         device it was fetched from",
                    );
                }
            },

            Err(why) => {
                tracing::warn!(
                    conversation_id,
                    device = %at,
                    "the login this session left could not be written back: {}",
                    why.saying,
                );

                let line = format!(
                    "Verkstead didn't write this profile's login back to the device its \
                     account is on. {} Log in on that device, or a session there may find the \
                     account signed out.",
                    why.saying,
                );

                if let Err(error) = store::note(&pool, conversation_id, &line).await {
                    tracing::error!(
                        error = ?error,
                        conversation_id,
                        "saying on the Timeline that an account was not written failed",
                    );
                }
            }
        }
    }
}

impl Lending {
    /// Tell the device the account was fetched from that a session here found
    /// it **signed out**, naming the login that came down.
    ///
    /// **That device is the only one that can offer the login, and the only one
    /// that cannot see this.** Its login file is still there and its harness may
    /// call it fine, so without word from here its card goes on saying the
    /// account is logged in, with nothing to press — while every launch here
    /// comes up signed out. See [`crate::peer::account::signed_out`], which is
    /// the end that marks it, and only where its login is still the one that
    /// came down.
    ///
    /// Said in the log where it does not get there, and nothing else: the stop
    /// here has already said what happened and where to log in.
    pub async fn signed_out(&self) {
        let body = match serde_json::to_vec(&AccountSignedOut {
            login: self.lent.came_down.map(|came_down| hex(&came_down)),
        }) {
            Ok(body) => body,

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    "the word that an account signed out could not be composed for the \
                     device it was fetched from",
                );

                return;
            }
        };

        let call = Call {
            method: reqwest::Method::POST,
            onwards: SIGNED_OUT.replace("{profile}", &self.lent.profile),
            headers: as_json(),
            body: Streamed::saying(body),
        };

        match relaying::put_to(self.devices.as_ref(), &self.lent.at, call).await {
            Ok(_) => tracing::info!(
                device = %self.lent.at,
                profile = %self.lent.profile,
                "the device this account was fetched from has been told it signed out",
            ),

            Err(why) => tracing::warn!(
                device = %self.lent.at,
                profile = %self.lent.profile,
                "the device this account was fetched from could not be told it signed out: {}",
                why.saying,
            ),
        }
    }
}

/// A login's SHA-256 as the word that names it across a link: lower-case hex.
///
/// What [`AccountSignedOut::login`] carries, and what the device the account is
/// on hashes its own login into to compare — see [`login_named`].
pub(crate) fn hex(fingerprint: &Fingerprint) -> String {
    fingerprint
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The login at `login` named as [`hex`] names one, or `None` where there is no
/// login file there.
pub(crate) fn login_named(login: &Path) -> Option<String> {
    std::fs::read(login)
        .ok()
        .map(|bytes| hex(&fingerprint(&bytes)))
}

/// What the session left in the mirror's login, where that is something the
/// account it was fetched from has not got — and `None` where nothing is to
/// travel.
///
/// Three readings in one comparison against `held`, which is what the account
/// holds as far as this device knows — see [`known`]. A login whose bytes are no
/// longer that is one the session wrote or replaced, and it goes back whole. A
/// login where `held` is nothing is one made where the account had none, and it is
/// handed over. And one that still hashes to it is a session that did not touch
/// it, which is not a write for the account it was fetched from to take.
///
/// **A name with nothing at it is nothing to send**, which is the rule a local
/// ending holds to: a session that took away the file it was given is not a
/// session asking for the account's to go. And so is a login that is not text —
/// what a file no harness wrote holds is not a login, the reading
/// [`AccountFile::text`] is answered with in the other direction.
///
/// **The mirror is what is read, rather than what this session was handed.** Two
/// Conversations may be running under the one member's Profile and there is one
/// mirror between them, written beside and renamed over at each fetch — so a
/// session may read a login the other one's launch brought down, or send back
/// what the other one left. Which is why `held` is the mirror's rather than the
/// session's: neither is held up for the other.
///
/// Blocking: one read.
fn left_behind(login: &Path, held: Option<Fingerprint>) -> Option<String> {
    let now = std::fs::read_to_string(login).ok()?;

    (held != Some(fingerprint(now.as_bytes()))).then_some(now)
}

/// Whether `sent` goes into the account whose login is at `login` — the half on
/// the device it is bound on, called by [`crate::peer::account::written`].
///
/// **Where the account still holds the login it names**, which is the login the
/// sending device last knew it to hold — see [`AccountLogin::over`]. One made or
/// refreshed here since is newer than anything the session had: a refresh token
/// is spent as it is used, so of two machines refreshing one login only the first
/// holds a live one, and a session that signed out on a token rotated away under
/// it would otherwise put that token over the live one here.
///
/// **And where it would change nothing**, the account already holding exactly what
/// was sent. And always for a Verkstead from before the login was named, which is
/// written as it always was.
///
/// Read and written as two steps rather than one: a refresh here landing between
/// them is the harness's own write and the last one, the race a local ending has
/// with a harness at the same file.
///
/// Blocking: one read.
pub(crate) fn goes_over(login: &Path, sent: &AccountLogin) -> bool {
    let Some(over) = &sent.over else {
        return true;
    };

    let here = login_named(login);

    let named = match over {
        LoginOver::Login(over) => here.as_deref() == Some(over.as_str()),
        LoginOver::Nothing => here.is_none(),
    };

    named || here == Some(hex(&fingerprint(sent.text.as_bytes())))
}

/// And the login that arrived, written into the account at `login` — which is the
/// half on the device it is bound on, called by
/// [`crate::peer::account::written`].
///
/// **Here rather than there** because it is the same file this module writes into a
/// mirror, written by the same rules: in place, and readable by its owner alone.
///
/// **In place over whatever is at the path**, which is how a local session's ending
/// writes a login back: whatever else is a name for that file — a session running
/// on this device right now has it hard-linked or bound into its root — is a name
/// for what arrived, rather than for a file a rename left behind. Which is also
/// the exposure a local ending has: a write that fails part way through has
/// truncated the file, and what stands against that is the harness logging in
/// again.
///
/// The directory is made where the account has none, which is an account whose
/// harness has never been logged in to on that machine. One that is already there
/// is the human's own and is left exactly as it is.
///
/// Blocking.
pub(crate) fn into_the_account(login: &Path, text: &str) -> std::io::Result<()> {
    if let Some(over) = login.parent()
        && !over.is_dir()
    {
        std::fs::create_dir_all(over)?;
        owner_only(over)?;
    }

    std::fs::write(login, text)?;

    // Every time rather than only where the file is new: what is at that path is a
    // login, and a login the machine can read is one the machine could use.
    owner_only(login)
}

/// The SHA-256 of `bytes`, as a login is remembered by across a session — see
/// [`Fingerprint`].
fn fingerprint(bytes: &[u8]) -> Fingerprint {
    Sha256::digest(bytes).into()
}

/// What the device it is bound on said, written into the mirror at `under` — and
/// every file of the allowlist it did not say taken away.
///
/// **The allowlist is this device's own**, rather than whatever arrived: a
/// mirror holds the files a Built Root is made of and nothing else, so a path
/// that is not one of them is dropped with a line in the log. Which is also what
/// keeps a path out of it that was never a path at all — nothing here joins a
/// name a far end chose onto a directory of this device's without saying which
/// names it will take.
///
/// **Written beside and renamed over**, which is how a harness saves a login and
/// what lets a session already running keep the file it was given: a rename
/// replaces the name and leaves the inode a bind is holding alone. Owner-only,
/// because one of these is a login.
///
/// Blocking.
fn written(
    under: &Path,
    agent_type: store::AgentType,
    said: &[AccountFile],
) -> std::io::Result<()> {
    for inside in root::mirrored_of(agent_type) {
        let path = under.join(&inside);

        // The one the far end sent for this path, where it sent one at all: a
        // path spelled otherwise is not this file, and a Verkstead that answers
        // paths this one does not know about is one whose extra files are no
        // part of a root built here.
        let sent = said
            .iter()
            .find(|file| named(&file.inside).as_deref() == Some(inside.as_path()));

        match sent.and_then(|file| file.text.as_deref()) {
            Some(text) => {
                if let Some(over) = path.parent() {
                    std::fs::create_dir_all(over)?;
                    owner_only(over)?;
                }

                beside(&path, text.as_bytes())?;
            }

            // Nothing there on the device it is bound on, so nothing here: an
            // account with no login file, or one signed out since the last session
            // run from a copy. Taking it away
            // is the point — a mirror that kept the last login it saw would be a
            // session running as somebody who has signed out.
            None => match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
                Err(why) => return Err(why),
            },
        }
    }

    Ok(())
}

/// `inside` as a path this device will write under a mirror, or nothing where it
/// is not one.
///
/// A name at all, relative, and every component a plain one: a path that is
/// empty, that is absolute, that starts at a root or a prefix, or that steps up
/// through `..` is one that would name nothing or land outside the mirror, and
/// the answer to each is that it is not a file of an account. Spelled with
/// forward slashes by the device that answered, whichever platform it is on — see
/// [`crate::peer::account::held`].
pub(crate) fn named(inside: &str) -> Option<PathBuf> {
    let path = PathBuf::from(inside.replace('\\', "/"));
    let mut parts = path.components();

    (parts.next().is_some_and(plain) && parts.all(plain)).then_some(path)
}

/// Whether one component of such a path is a plain name.
fn plain(part: Component<'_>) -> bool {
    matches!(part, Component::Normal(_))
}

/// One file written beside its own name and renamed over it, so that what is at
/// the path is either the whole of the new file or the whole of the old one.
///
/// Owner-only from the moment it exists, which is what it has to be: the file
/// written here may be a login, and one that spent a moment readable by the
/// machine would be one the moment was enough for.
///
/// The name it is written beside under is [`super::aside`]'s, which is where the
/// reason two of these must never share one is.
fn beside(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = super::aside(path);

    let written = std::fs::write(&temporary, bytes)
        .and_then(|()| owner_only(&temporary))
        .and_then(|()| std::fs::rename(&temporary, path));

    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }

    written
}

/// And the mode a mirror's files and directories are kept at: readable by the
/// account the server runs as and by nobody else.
///
/// Nothing at all off Unix, where a mode is not what says who may read a file:
/// what stands there is the Data Directory's own access control, which is where
/// every other file Verkstead writes for a session is protected.
///
/// Shared with [`into_the_account`], which writes a login the other way: it is the
/// same file, and the mode it is kept at is one rule rather than two.
fn owner_only(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = match path.is_dir() {
            true => 0o700,
            false => 0o600,
        };

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }

    let _ = path;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's account mirrored under a Profile's integer moves to the uuid
    /// the switch gave it, and the record of the move is let go of — so a
    /// second start finds nothing to move.
    #[tokio::test]
    async fn an_account_mirror_moves_from_the_integer_to_the_uuid() {
        let held = tempfile::tempdir().unwrap();
        let pool = store::open_database(&held.path().join("verkstead.db"))
            .await
            .unwrap();
        let uuid = "5e1f0a2b-3c4d-4e5f-8a6b-7c8d9e0f1a2b";

        sqlx::query("INSERT INTO profiles_renumbered (was, uuid) VALUES (2, ?), (3, ?)")
            .bind(uuid)
            .bind("9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d")
            .execute(&pool)
            .await
            .unwrap();

        let accounts = crate::sandbox::accounts(held.path());
        std::fs::create_dir_all(accounts.join("2/.claude")).unwrap();
        std::fs::write(accounts.join("2/.claude/.credentials.json"), "{}").unwrap();

        renumbered(&pool, held.path()).await.unwrap();

        assert!(
            !accounts.join("2").exists(),
            "nothing is left under the integer"
        );
        assert!(
            accounts
                .join(uuid)
                .join(".claude/.credentials.json")
                .is_file(),
            "and the login is under the uuid",
        );
        assert_eq!(
            store::renumbered_profiles(&pool).await.unwrap(),
            Vec::new(),
            "a Profile nothing was mirrored for is let go of too",
        );
    }

    /// A path of an account's own is taken, and one that would land anywhere but
    /// under the mirror is not a file of an account.
    #[test]
    fn only_a_plain_relative_path_is_written() {
        assert_eq!(
            named(".claude/.credentials.json").as_deref(),
            Some(Path::new(".claude/.credentials.json")),
        );

        for elsewhere in [
            "/etc/passwd",
            "../../.ssh/authorized_keys",
            ".claude/../../.ssh/id_ed25519",
            "",
        ] {
            assert_eq!(
                named(elsewhere),
                None,
                "{elsewhere} is not a file of an account",
            );
        }
    }

    /// And a Windows account's paths arrive spelled that machine's way and are
    /// read as the same files: the route answers with forward slashes, and a
    /// device that spelled them otherwise is still understood.
    #[test]
    fn a_backslash_is_the_same_path() {
        assert_eq!(
            named(r".claude\settings.json").as_deref(),
            Some(Path::new(".claude/settings.json")),
        );
    }

    /// The mirror's login as the ending finds it, and what came down in it.
    fn mirror(now: Option<&str>, came_down: Option<&str>) -> (tempfile::TempDir, Option<String>) {
        let dir = tempfile::tempdir().unwrap();
        let login = dir.path().join(".credentials.json");

        if let Some(now) = now {
            std::fs::write(&login, now).unwrap();
        }

        let left = left_behind(&login, came_down.map(|text| fingerprint(text.as_bytes())));

        (dir, left)
    }

    /// A login the session replaced goes back whole — which is a harness that
    /// refreshed its OAuth pair, the one thing a session does to an account.
    #[test]
    fn a_login_the_session_replaced_goes_back_whole() {
        let (_dir, left) = mirror(Some("refreshed\n"), Some("what came down\n"));

        assert_eq!(left.as_deref(), Some("refreshed\n"));
    }

    /// One the session left exactly as it was given is not written at all: only
    /// what changed travels, and a write of an unchanged login is a write that
    /// could only lose a refresh the device it is bound on made meanwhile.
    #[test]
    fn a_login_the_session_left_alone_is_not_written() {
        let (_dir, left) = mirror(Some("what came down\n"), Some("what came down\n"));

        assert_eq!(left, None);
    }

    /// And one made where the account had none is handed over, which is the case a
    /// Profile nobody has logged in to starts in.
    #[test]
    fn a_login_made_where_the_account_had_none_is_handed_over() {
        let (_dir, left) = mirror(Some("logged in\n"), None);

        assert_eq!(left.as_deref(), Some("logged in\n"));
    }

    /// A mirror with no login in it at the end sends nothing, whether or not one
    /// came down: a session that took away the file it was given is not a session
    /// asking for the account's to go.
    #[test]
    fn a_login_the_session_took_away_sends_nothing() {
        for came_down in [Some("what came down\n"), None] {
            let (_dir, left) = mirror(None, came_down);

            assert_eq!(left, None, "came down: {came_down:?}");
        }
    }

    /// The login that arrives back is written in place over the account's own file,
    /// so that whatever else is a name for it — a session running there has it
    /// linked into its root — is a name for what arrived.
    #[cfg(unix)]
    #[test]
    fn the_login_that_arrives_is_written_over_the_accounts_own_file() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let account = tempfile::tempdir().unwrap();
        let login = account.path().join(".claude/.credentials.json");

        into_the_account(&login, "the first one\n").unwrap();

        let made = std::fs::metadata(&login).unwrap();

        assert_eq!(
            made.permissions().mode() & 0o777,
            0o600,
            "a login is readable by its owner and nobody else",
        );

        // A second name for that file, which is what a session running on the
        // device it is bound on has while it runs.
        let linked = account.path().join("linked");
        std::fs::hard_link(&login, &linked).unwrap();

        into_the_account(&login, "the later one\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(&login).unwrap(),
            "the later one\n",
            "what arrived is what is there: nothing is merged and nothing is held",
        );
        assert_eq!(
            std::fs::metadata(&login).unwrap().ino(),
            made.ino(),
            "and it is the same file, so every other name for it is a name for \
             what arrived",
        );
        assert_eq!(std::fs::read_to_string(&linked).unwrap(), "the later one\n",);
    }

    /// The account at `here` — `None` for none — and whether a login sent back
    /// over `over` goes into it.
    fn goes(here: Option<&str>, sent: &str, over: Option<LoginOver>) -> bool {
        let dir = tempfile::tempdir().unwrap();
        let login = dir.path().join(".credentials.json");

        if let Some(here) = here {
            std::fs::write(&login, here).unwrap();
        }

        goes_over(
            &login,
            &AccountLogin {
                text: sent.to_owned(),
                over,
            },
        )
    }

    /// Named by what it hashes to, as the sending device names it.
    fn over(text: &str) -> Option<LoginOver> {
        Some(LoginOver::Login(hex(&fingerprint(text.as_bytes()))))
    }

    /// A login sent back goes over the one it names, which is an account nobody
    /// here has touched since the sending device last knew it.
    #[test]
    fn a_login_goes_over_the_one_it_names() {
        assert!(goes(Some("lent\n"), "refreshed away\n", over("lent\n")));
        assert!(goes(None, "logged in away\n", Some(LoginOver::Nothing)));
    }

    /// And not over one made or refreshed here since: of two machines refreshing
    /// one login only the first holds a live one, and the session's was rotated
    /// away under it.
    #[test]
    fn a_login_made_here_since_is_kept() {
        assert!(!goes(
            Some("refreshed here\n"),
            "rotated away\n",
            over("lent\n")
        ));
        assert!(!goes(
            Some("logged in here\n"),
            "logged in away\n",
            Some(LoginOver::Nothing)
        ));
        assert!(!goes(None, "rotated away\n", over("lent\n")));
    }

    /// A login the account already holds goes in, there being nothing to lose; and
    /// one from a Verkstead that named nothing is written as it always was.
    #[test]
    fn the_same_login_and_an_unnamed_one_go_in() {
        assert!(goes(Some("same\n"), "same\n", over("lent\n")));
        assert!(goes(Some("refreshed here\n"), "from before\n", None));
    }

    /// What the mirror last knew is per mirror, and the latest word wins.
    #[test]
    fn a_mirror_knows_what_it_was_last_told() {
        let dir = tempfile::tempdir().unwrap();
        let login = dir.path().join(".credentials.json");

        assert_eq!(known(&login), None, "nothing fetched, nothing known");

        knows(&login, None);
        assert_eq!(known(&login), Some(None));

        let written = fingerprint(b"written home\n");

        knows(&login, Some(written));
        assert_eq!(known(&login), Some(Some(written)));
    }
}
