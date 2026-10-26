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
//! **And then the member goes quiet and comes back.** Its app quitting to
//! install ends the Nudge stream this device holds to it, and from there the
//! row reads *restarting…* — said by this device's server, which is still up,
//! where this device's own row has the viewer say it. The stream taken up
//! again reads the member's identity again (see
//! [`crate::relaying::freshness`]), and that is the whole of what decides it:
//! on the release the press carried, or any higher than the one it ran at the
//! press, the Update is over and the row shows the version with nothing else.
//!
//! **Every outcome but an Update going, or one that came back, is the row's
//! trouble line**, standing until the next press: the member's own refusal,
//! its updater's failure in its own words, the press finding the member not
//! answering — so it wasn't updated — or the member back on the release it
//! ran. And one that hasn't come back [`COMING_BACK_WITHIN`] the press says
//! so, until it turns up on the new release after all. Nothing is remembered
//! for later (ADR-0027): the human presses again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

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

/// How long after the press a member has to come back before its row says it
/// didn't: **ten minutes** (Set 1416). A download, an install and a restart
/// are a minute or two on any link a cluster is on; ten is long enough that
/// the line is never about a slow one, and short enough that Update all, which
/// waits on it, doesn't wait all evening on a laptop that was shut.
pub(crate) const COMING_BACK_WITHIN: Duration = Duration::from_secs(10 * 60);

/// The most a member's answer about its Update may be: a percentage and a
/// sentence, so a few kilobytes is far more than any Verkstead says.
const MOST_AN_ANSWER_IS: usize = 16 * 1024;

/// Every member Update pressed here, by Device Id, shared by every clone of the
/// Devices handle.
#[derive(Debug, Clone)]
pub(crate) struct MemberUpdates {
    held: Arc<Mutex<HashMap<String, Held>>>,

    /// Counted up on every change to any of them, which is what anything
    /// waiting on one ending waits on — see [`MemberUpdates::settled`].
    moved: Arc<watch::Sender<u64>>,

    /// How long a member has to come back: [`COMING_BACK_WITHIN`], shorter in
    /// a suite.
    within: Duration,
}

impl Default for MemberUpdates {
    fn default() -> Self {
        MemberUpdates::within(COMING_BACK_WITHIN)
    }
}

/// One member's: the Update going, and the trouble line the last press left.
#[derive(Debug, Clone, Default)]
struct Held {
    going: Option<ThisUpdating>,

    /// And whether the member's stream has dropped since the press, which is
    /// its app quitting to install: *restarting…*.
    restarting: bool,

    /// The version a take-up read since it went quiet, where it wasn't the
    /// new one — for the follower to ask the member about. `Some(None)` is a
    /// member that came back saying no version.
    back_on: Option<Option<String>>,

    trouble: Option<String>,

    /// Whether `trouble` is the deadline's, which the member turning up on the
    /// new release after all takes away.
    late: bool,

    /// What the press found: the member's name, the release it ran and the
    /// one it was told of, which is what the coming back is held up to.
    name: String,
    from: Option<String>,
    release: Option<String>,

    /// Which press this is, so that what a reading of an earlier one finds is
    /// never written over a later one.
    press: u64,
}

impl Held {
    /// Whether coming back on `version` is this press's Update done: the
    /// release it was told of, or any higher than the one it ran.
    fn updated_by(&self, version: Option<&str>) -> bool {
        let Some(version) = version else {
            return false;
        };

        if self.release.as_deref() == Some(version) {
            return true;
        }

        let Ok(version) = semver::Version::parse(version) else {
            return false;
        };

        // A member that said no version at the press was on an older release
        // than any (ADR-0027, *Seeing versions*).
        match self.from.as_deref().map(semver::Version::parse) {
            Some(Ok(from)) => version > from,
            Some(Err(_)) => false,
            None => true,
        }
    }

    /// The line a member back on `back` rather than the new release leaves,
    /// naming both: *crow couldn't be updated: it came back on 0.2.2, not
    /// 0.3.0.*
    fn came_back_on(&self, back: Option<&str>) -> String {
        let name = &self.name;

        match (back, self.release.as_deref()) {
            (Some(back), Some(release)) => {
                format!("{name} couldn't be updated: it came back on {back}, not {release}.")
            }
            (Some(back), None) => format!("{name} couldn't be updated: it came back on {back}."),
            (None, _) => {
                format!("{name} couldn't be updated: it came back on the release it was on.")
            }
        }
    }
}

/// What a press found, which its record keeps: see [`Held`].
struct Pressed<'a> {
    name: &'a str,
    from: Option<String>,
    release: Option<String>,
}

impl MemberUpdates {
    /// None pressed, a member having `within` of the press to come back.
    pub(crate) fn within(within: Duration) -> MemberUpdates {
        MemberUpdates {
            held: Arc::default(),
            moved: Arc::new(watch::Sender::new(0)),
            within,
        }
    }

    fn held(&self) -> MutexGuard<'_, HashMap<String, Held>> {
        self.held
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn moved(&self) {
        self.moved
            .send_modify(|count| *count = count.wrapping_add(1));
    }

    /// Take a press on `device`'s row: *updating…* from here, and its last
    /// line gone. The press's number, or `None` where one is already going,
    /// which a second press reads rather than starts again.
    fn begin(&self, device: &str, pressed: Pressed<'_>) -> Option<u64> {
        let press = {
            let mut held = self.held();
            let held = held.entry(device.to_owned()).or_default();

            if held.going.is_some() {
                return None;
            }

            *held = Held {
                going: Some(ThisUpdating::default()),
                name: pressed.name.to_owned(),
                from: pressed.from,
                release: pressed.release,
                press: held.press + 1,
                ..Held::default()
            };

            held.press
        };

        self.moved();
        Some(press)
    }

    /// Whether `press` is still the Update going on `device`.
    fn following(&self, device: &str, press: u64) -> bool {
        self.held()
            .get(device)
            .is_some_and(|held| held.press == press && held.going.is_some())
    }

    /// The member's own account of `press`, going: `true` where the row moved.
    ///
    /// Read as it came back from the member, so an account that says it is
    /// going again after its stream dropped is a member that never went —
    /// a link that blipped — and the row reads *updating…* again.
    fn going(&self, device: &str, press: u64, update: ThisUpdating) -> bool {
        let moved = {
            let mut held = self.held();

            let Some(held) = held
                .get_mut(device)
                .filter(|held| held.press == press && held.going.is_some())
            else {
                return false;
            };

            let moved = held.going != Some(update) || held.restarting;
            held.going = Some(update);
            held.restarting = false;
            held.back_on = None;
            moved
        };

        if moved {
            self.moved();
        }
        moved
    }

    /// And `press` ending in `line` rather than an Update: `true` where it
    /// was still going.
    fn troubled(&self, device: &str, press: u64, line: String) -> bool {
        self.ended(device, press, line, false)
    }

    /// The same, the line being the deadline's: see [`Held::late`].
    fn late(&self, device: &str, press: u64, line: String) -> bool {
        self.ended(device, press, line, true)
    }

    fn ended(&self, device: &str, press: u64, line: String, late: bool) -> bool {
        {
            let mut held = self.held();

            let Some(held) = held
                .get_mut(device)
                .filter(|held| held.press == press && held.going.is_some())
            else {
                return false;
            };

            held.going = None;
            held.restarting = false;
            held.back_on = None;
            held.trouble = Some(line);
            held.late = late;
        }

        self.moved();
        true
    }

    /// `device`'s Nudge stream ended: where an Update is going on it, that is
    /// its app quitting to install, and its row reads *restarting…*. `true`
    /// where the row moved.
    pub(crate) fn went_quiet(&self, device: &str) -> bool {
        {
            let mut held = self.held();

            let Some(held) = held
                .get_mut(device)
                .filter(|held| held.going.is_some() && !held.restarting)
            else {
                return false;
            };

            held.restarting = true;
            held.back_on = None;
        }

        tracing::info!(
            device,
            "a member updating itself went quiet, so its row reads restarting"
        );

        self.moved();
        true
    }

    /// And `device`'s stream taken up again, its identity read beside it
    /// saying `version`. `true` where its row moved.
    ///
    /// **On the new release, the Update is done** — going, or past its
    /// deadline — and the row shows the version with nothing else. On any
    /// other, where it had gone quiet, the follower asks the member before
    /// saying anything: its own relaunch check may have its own line, and an
    /// account of an Update still going is a link that blipped rather than a
    /// restart. A take-up of a member that never went quiet is nothing.
    pub(crate) fn taken_up(&self, device: &str, version: Option<&str>) -> bool {
        let moved = {
            let mut held = self.held();

            let Some(held) = held.get_mut(device) else {
                return false;
            };

            let pending = held.going.is_some() || held.late;

            if pending && held.updated_by(version) {
                tracing::info!(
                    device,
                    version = version.unwrap_or("none"),
                    "a member told to update itself came back on the new release",
                );

                held.going = None;
                held.restarting = false;
                held.back_on = None;
                held.trouble = None;
                held.late = false;
                true
            } else if held.going.is_some() && held.restarting {
                held.back_on = Some(version.map(str::to_owned));
                false
            } else {
                return false;
            }
        };

        // Moved or not, the follower is woken to look at what it came back on.
        self.moved();
        moved
    }

    /// What a take-up since `press` went quiet read, where it wasn't the new
    /// release, taken so that it is asked about once.
    fn came_back(&self, device: &str, press: u64) -> Option<Option<String>> {
        self.held()
            .get_mut(device)
            .filter(|held| held.press == press)
            .and_then(|held| held.back_on.take())
    }

    /// The line `press` leaves where the member came back on `back`.
    fn came_back_line(&self, device: &str, press: u64, back: Option<&str>) -> Option<String> {
        self.held()
            .get(device)
            .filter(|held| held.press == press)
            .map(|held| held.came_back_on(back))
    }

    /// What `device`'s row is drawn with: the Update going, whether it has
    /// gone quiet, and the line the last press left.
    pub(crate) fn reading(&self, device: &str) -> (Option<ThisUpdating>, bool, Option<String>) {
        self.held().get(device).map_or((None, false, None), |held| {
            (held.going, held.restarting, held.trouble.clone())
        })
    }

    /// Wait until no Update is going on `device`: it came back, or it left a
    /// line, or it is past its deadline. At once where none is.
    ///
    /// **The one place a member's Update is awaited**, which is what Update
    /// all waits on before this device updates itself last.
    pub(crate) async fn settled(&self, device: &str) {
        let mut moved = self.moved.subscribe();

        loop {
            let going = self
                .held()
                .get(device)
                .is_some_and(|held| held.going.is_some());

            if !going || moved.changed().await.is_err() {
                return;
            }
        }
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

    let told = UpdateTold {
        release: state.updates.newer(member.version.as_deref()),
    };

    let pressed = Pressed {
        name: &member.name,
        from: member.version.clone(),
        release: told.release.clone(),
    };

    let Some(press) = updates.begin(device, pressed) else {
        return Ok(());
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
/// Nudge stream saying its Devices moved, and every [`READING_AGAIN`] between —
/// and, once it has come back on a release that isn't the new one, ask it
/// what it makes of that. At the deadline, its row says it didn't come back.
///
/// Ends once the press is no longer the one going — a line has taken its
/// place, or a later press has, or the member came back — and with the server.
async fn following(state: AppState, device: String, press: u64) {
    let Some(devices) = state.devices.clone() else {
        return;
    };

    let updates = devices.members_updating().clone();

    let mut heard = state.nudges.subscribe();
    let mut moved = updates.moved.subscribe();
    let mut again = tokio::time::interval(READING_AGAIN);
    let deadline = tokio::time::sleep_until(Instant::now() + updates.within);
    tokio::pin!(deadline);

    // The first tick is now, and the press has just been answered.
    again.tick().await;

    loop {
        tokio::select! {
            heard = heard.recv() => match heard {
                Ok(heard) if heard.device.as_deref() == Some(device.as_str())
                    && matches!(heard.moved, Nudge::Devices | Nudge::Everything) => {}
                Ok(_) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            },

            changed = moved.changed() => if changed.is_err() {
                return;
            },

            _ = again.tick() => {}

            () = &mut deadline => {
                let line = updates
                    .held()
                    .get(&device)
                    .map(|held| format!("{} didn't come back after its update.", held.name));

                if let Some(line) = line
                    && updates.late(&device, press, line)
                {
                    tracing::warn!(device, "a member told to update itself didn't come back in time");
                    state.nudges.announce_here(Nudge::Devices);
                }

                return;
            }
        }

        if !updates.following(&device, press) {
            return;
        }

        match updates.came_back(&device, press) {
            Some(back) => came_back(&state, &devices, &device, press, back.as_deref()).await,
            None => read_back(&state, &devices, &device, press).await,
        }
    }
}

/// `device` came back from going quiet on `back`, which is not the release
/// `press` was for: ask it how its Update stands before saying so. Its own
/// relaunch check's line, where it has one, is the one line; an Update still
/// going there is a link that blipped, and the row reads *updating…* again;
/// otherwise the row says what it came back on.
async fn came_back(
    state: &AppState,
    devices: &Devices,
    device: &str,
    press: u64,
    back: Option<&str>,
) {
    let updates = devices.members_updating();

    let said = crate::relaying::answer_of(
        devices,
        device,
        UPDATE.to_owned(),
        "Update",
        MOST_AN_ANSWER_IS,
    )
    .await
    .ok()
    .and_then(|said| serde_json::from_slice::<MemberUpdate>(&said).ok())
    .unwrap_or_default();

    let moved = match (said.updating, said.trouble) {
        (Some(update), _) => updates.going(device, press, update),
        (None, Some(line)) => updates.troubled(device, press, line),
        (None, None) => match updates.came_back_line(device, press, back) {
            Some(line) => {
                tracing::warn!(
                    device,
                    version = back.unwrap_or("none"),
                    "a member told to update itself came back on the release it ran",
                );
                updates.troubled(device, press, line)
            }
            None => false,
        },
    };

    if moved {
        state.nudges.announce_here(Nudge::Devices);
    }
}

/// Ask `device` how its Update stands, and draw what it said on its row: the
/// percent, or its updater's failure as the line, in its own words — *crow
/// couldn't be updated: …*. A read that didn't land changes nothing.
async fn read_back(state: &AppState, devices: &Devices, device: &str, press: u64) {
    // Gone quiet: there is nobody to ask until its stream is taken up again,
    // and what is asked then is [`came_back`]'s.
    if devices.members_updating().reading(device).1 {
        return;
    }

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

    /// crow pressed on 0.2.2, told of 0.3.0.
    fn crow() -> Pressed<'static> {
        Pressed {
            name: "crow",
            from: Some("0.2.2".to_owned()),
            release: Some("0.3.0".to_owned()),
        }
    }

    /// A press is taken once while it goes, and a line ends it.
    #[test]
    fn a_press_goes_until_a_line_ends_it() {
        let updates = MemberUpdates::default();

        let press = updates.begin("crow", crow()).expect("nothing is going");
        assert_eq!(updates.begin("crow", crow()), None);
        assert_eq!(
            updates.reading("crow"),
            (Some(ThisUpdating { percent: None }), false, None)
        );

        assert!(updates.going("crow", press, ThisUpdating { percent: Some(40) }));
        assert!(!updates.going("crow", press, ThisUpdating { percent: Some(40) }));

        assert!(updates.troubled("crow", press, "crow couldn't be updated.".to_owned()));
        assert_eq!(
            updates.reading("crow"),
            (None, false, Some("crow couldn't be updated.".to_owned()))
        );
        assert!(!updates.following("crow", press));
    }

    /// The next press takes the line away, and a reading of the earlier one
    /// lands on nothing.
    #[test]
    fn a_later_press_is_not_written_over() {
        let updates = MemberUpdates::default();

        let first = updates.begin("crow", crow()).unwrap();
        updates.troubled("crow", first, "crow couldn't be reached.".to_owned());

        let second = updates.begin("crow", crow()).unwrap();
        assert_eq!(updates.reading("crow").2, None);

        assert!(!updates.going("crow", first, ThisUpdating { percent: Some(90) }));
        assert!(!updates.troubled("crow", first, "late".to_owned()));
        assert!(updates.following("crow", second));
        assert_eq!(updates.reading("wolf"), (None, false, None));
    }

    /// Going quiet is *restarting…*, and coming back on the new release is the
    /// Update over, with nothing left on the row.
    #[test]
    fn back_on_the_new_release_is_the_update_over() {
        let updates = MemberUpdates::default();
        let press = updates.begin("crow", crow()).unwrap();

        assert!(updates.went_quiet("crow"));
        assert!(!updates.went_quiet("crow"));
        assert_eq!(
            updates.reading("crow"),
            (Some(ThisUpdating { percent: None }), true, None)
        );

        assert!(updates.taken_up("crow", Some("0.3.0")));
        assert_eq!(updates.reading("crow"), (None, false, None));
        assert!(!updates.following("crow", press));
    }

    /// Any release higher than the one it ran counts, and a take-up of a
    /// member that never went quiet on the old one is nothing.
    #[test]
    fn a_higher_release_counts_and_a_blip_before_the_restart_does_not() {
        let updates = MemberUpdates::default();
        let press = updates.begin("crow", crow()).unwrap();

        assert!(!updates.taken_up("crow", Some("0.2.2")));
        assert_eq!(updates.came_back("crow", press), None);
        assert!(updates.following("crow", press));

        updates.went_quiet("crow");
        assert!(updates.taken_up("crow", Some("0.3.1")));
        assert_eq!(updates.reading("crow"), (None, false, None));
    }

    /// Back on the release it ran is for the follower to ask about once, and
    /// its line names both releases.
    #[test]
    fn back_on_the_old_release_is_asked_about_once() {
        let updates = MemberUpdates::default();
        let press = updates.begin("crow", crow()).unwrap();

        updates.went_quiet("crow");
        assert!(!updates.taken_up("crow", Some("0.2.2")));

        assert_eq!(
            updates.came_back("crow", press),
            Some(Some("0.2.2".to_owned()))
        );
        assert_eq!(updates.came_back("crow", press), None);
        assert_eq!(
            updates
                .came_back_line("crow", press, Some("0.2.2"))
                .as_deref(),
            Some("crow couldn't be updated: it came back on 0.2.2, not 0.3.0.")
        );
    }

    /// The deadline's line goes once the member turns up on the new release
    /// after all; any other line stays.
    #[test]
    fn only_the_deadlines_line_goes_when_it_turns_up() {
        let updates = MemberUpdates::default();

        let press = updates.begin("crow", crow()).unwrap();
        updates.late(
            "crow",
            press,
            "crow didn't come back after its update.".to_owned(),
        );
        assert!(updates.taken_up("crow", Some("0.3.0")));
        assert_eq!(updates.reading("crow"), (None, false, None));

        let press = updates.begin("crow", crow()).unwrap();
        updates.troubled("crow", press, "crow couldn't be updated.".to_owned());
        assert!(!updates.taken_up("crow", Some("0.3.0")));
        assert!(updates.reading("crow").2.is_some());
    }

    /// What is awaited ends with the Update.
    #[tokio::test]
    async fn settled_is_the_update_ending() {
        let updates = MemberUpdates::default();
        updates.settled("crow").await;

        let press = updates.begin("crow", crow()).unwrap();

        let waiting = {
            let updates = updates.clone();
            tokio::spawn(async move { updates.settled("crow").await })
        };

        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());

        updates.troubled("crow", press, "crow couldn't be updated.".to_owned());

        tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("the wait ends with the Update")
            .unwrap();
    }
}
