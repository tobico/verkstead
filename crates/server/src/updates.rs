//! Learning that a newer Verkstead has been released.
//!
//! One unauthenticated request to GitHub's latest-release API, at startup and
//! once a day after — server-side rather than from the browser, because a
//! tailnet of devices each asking GitHub would have the whole household phoning
//! home, and because the viewer must not need GitHub to be reachable to draw a
//! page.
//!
//! What it keeps is the latest stable release it found, whatever that means for
//! this device, because the **Devices** section tags every row behind it — a
//! member can be behind an up-to-date device (ADR-0027, *Seeing versions*). The
//! banner's verdict is worked out of it as it is read.
//!
//! The verdict is held in memory and nothing is written to the store: it is
//! cheap to work out again, and a server that has just started should ask rather
//! than believe what it concluded a week ago. A poll that fails leaves the last
//! verdict standing and costs nothing — the next cycle asks again.
//!
//! Nothing is ever installed on the human's behalf, and the whole thing can be
//! turned off: with no address to ask, no task is started and no request is ever
//! made.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use verkstead_render::UpdateNotice;

/// Where the latest release is asked about.
///
/// `releases/latest` rather than the tag list, because it excludes
/// pre-releases: a rehearsal tag is marked as one and is not something to tell
/// the human to update to. Before the first real release the endpoint answers
/// `404`, which is no news rather than a failure.
pub(crate) const LATEST_RELEASE: &str =
    "https://api.github.com/repos/tobico/verkstead/releases/latest";

/// How often to ask. A release is not an urgent thing to hear about, and the
/// startup poll covers the server that was restarted just after one.
const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

/// How long to wait on GitHub before giving up. Whatever the answer would have
/// been, it will be asked for again tomorrow.
const REACHABLE_WITHIN: Duration = Duration::from_secs(10);

/// Who is asking. GitHub refuses a caller that will not name itself, and this
/// is the only place Verkstead has to introduce itself to anyone.
const ASKING: &str = concat!("verkstead/", env!("CARGO_PKG_VERSION"));

/// The release this binary was built as, which is what a tag is compared
/// against.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// The latest stable release the server has learned of, as the handlers read
/// it: `None` until the check has an answer, and for as long as a server with
/// the check turned off runs.
///
/// Cheap to clone and shared with whatever task is doing the asking: the
/// release is one small value replaced whole, so a reader never waits on more
/// than the swap.
#[derive(Clone)]
pub(crate) struct Updates(Arc<RwLock<Option<semver::Version>>>);

impl Updates {
    /// Nothing learned yet — what a server answers between starting and its
    /// first poll coming back, and what one with the check turned off answers
    /// for as long as it runs. It says there is nothing to update to, which is
    /// the honest thing to say when you do not know.
    pub(crate) fn nothing_learned() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }

    /// The verdict as it stands, for this device.
    pub(crate) fn notice(&self) -> UpdateNotice {
        match self.latest() {
            Some(released) if released > running() => UpdateNotice::Available {
                version: released.to_string(),
            },
            _ => UpdateNotice::Current,
        }
    }

    /// Whether a release newer than `version` exists — what tags a row of the
    /// Devices section *update available*, this device's included.
    ///
    /// False where nothing has been learned, and for a device that says no
    /// version or one that will not parse: the missing line under its name is
    /// the sign for the first, and a tag guessed from the second would be a
    /// guess.
    pub(crate) fn newer_than(&self, version: Option<&str>) -> bool {
        self.newer(version).is_some()
    }

    /// The release newer than `version`, where there is one — what this
    /// device's confirm card names as the version its Update goes onto. `None`
    /// wherever [`Updates::newer_than`] is false.
    pub(crate) fn newer(&self, version: Option<&str>) -> Option<String> {
        let latest = self.latest()?;

        version
            .and_then(|version| semver::Version::parse(version).ok())
            .is_some_and(|version| latest > version)
            .then(|| latest.to_string())
    }

    fn latest(&self) -> Option<semver::Version> {
        self.0
            .read()
            .expect("the release lock is never poisoned")
            .clone()
    }

    fn learned(&self, latest: Option<semver::Version>) {
        *self.0.write().expect("the release lock is never poisoned") = latest;
    }
}

/// **The gap between a member's release and this device's**, as a sentence
/// and the one action after it: *winvm is on 0.2.0 and this device is on 0.2.2.
/// Update winvm.* — or *Update this device.* where this device is the older
/// one. A member that says no version is from before versions were said, so
/// older than any (ADR-0027, *Seeing versions*): *winvm is on an older release
/// than this device's 0.2.2. Update winvm.*
///
/// `None` where the two are the same, and where this device has no version to
/// compare against. One sentence for every place a call to a member can fail,
/// because it is the one place two devices a release apart become legible.
pub(crate) fn gap(name: &str, theirs: Option<&str>, ours: Option<&str>) -> Option<String> {
    let ours = ours?;

    let Some(theirs) = theirs else {
        return Some(format!(
            "{name} is on an older release than this device's {ours}. Update {name}."
        ));
    };

    if theirs == ours {
        return None;
    }

    let this_is_older = match (semver::Version::parse(theirs), semver::Version::parse(ours)) {
        (Ok(theirs), Ok(ours)) => ours < theirs,
        _ => false,
    };

    let update = match this_is_older {
        true => "this device".to_owned(),
        false => name.to_owned(),
    };

    Some(format!(
        "{name} is on {theirs} and this device is on {ours}. Update {update}."
    ))
}

/// The release this binary was built as.
fn running() -> semver::Version {
    semver::Version::parse(RUNNING).expect("the version cargo built this with is a semver version")
}

/// Start asking, and hand back the verdict the handler reads.
///
/// `releases` is where to ask — [`LATEST_RELEASE`] in the running server, and a
/// server the test stood up itself under test. `None` is the check turned off:
/// no task, and so no request ever made.
pub(crate) fn watching(releases: Option<&str>) -> Updates {
    let updates = Updates::nothing_learned();

    let Some(releases) = releases else {
        return updates;
    };
    let releases = releases.to_owned();

    let client = match reqwest::Client::builder()
        .timeout(REACHABLE_WITHIN)
        .user_agent(ASKING)
        .build()
    {
        Ok(client) => client,
        // Nothing left to do but say nothing: this is the whole of the check,
        // and it is not something the human asked for by starting the server.
        Err(error) => {
            tracing::warn!(error = ?error, "the update check could not be started");
            return updates;
        }
    };

    let asking = updates.clone();
    tokio::spawn(async move {
        loop {
            match ask(&client, &releases).await {
                Ok(latest) => asking.learned(latest),
                // Left standing rather than cleared. GitHub being out of reach
                // is no reason to stop naming an update already learned of, and
                // it costs nothing: tomorrow's cycle asks again.
                Err(error) => {
                    tracing::debug!(error = ?error, "asking about newer releases failed");
                }
            }

            tokio::time::sleep(DAILY).await;
        }
    });

    updates
}

/// What GitHub answers with, of which one field is any of our business.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// Ask once, and say which release is the latest.
async fn ask(client: &reqwest::Client, releases: &str) -> Result<Option<semver::Version>> {
    let response = client
        .get(releases)
        .send()
        .await
        .context("reaching GitHub")?;

    // There is no latest release until there is one, and GitHub says so with a
    // 404. Nothing to update to, rather than something to report.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let release: Release = response
        .error_for_status()
        .context("asking GitHub for the latest release")?
        .json()
        .await
        .context("reading what GitHub answered")?;

    Ok(released(&release.tag_name))
}

/// The release one tag names.
///
/// A tag that will not parse is no news, same as no release at all: the
/// alternative is telling the human to update to something we cannot name.
fn released(tag: &str) -> Option<semver::Version> {
    // Releases are tagged `v0.1.0`; the version inside one is not.
    let released = semver::Version::parse(tag.trim_start_matches('v')).ok();

    if released.is_none() {
        tracing::debug!(
            tag,
            "the latest release is not tagged with a version we can read"
        );
    }

    released
}

#[cfg(test)]
mod tests {
    use super::{RUNNING, UpdateNotice, Updates, gap, released};

    /// The server as it stands once GitHub has answered with `tag`.
    fn answered(tag: &str) -> Updates {
        let updates = Updates::nothing_learned();
        updates.learned(released(tag));
        updates
    }

    #[test]
    fn a_higher_tag_is_an_update_named_without_its_v() {
        assert_eq!(
            answered("v99.0.0").notice(),
            UpdateNotice::Available {
                version: "99.0.0".to_owned(),
            },
        );
    }

    #[test]
    fn the_running_version_is_nothing_to_update_to() {
        assert_eq!(
            answered(&format!("v{RUNNING}")).notice(),
            UpdateNotice::Current
        );
    }

    #[test]
    fn a_tag_that_will_not_parse_is_no_news() {
        assert_eq!(answered("the-first-one").notice(), UpdateNotice::Current);
        assert!(!answered("the-first-one").newer_than(Some("0.0.1")));
    }

    /// The release is kept even where this device is on it, because a member
    /// can still be behind it.
    #[test]
    fn a_device_behind_an_up_to_date_one_is_still_behind() {
        let updates = answered(&format!("v{RUNNING}"));

        assert!(updates.newer_than(Some("0.0.1")));
        assert!(!updates.newer_than(Some(RUNNING)));
    }

    /// Compared as versions rather than as strings: 0.10.0 is after 0.9.0.
    #[test]
    fn versions_are_compared_as_versions() {
        let updates = answered("v0.10.0");

        assert!(updates.newer_than(Some("0.9.0")));
        assert!(!updates.newer_than(Some("0.10.0")));
        assert!(!updates.newer_than(Some("0.11.0")));
    }

    /// No answer yet, a device saying no version, and one saying a version
    /// that will not parse are all no tag.
    #[test]
    fn what_cannot_be_compared_is_not_tagged() {
        assert!(!Updates::nothing_learned().newer_than(Some("0.0.1")));
        assert!(!answered("v99.0.0").newer_than(None));
        assert!(!answered("v99.0.0").newer_than(Some("not a version")));
    }

    /// The gap names both versions and the older device as the one to update.
    #[test]
    fn a_gap_says_both_releases_and_which_to_update() {
        assert_eq!(
            gap("winvm", Some("0.2.0"), Some("0.2.2")).as_deref(),
            Some("winvm is on 0.2.0 and this device is on 0.2.2. Update winvm."),
        );
        assert_eq!(
            gap("winvm", Some("0.10.0"), Some("0.9.0")).as_deref(),
            Some("winvm is on 0.10.0 and this device is on 0.9.0. Update this device."),
        );
    }

    /// A member that says no version is from before versions were said.
    #[test]
    fn a_member_with_no_version_is_on_an_older_release() {
        assert_eq!(
            gap("winvm", None, Some("0.2.2")).as_deref(),
            Some("winvm is on an older release than this device's 0.2.2. Update winvm."),
        );
    }

    #[test]
    fn the_same_release_is_no_gap() {
        assert_eq!(gap("winvm", Some("0.2.2"), Some("0.2.2")), None);
        assert_eq!(gap("winvm", Some("0.2.0"), None), None);
    }
}
