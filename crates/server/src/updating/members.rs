//! A member's Update, pressed on its row here: the telling, and the member's
//! own account of it read back for the row (ADR-0027, *A device updates
//! itself*).
//!
//! **This device tells and the member does the whole of it.** The press is
//! put to the member over the link — [`crate::peer::updating`] is the far end
//! of it — and what the member does is what a press on its own row does:
//! refuse by name where its install is updated by hand, or ask its own
//! desktop app to fetch its own latest stable release. No bits cross.
//!
//! **Held here, in memory, for every workbench of this device to draw.** The
//! press is this device's, so the record of it is too: a phone that opens the
//! Devices section mid-Update reads the same *updating…* the laptop that
//! pressed it does, out of [`crate::device::Devices::listing`]. Nothing is
//! written down — a member's Update ends in that member restarting, and a
//! press left over from a run of this server would be a row reading
//! *updating…* about something nobody here is watching.
//!
//! **And read back from the member while it goes.** Its progress reports are
//! its own server's and carry no Nudge, for the reason
//! [`crate::ui`]'s `update_progressed` gives, so the member's own reading is
//! asked again on a short clock — [`READING_AGAIN`] — and at once whenever its
//! Nudge stream says its Devices moved, which is what a failure on it says.
//!
//! **Every outcome but an Update going is the row's trouble line**, standing
//! until the next press: the member's own refusal, its updater's failure in
//! its own words, or — the press finding the member not answering — that it
//! was unreachable and so wasn't updated. Nothing is remembered for later
//! (ADR-0027): the human presses again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::http::StatusCode;
use verkstead_render::{MemberUpdate, ThisUpdating, UpdateTold};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::device::{Devices, Unrelayed};
use crate::peer::updating::UPDATE;
use crate::relaying::{Call, Streamed};

/// How often a member's Update is read again while it goes: **a second**,
/// which is how often the row is polled while it reads *updating…*.
const READING_AGAIN: Duration = Duration::from_secs(1);

/// The most a member's answer about its Update may be: a percentage and a
/// sentence, so a few kilobytes is far more than any Verkstead says.
const MOST_AN_ANSWER_IS: usize = 16 * 1024;

/// Every member Update pressed here, by Device Id, shared by every clone of the
/// Devices handle.
#[derive(Debug, Clone, Default)]
pub(crate) struct MemberUpdates(Arc<Mutex<HashMap<String, Held>>>);

/// One member's: the Update going, and the trouble line the last press left.
#[derive(Debug, Clone, Default)]
struct Held {
    going: Option<ThisUpdating>,
    trouble: Option<String>,

    /// Which press this is, so that what a reading of an earlier one finds is
    /// never written over a later one.
    press: u64,
}

impl MemberUpdates {
    fn held(&self) -> MutexGuard<'_, HashMap<String, Held>> {
        self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    /// Take a press on `device`'s row: *updating…* from here, and its last
    /// line gone. The press's number, or `None` where one is already going,
    /// which a second press reads rather than starts again.
    fn begin(&self, device: &str) -> Option<u64> {
        let mut held = self.held();
        let held = held.entry(device.to_owned()).or_default();

        if held.going.is_some() {
            return None;
        }

        held.press += 1;
        held.going = Some(ThisUpdating::default());
        held.trouble = None;

        Some(held.press)
    }

    /// Whether `press` is still the Update going on `device`.
    fn following(&self, device: &str, press: u64) -> bool {
        self.held()
            .get(device)
            .is_some_and(|held| held.press == press && held.going.is_some())
    }

    /// The member's own account of `press`, going: `true` where the row moved.
    fn going(&self, device: &str, press: u64, update: ThisUpdating) -> bool {
        let mut held = self.held();

        let Some(held) = held.get_mut(device).filter(|held| held.press == press) else {
            return false;
        };

        let moved = held.going != Some(update);
        held.going = Some(update);
        moved
    }

    /// And `press` ending in `line` rather than an Update: `true` where it
    /// was still going.
    fn troubled(&self, device: &str, press: u64, line: String) -> bool {
        let mut held = self.held();

        let Some(held) = held.get_mut(device).filter(|held| held.press == press) else {
            return false;
        };

        held.going = None;
        held.trouble = Some(line);
        true
    }

    /// What `device`'s row is drawn with: the Update going, and the line the
    /// last press left.
    pub(crate) fn reading(&self, device: &str) -> (Option<ThisUpdating>, Option<String>) {
        self.held()
            .get(device)
            .map_or((None, None), |held| (held.going, held.trouble.clone()))
    }
}

/// Why a press on a member's row was not put to it at all.
#[derive(Debug)]
pub(crate) enum Untold {
    /// The device is no member of this cluster.
    NoSuchMember,

    /// It is an **Annex**, which its Keeper re-pins to its own release at
    /// every start (ADR-0024) — nothing to press.
    Annex { name: String },

    /// The membership couldn't be read.
    Unreadable(anyhow::Error),
}

/// **Update** on `device`'s row: tell it to update itself, and hold what it
/// said for its row — an Update going, read back from here on, or the line
/// saying why not (see this module's documentation).
///
/// `Ok` whatever the member made of it, the row being where that is said; a
/// press while one is going reads the one there is. Every workbench of this
/// device is told the row moved.
pub(crate) async fn tell(state: &AppState, devices: &Devices, device: &str) -> Result<(), Untold> {
    let rows = devices
        .membership()
        .rows()
        .await
        .map_err(Untold::Unreadable)?;

    let Some(member) = rows.into_iter().find(|member| member.device == device) else {
        return Err(Untold::NoSuchMember);
    };

    if member.kept_by.is_some() {
        return Err(Untold::Annex { name: member.name });
    }

    let updates = devices.members_updating();

    let Some(press) = updates.begin(device) else {
        return Ok(());
    };

    let told = UpdateTold {
        release: state.updates.newer(member.version.as_deref()),
    };

    tracing::info!(
        device,
        release = told.release.as_deref().unwrap_or("unknown"),
        "Update was pressed on a member's row, so the member is told to update itself",
    );

    let call = Call {
        method: reqwest::Method::POST,
        onwards: UPDATE.to_owned(),
        headers: crate::relaying::as_json(),
        body: Streamed::saying(serde_json::to_vec(&told).expect("a release is always JSON")),
    };

    let name = &member.name;

    match devices.relay(device, call).await {
        Ok(answered) if answered.status().is_success() => {
            match said::<MemberUpdate>(answered).await {
                Some(said) => {
                    updates.going(device, press, said.updating.unwrap_or_default());

                    tokio::spawn(following(state.clone(), device.to_owned(), press));
                }

                None => {
                    updates.troubled(
                        device,
                        press,
                        format!(
                            "{name} answered in a way this device can't read, so it wasn't updated."
                        ),
                    );
                }
            }
        }

        // The member's own sentence, naming itself: its install is updated by
        // hand.
        Ok(answered) if answered.status() == StatusCode::CONFLICT => {
            let status = answered.status();
            let why = crate::relaying::bounded(answered, MOST_AN_ANSWER_IS)
                .await
                .ok()
                .map(|said| String::from_utf8_lossy(&said).trim().to_owned())
                .filter(|said| !said.is_empty())
                .unwrap_or_else(|| format!("{name} refused the update: {status}."));

            updates.troubled(device, press, why);
        }

        Ok(answered) => {
            updates.troubled(
                device,
                press,
                format!("{name} refused the update: {}.", answered.status()),
            );
        }

        Err(Unrelayed::Unreachable(why)) => {
            tracing::info!(
                device,
                "a member told to update itself wasn't reached: {why:#}"
            );

            updates.troubled(
                device,
                press,
                format!("{name} couldn't be reached, so it wasn't updated."),
            );
        }

        // The membership read a moment ago said otherwise: unlinked, or
        // unreadable, in between.
        Err(Unrelayed::NoSuchMember | Unrelayed::ThisDevice | Unrelayed::Unreadable(_)) => {
            tracing::error!(
                device,
                "a member couldn't be dialled to be told to update itself"
            );

            updates.troubled(device, press, format!("{name} couldn't be told to update."));
        }
    }

    state.nudges.announce_here(Nudge::Devices);

    Ok(())
}

/// What a member answered, read as a `T`, or `None` where it can't be.
async fn said<T: serde::de::DeserializeOwned>(answered: reqwest::Response) -> Option<T> {
    let said = crate::relaying::bounded(answered, MOST_AN_ANSWER_IS)
        .await
        .ok()?;

    serde_json::from_slice(&said).ok()
}

/// Read `device`'s own account of `press` back for as long as it goes: on its
/// Nudge stream saying its Devices moved, and every [`READING_AGAIN`] between.
///
/// Ends once the press is no longer the one going — a line has taken its
/// place, or a later press has — and with the server.
async fn following(state: AppState, device: String, press: u64) {
    let Some(devices) = state.devices.clone() else {
        return;
    };

    let mut moved = state.nudges.subscribe();
    let mut again = tokio::time::interval(READING_AGAIN);

    // The first tick is now, and the press has just been answered.
    again.tick().await;

    loop {
        tokio::select! {
            heard = moved.recv() => match heard {
                Ok(heard) if heard.device.as_deref() == Some(device.as_str())
                    && matches!(heard.moved, Nudge::Devices | Nudge::Everything) => {}
                Ok(_) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            },

            _ = again.tick() => {}
        }

        if !devices.members_updating().following(&device, press) {
            return;
        }

        read_back(&state, &devices, &device, press).await;
    }
}

/// Ask `device` how its Update stands, and draw what it said on its row: the
/// percent, or its updater's failure as the line, in its own words — *crow
/// couldn't be updated: …*. A read that didn't land changes nothing.
async fn read_back(state: &AppState, devices: &Devices, device: &str, press: u64) {
    let Ok(said) = crate::relaying::answer_of(
        devices,
        device,
        UPDATE.to_owned(),
        "Update",
        MOST_AN_ANSWER_IS,
    )
    .await
    else {
        return;
    };

    let Ok(said) = serde_json::from_slice::<MemberUpdate>(&said) else {
        tracing::warn!(
            device,
            "a member answered its Update in a way this device can't read"
        );
        return;
    };

    let moved = match (said.updating, said.trouble) {
        (Some(update), _) => devices.members_updating().going(device, press, update),
        (None, Some(line)) => devices.members_updating().troubled(device, press, line),
        (None, None) => false,
    };

    if moved {
        state.nudges.announce_here(Nudge::Devices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A press is taken once while it goes, and a line ends it.
    #[test]
    fn a_press_goes_until_a_line_ends_it() {
        let updates = MemberUpdates::default();

        let press = updates.begin("crow").expect("nothing is going");
        assert_eq!(updates.begin("crow"), None);
        assert_eq!(
            updates.reading("crow"),
            (Some(ThisUpdating { percent: None }), None)
        );

        assert!(updates.going("crow", press, ThisUpdating { percent: Some(40) }));
        assert!(!updates.going("crow", press, ThisUpdating { percent: Some(40) }));

        assert!(updates.troubled("crow", press, "crow couldn't be updated.".to_owned()));
        assert_eq!(
            updates.reading("crow"),
            (None, Some("crow couldn't be updated.".to_owned()))
        );
        assert!(!updates.following("crow", press));
    }

    /// The next press takes the line away, and a reading of the earlier one
    /// lands on nothing.
    #[test]
    fn a_later_press_is_not_written_over() {
        let updates = MemberUpdates::default();

        let first = updates.begin("crow").unwrap();
        updates.troubled("crow", first, "crow couldn't be reached.".to_owned());

        let second = updates.begin("crow").unwrap();
        assert_eq!(updates.reading("crow").1, None);

        assert!(!updates.going("crow", first, ThisUpdating { percent: Some(90) }));
        assert!(!updates.troubled("crow", first, "late".to_owned()));
        assert!(updates.following("crow", second));
        assert_eq!(updates.reading("wolf"), (None, None));
    }
}
