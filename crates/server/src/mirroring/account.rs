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

use std::path::{Component, Path, PathBuf};

use verkstead_render::AccountFile;

use crate::device::Devices;
use crate::peer::account::ACCOUNT;
use crate::relaying::{self, Refusal};
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
) -> Result<Option<store::Profile>, Refusal> {
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

    Ok(Some(store::Profile {
        account: kept_in(agent_type, &under),
        ..profile.clone()
    }))
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
}
