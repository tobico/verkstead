//! The **account mirror**: the login and the configuration of a member's
//! account, fetched from the device it is at home on into a directory of this
//! device's own under the Data Directory, so that a session launched here under
//! that Profile is given a Built Root made out of it (ADR-0020, *Shared
//! Profiles*).
//!
//! **The row travels and the account does not.** A mirror row carries what a
//! Profile is drawn and picked by — see [`super`] — and none of it is an account
//! a session could run as. What a session away from home gets is a copy of the
//! little of an account a root is made from: the login, and what the written
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
//! at home since the last session is the one this session has to be given, and a
//! mirror left to go stale would sign a session out for no reason anybody could
//! see. What the home device says is written over what is there, and every file
//! of the allowlist it did *not* send is taken away — a login that has gone at
//! home is a login gone here.
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
//! once the ending has written everything back into it, and put to the home device
//! where it is not still what came down: replaced or made where there was none
//! goes back whole, untouched goes nowhere. **Last write wins** — two machines
//! refreshing one login may sign one of them out, and nothing here merges or locks
//! anything. See [`Lending::written_home`], and [`crate::peer::account::LOGIN`],
//! which is where it lands.

use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use verkstead_render::{AccountFile, AccountLogin};

use crate::device::Devices;
use crate::peer::account::{ACCOUNT, LOGIN};
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

/// A member's account mirrored here for a launch to be made out of: the Profile
/// the launch reads in the mirror row's place, and what its ending has to write
/// back home.
#[derive(Debug, Clone)]
pub struct Mirrored {
    /// The same row by the same local id, naming this device's mirror of the
    /// account rather than the paths on the machine it is at home on. So the root,
    /// the Tail that follows the session's log and everything the ending writes
    /// back are what they are for an account on this machine.
    pub profile: store::Profile,

    /// And what the ending puts to the home device — see [`Lent`] and
    /// [`Lending`].
    pub lent: Lent,
}

/// What a session away from home carries to its ending: which machine the account
/// is at home on, the id the Profile has there, where the mirror keeps its login,
/// and how to tell whether the session changed it.
///
/// **Held for as long as the session runs**, by whoever holds the [`Closing`] that
/// sees to the rest of the ending: the two are the same moment, and the login is
/// read only once that one has written what the session left into the mirror.
///
/// [`Closing`]: crate::sandbox::Closing
#[derive(Debug, Clone)]
pub struct Lent {
    /// The device the account is at home on, and the Profile's id there — which is
    /// the id the write-back is addressed by, the way every press over a mirror is.
    at: store::Mirror,

    /// Where the mirror keeps its login, on this device.
    login: PathBuf,

    /// And what came down in it, or nothing where the account at home had no login
    /// file at all — which is the case a session that logs in for the first time
    /// starts in.
    ///
    /// **A fingerprint rather than the login.** What the ending asks is one
    /// question — *is this still what came down* — and the bytes it answers about
    /// are read off the mirror at the time. So a Profile lent out does not cost a
    /// login held in this process's memory for the hours a session runs.
    came_down: Option<Fingerprint>,
}

/// What a login is remembered by across a session: the SHA-256 of what came down
/// in it.
///
/// Nothing is proved against it and nothing is signed with it. What it is for is
/// the one comparison the ending makes, and a hash of the bytes is what cannot
/// miss a change the way a length or a modification time can — the same reading
/// the Code pane's version is, see [`crate::files`].
type Fingerprint = [u8; 32];

/// The Profile a session on this device is launched under, where the row is a
/// **mirror**: its account fetched from the device it is at home on into this
/// device's own mirror of it, and the Profile as the launch below reads it — the
/// same row, naming that directory as its account.
///
/// `Ok(None)` is one of this device's own rows, which is every Profile on a
/// Verkstead that is linked to nothing: there is nothing to fetch and nothing to
/// rewrite, and the launch goes on with the Profile it already had.
///
/// `Err` is *the account was never fetched* — the home device did not answer, or
/// this device could not write the mirror down — and it names the machine. A
/// session is not started on it: what would come up is a session logged out, with
/// nothing saying why, which is the whole failure this is in front of.
pub async fn fetched(
    devices: Option<&Devices>,
    homes: &Homes,
    profile: &store::Profile,
) -> Result<Option<Mirrored>, Refusal> {
    let Some(at) = profile.mirror.clone() else {
        return Ok(None);
    };

    let said: Vec<AccountFile> = relaying::word_from(
        devices,
        &at.device,
        relaying::asking(ACCOUNT.replace("{profile}", &at.id.to_string())),
        MOST_AN_ACCOUNT_IS,
    )
    .await?;

    let under = homes.account_mirror(profile.id);
    let agent_type = profile.agent_type();

    // What came down in the login, taken before the answer is written down and off
    // the answer rather than off the file: this is the one thing the ending has to
    // hold, and a session running beside this one may write the mirror's own file
    // between here and there.
    let inside = root::login_inside(agent_type);
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
                "the account could not be written into {}: {why:#}",
                under.display(),
            )));
        }

        Err(why) => {
            return Err(Refusal::ours(format!(
                "the account could not be written down: {why:#}",
            )));
        }
    }

    Ok(Some(Mirrored {
        profile: store::Profile {
            account: kept_in(agent_type, &under),
            ..profile.clone()
        },
        lent: Lent {
            at,
            login: under.join(inside),
            came_down,
        },
    }))
}

/// What a session or a terminal away from home holds until its ending, in one
/// value: the login to write back, the store the sentence goes in where the home
/// has gone away, and the cluster handle it travels over.
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

    /// The login a session away from home left, written back into the account it
    /// belongs to on the device that account is at home on.
    ///
    /// **Called once the ending has seen to the profile** — see
    /// [`crate::sandbox::Closing`], which is what puts a login the session replaced
    /// rather than wrote through back over the mirror's own file. So what is read here
    /// is the login as the session finished with it, whichever way the harness saved
    /// it and whichever platform's rendering gave it to them.
    ///
    /// **Only what changed travels**, which is the three cases a local ending already
    /// tells apart, asked the one way a mirror can be asked: a login that is no longer
    /// what came down goes back whole, one made where the account had none is handed
    /// over, and one still exactly what came down is not sent at all. Nothing is
    /// merged and nothing is locked — **last write wins**, and a login refreshed on two
    /// machines at once may sign one of them out (ADR-0020, *Shared Profiles*).
    ///
    /// **A home that has gone away by then is said rather than swallowed.** The mirror
    /// is left exactly where it is and nothing retries: what is put on the
    /// Conversation's Timeline is a sentence saying the account was not written, and
    /// the next session at home may find itself signed out. Which is worth a line — the
    /// human is the one who can put it right, with a login on that device — where a
    /// silent loss would be an account signing itself out with nothing anywhere saying
    /// so.
    pub async fn written_home(self, conversation_id: i64) {
        let Lending {
            pool,
            devices,
            lent:
                Lent {
                    at,
                    login,
                    came_down,
                },
        } = self;

        let devices = devices.as_ref();

        let left = tokio::task::spawn_blocking(move || left_behind(&login, came_down)).await;

        let text = match left {
            Ok(Some(text)) => text,

            // The login is still what came down, or there is none there at all: a
            // session that wrote nothing to it, and one that took away the file it was
            // given, which is not a session asking for the account's to go.
            Ok(None) => return,

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    conversation_id,
                    "reading what a session away from home left in its login ended badly, so the \
                     account it belongs to was not written",
                );

                return;
            }
        };

        let body = match serde_json::to_vec(&AccountLogin { text }) {
            Ok(body) => body,

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    conversation_id,
                    "the login a session away from home left could not be composed for the device \
                     its account is on",
                );

                return;
            }
        };

        let call = Call {
            method: reqwest::Method::POST,
            onwards: LOGIN.replace("{profile}", &at.id.to_string()),
            headers: as_json(),
            body: Streamed::saying(body),
        };

        match relaying::put_to(devices, &at.device, call).await {
            Ok(_) => tracing::debug!(
                conversation_id,
                device = %at.device,
                "the login this session left was written into the account on the device it is at \
                 home on",
            ),

            Err(why) => {
                tracing::warn!(
                    conversation_id,
                    device = %at.device,
                    "the login this session left could not be written back: {}",
                    why.saying,
                );

                let line = format!(
                    "Verkstead did not write this Profile's login back to the device its \
                     account is on — {}. A session there may find the account signed out, \
                     and the fix is a login on that device.",
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

/// What the session left in the mirror's login, where that is something the
/// account at home has not got — and `None` where nothing is to travel.
///
/// Three readings in one comparison. A login whose bytes are no longer what came
/// down is one the session wrote or replaced, and it goes back whole. A login
/// where `came_down` is nothing is one made where the account had none, and it is
/// handed over. And one that still hashes to what arrived is a session that did
/// not touch it, which is not a write for the account at home to take.
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
/// session may read a login the other one's launch brought down, or send home what
/// the other one left. Which last write wins settles between two sessions exactly
/// as it settles between two machines, and neither is held up for the other.
///
/// Blocking: one read.
fn left_behind(login: &Path, came_down: Option<Fingerprint>) -> Option<String> {
    let now = std::fs::read_to_string(login).ok()?;

    (came_down != Some(fingerprint(now.as_bytes()))).then_some(now)
}

/// And the login that arrived, written into the account at `login` — which is the
/// home device's half, called by [`crate::peer::account::written`].
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
    if let Some(over) = login.parent() {
        if !over.is_dir() {
            std::fs::create_dir_all(over)?;
            owner_only(over)?;
        }
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

/// What the home device said, written into the mirror at `under` — and every file
/// of the allowlist it did not say taken away.
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

            // Nothing there at home, so nothing here: an account with no login
            // file, or one signed out since the last session away. Taking it away
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
fn named(inside: &str) -> Option<PathBuf> {
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
fn beside(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".verkstead-{}", std::process::id()));
    let temporary = PathBuf::from(temporary);

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
    /// could only lose a refresh the home device made meanwhile.
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

    /// The login that arrives home is written in place over the account's own file,
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

        // A second name for that file, which is what a session running on the home
        // device has while it runs.
        let linked = account.path().join("linked");
        std::fs::hard_link(&login, &linked).unwrap();

        into_the_account(&login, "the later one\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(&login).unwrap(),
            "the later one\n",
            "last write wins: nothing is merged and nothing is held",
        );
        assert_eq!(
            std::fs::metadata(&login).unwrap().ino(),
            made.ino(),
            "and it is the same file, so every other name for it is a name for \
             what arrived",
        );
        assert_eq!(std::fs::read_to_string(&linked).unwrap(), "the later one\n",);
    }
}
