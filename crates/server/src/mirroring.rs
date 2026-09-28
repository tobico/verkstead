//! The **mirror rows**: a local Profile per member Profile this device has
//! heard of, marked with the device it is at home on and the id it has there
//! (ADR-0020, *Shared Profiles*).
//!
//! **Why a row rather than a list held in memory.** The merged sidebar holds
//! each member's Conversations in this device's memory, because none of it is
//! this device's record — see [`crate::merging`]. A Profile is the other case: a
//! pairing names one by id, a Repo remembers what it was last grilled with by
//! id, and a Conversation carries three of them. So a member's Profile is
//! written into this device's own `profiles` table with the device and the
//! remote id beside it, and every one of those readers goes on holding a
//! **local** id. Nothing that reads a Profile id changes, which is the whole of
//! why mirrors exist.
//!
//! **Fetched device to device rather than through the browser.** The whole
//! viewer namespace is served to a member over the Peer Listener behind the
//! Member Gate, so a member's `/api/ui/profiles` is readable as it stands and
//! there is no new route here. What is new is this device *asking* — the way it
//! already holds a Nudge stream to each member rather than making the browser
//! hold one.
//!
//! **When it asks** is three moments: at the start, whenever a member says its
//! Profiles moved, and whenever the membership itself moves. The second covers
//! a member that has been away — the Nudge stream taken up to one announces
//! [`Nudge::Everything`] under that device, which is *read back whatever of it
//! you hold* — and the third is a device linked while this server is up, which
//! has accounts nothing here has heard of, and one unlinked, which has mirrors
//! to take away.
//!
//! **A member that is not answering keeps the rows it last gave**, exactly as
//! its Conversations are kept: a read that could not be made changes nothing at
//! all, and the rows stay on the list with that machine drawn unreachable on
//! them. What takes a mirror away is a member *saying* it no longer holds that
//! Profile — which is what a removal at home comes to — or the device ceasing to
//! be a member of this cluster.
//!
//! **What is mirrored is the member's own rows and nothing it is mirroring.** A
//! cluster of three has A holding mirrors of C, and a list read off A carries
//! them; B taking those in would be B holding two rows for one account, or its
//! own accounts back again under A's name. So a row that says it is at home
//! somewhere is skipped here, and every mirror on this device is one hop from
//! the machine its account is on.
//!
//! **And a mirror is started under like any other row.** What a session needs of
//! an account is the little a Built Root is made from, and that is fetched from
//! the home device into a mirror of the account under this device's own Data
//! Directory before each launch — see [`account`]. So the row is what it looks
//! like: a Profile of this cluster's, with the machine its account sits on drawn
//! beside it.
//!
//! **And an edit or a removal over a mirror is put to the device it is at home
//! on.** Every device's Profiles section lists everyone's and the form over a
//! mirror saves — the human's choice over editing an account only where it lives
//! — so a save pressed here is put to the home device as the ordinary edit of
//! *its own* Profile, addressed by the id the row records for it there. What
//! comes back is that device's own answer rather than a second opinion composed
//! here, the two uniqueness rules being its rows' to hold; and the mirror then
//! redraws off the refreshed row rather than off what was typed. A removal is the
//! same hop, and takes the Profile off its home device. See [`edited`] and
//! [`removed`].

pub mod account;

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{ProfileAccount, ProfileDeleted, ProfileEdit, ProfileEntry, ProfileSaved};
use verkstead_schema::Nudge;

use crate::relaying::{self, Call, Refusal, Streamed, as_json, read_of};
use crate::{AppState, store};

/// What a member's Agent Profiles are read at, which is the path this device
/// serves its own at: the namespace is one router mounted twice, so the list a
/// member answers here and the list this device answers a browser are the same
/// endpoint seen from the two ends.
const PROFILES: &str = "/api/ui/profiles";

/// The most a member's Profiles may be before the answer is dropped rather than
/// written down: **one megabyte**.
///
/// A Profile is a name, an account's paths and a handful of model ids — some
/// hundreds of bytes — so a megabyte is thousands of accounts, which is more
/// than one person has. What it is for is the other case: a machine on the far
/// end of a relay that answers and then writes without stopping, which this
/// device makes one read of per member. The merged list's own bound, for its
/// reason — see [`crate::merging::MOST_A_LIST_IS`].
pub(crate) const MOST_THE_PROFILES_ARE: usize = 1024 * 1024;

/// And the most a press this device puts home may be answered with: **four
/// kilobytes**.
///
/// An outcome is a single word — `"Saved"`, `"NameTaken"`, `"Removed"` — so four
/// kilobytes is thousands of times what one is. What a bound is for here is the
/// same thing it is for above: a machine on the far end of a relay that answers
/// and then writes without stopping.
const MOST_AN_OUTCOME_IS: usize = 4 * 1024;

/// Keep every member's mirrors fresh, for as long as this server runs.
///
/// Spawned rather than waited on, beside the merged list's own refresher and for
/// its reason: what it does first is read every member, some of which are
/// laptops that are shut, and it never returns.
///
/// Nothing at all on a router with no identity: a Verkstead stood up without a
/// Data Directory is linked to nothing and has nobody's Profiles to mirror.
pub(crate) fn refreshing(state: &AppState) {
    if state.devices.is_none() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move { held(state).await });
}

/// The loop itself: every member read once, and then whatever a Nudge says to
/// read again.
///
/// **Subscribed before the first round of reads**, so that a Nudge landing while
/// this device is still working down its members is one it hears rather than one
/// that slips past it — the order the merged list's refresher and the Nudge
/// stream's own endpoint both take, for the same reason.
async fn held(state: AppState) {
    let mut moved = state.nudges.subscribe();

    read_every_member(&state).await;

    loop {
        match moved.recv().await {
            Ok(moved) => match (moved.device, &moved.moved) {
                // A member saying its Profiles moved, or a stream to one taken
                // up after a machine came back.
                (Some(device), kind) if re_read(kind) => read_member(&state, &device).await,

                // And this device's own membership moving, which is the third
                // moment: a device linked while this server is up has Profiles
                // to mirror, and one unlinked has mirrors to take away. Both are
                // one pass over the membership.
                (None, Nudge::Devices) => read_every_member(&state).await,

                // Everything else this device announces is about its own work,
                // and its own Profiles are rows in this store already.
                _ => {}
            },

            // Fallen behind, which is a burst this device saw the middle of
            // none of: what it missed is unknowable, so every member is read
            // again. The same reaction the merged list makes of the same thing.
            Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                tracing::debug!(
                    missed,
                    "the mirrors fell behind the Nudges, so every member's Profiles are read again",
                );
                read_every_member(&state).await;
            }

            // Nobody is announcing any more, which is a server that is going
            // away: there is nothing left for a list to be drawn on.
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// Whether a Nudge is one the mirrors are read again on.
///
/// Two kinds. [`Nudge::Profiles`] is a member saying its accounts moved — which
/// is what its own `POST /api/ui/profiles` announces, and the reason that kind
/// existed before anything sent it. [`Nudge::Everything`] is the stream to that
/// member having been taken up, which is *read back whatever of it you hold*:
/// the member may have been away for a week, and the Profiles it came back with
/// are not the ones this device wrote down.
fn re_read(moved: &Nudge) -> bool {
    matches!(moved, Nudge::Profiles | Nudge::Everything)
}

/// Read every member's Profiles, one after another — and take away the mirrors
/// of every device that is not one.
///
/// Sequential rather than at once, for the merged list's reason: nothing is
/// drawn behind this, and what it buys is one relay in flight from this device
/// at a time.
async fn read_every_member(state: &AppState) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let members = match devices.membership().rows().await {
        Ok(members) => members,
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no member's Profiles \
                 were read and the mirrors stand as they are",
            );
            return;
        }
    };

    pruned(
        state,
        &members
            .iter()
            .map(|member| member.device.clone())
            .collect::<Vec<_>>(),
    )
    .await;

    for member in members {
        read_member(state, &member.device).await;
    }
}

/// Take away the mirrors of every device that is no longer a member of this
/// cluster.
///
/// **The membership is what prunes these**, which is the stance the merged list
/// takes about the rows it holds of a departed member: a device that has been
/// unlinked is not one whose accounts this device has any business offering, and
/// nothing over there is ever going to say so. A member that is merely not
/// answering is not this — its rows are as much its own as they were yesterday.
///
/// The pages are told where anything went, because a Profile leaving is a list
/// that reads differently everywhere it is drawn.
async fn pruned(state: &AppState, members: &[String]) {
    match store::forget_mirrors_of_departed(&state.pool, members).await {
        Ok(gone) if gone.is_empty() => {}

        Ok(gone) => {
            tracing::info!(
                profiles = gone.len(),
                "the Profiles of devices this one is no longer linked to are gone",
            );

            state.nudges.announce_here(Nudge::Profiles);
        }

        Err(error) => tracing::error!(
            error = ?error,
            "the Profiles of devices this one is no longer linked to could not be taken away",
        ),
    }
}

/// Read one member's Profiles and write down what it said.
///
/// **A read that could not be made leaves every mirror exactly as it is.** A
/// member that is switched off, a link that is down, a machine that has moved:
/// its Profiles stay on this device's list from the last time it answered, with
/// that machine drawn unreachable on the row. Which is the whole of why they are
/// written down rather than fetched per load — there is something to draw of a
/// laptop whose lid is shut, and a pairing made against one of its accounts
/// still names a row.
///
/// **And what it said is written whole**: every Profile upserted onto the row it
/// already had, and the mirrors of that member it did *not* name taken away. A
/// Profile removed at home is exactly the second of those.
async fn read_member(state: &AppState, device: &str) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let Some(said) = read_of(
        devices,
        device,
        PROFILES.to_owned(),
        "Agent Profiles",
        MOST_THE_PROFILES_ARE,
    )
    .await
    else {
        return;
    };

    let rows = match serde_json::from_slice::<Vec<ProfileEntry>>(&said) {
        Ok(rows) => rows,

        // A member running a Verkstead this one cannot read the Profiles of.
        // What it last said stands, which is the stance a read that never landed
        // takes: this device has no better account of that machine's accounts
        // than the one it already wrote down.
        Err(why) => {
            tracing::warn!(
                device,
                "a member answered its Agent Profiles in a way this device cannot read, so the \
                 mirrors of it stand: {why}",
            );
            return;
        }
    };

    // Its own rows and nothing it is itself mirroring — see this module's
    // documentation. Read before anything is written, so that a list that is all
    // somebody else's mirrors prunes this device's rows for that member rather
    // than being taken as *it answered nothing*.
    let its_own: Vec<&ProfileEntry> = rows.iter().filter(|row| row.device.is_none()).collect();

    let mut wrote = 0;

    for row in &its_own {
        let at = store::Mirror {
            device: device.to_owned(),
            id: row.id,
        };

        match store::record_mirror(&state.pool, &at, &facts(row)).await {
            Ok(_) => wrote += 1,

            // One Profile that could not be written down is one row missing from
            // the list, and the rest of what the member said is still worth
            // having: the next refresh writes it again.
            Err(error) => tracing::error!(
                error = ?error,
                device,
                profile = row.id,
                "a member's Agent Profile could not be written down",
            ),
        }
    }

    let held: Vec<i64> = its_own.iter().map(|row| row.id).collect();

    let gone = match store::forget_mirrors_except(&state.pool, device, &held).await {
        Ok(gone) => gone,
        Err(error) => {
            tracing::error!(
                error = ?error,
                device,
                "the Profiles a member no longer holds could not be taken away",
            );
            Vec::new()
        }
    };

    // And the pages are told, this being the moment there is something new for
    // them to read — said the way the merged list says it after a list of a
    // member's has landed, and kept to this device's own pages for that reason:
    // it is about the rows this device holds rather than about anybody's
    // accounts moving, and a member sent it would read it as *this device's
    // Profiles moved* and come back to read them, round for ever. See
    // [`crate::nudge::Nudges::announce_here`].
    if wrote > 0 || !gone.is_empty() {
        state.nudges.announce_here(Nudge::Profiles);
    }
}

/// Where a press over one Profile row is made, and what came of it where that is
/// not here.
///
/// Three answers rather than two, because *this row is one of this device's own*
/// is not an outcome of anything: it is the finding that there was nothing to
/// relay, and the press goes on to the half of the endpoint that was always
/// there.
pub(crate) enum Pressed<T> {
    /// One of this device's own rows. Nothing was put anywhere, and the press is
    /// this device's own to make.
    Here,

    /// A mirror, and the device it is at home on answered this — in that
    /// device's own vocabulary, which is the vocabulary a local press answers
    /// in.
    Away(T),

    /// A mirror whose home did not take it, named as the human named the
    /// machine. Nothing here has moved.
    Refused(Refusal),
}

/// Put an edit over a Profile row to the device it is at home on, or say that
/// the row is this device's own.
///
/// **The browser's own fields, put to the far end as the edit of its own Profile
/// it is.** The form over a mirror is the form over any other Profile — every
/// device's Profiles section lists everyone's, which is the human's choice over
/// editing an account only where it lives — and what crosses the link is what
/// was typed, addressed by the id the mirror records for that Profile at home.
/// Nothing here judges any of it: the paths belong to the home machine's
/// filesystem, and the two uniqueness rules are that device's own rows' (see the
/// `profiles` table's partial indexes). So the answer is that device's own — a
/// name already taken there comes back as the word its own store refused with,
/// said in the words a local clash is said in.
///
/// **And the mirror redraws off the refreshed row rather than off what was
/// typed**: a save that landed is followed by the ordinary read of that member,
/// so what this device holds is what the home device now says rather than what
/// the form sent it. A save that was refused there leaves every row alone.
///
/// **Nothing is relayed twice.** The row names one device and the press goes to
/// it and nowhere else; a third device of the cluster learns of the change by
/// refreshing its own mirror, which is the rule every announcement here is held
/// under. And the hop cannot chain: a mirror is only ever written from a
/// member's *own* rows — see this module's documentation — so the id this
/// addresses is a row of that device's own, and the same call arriving there
/// answers [`Pressed::Here`].
pub(crate) async fn edited(
    state: &AppState,
    id: i64,
    edit: &ProfileEdit,
) -> Result<Pressed<ProfileSaved>> {
    let Some(at) = home_of(&state.pool, id).await? else {
        return Ok(Pressed::Here);
    };

    let call = Call {
        method: reqwest::Method::POST,
        onwards: format!("{PROFILES}/{}", at.id),
        headers: as_json(),
        body: Streamed::saying(serde_json::to_vec(edit)?),
    };

    Ok(
        match relaying::word_from::<ProfileSaved>(
            state.devices.as_ref(),
            &at.device,
            call,
            MOST_AN_OUTCOME_IS,
        )
        .await
        {
            Ok(said) => {
                if said == ProfileSaved::Saved {
                    redrawn(state, &at.device).await;
                }

                Pressed::Away(said)
            }
            Err(why) => Pressed::Refused(why),
        },
    )
}

/// And a removal, which is the same hop: it takes the Profile off its home
/// device.
///
/// What follows from there is what follows from any removal at home — every
/// device's mirror of it goes on that device's next refresh, and each nulls the
/// Pairings that named it, so a Profile removed from B leaves a Conversation on C
/// reading as one nothing has been picked for. This device's own mirror is read
/// again at once rather than waiting for the news to come back round.
pub(crate) async fn removed(state: &AppState, id: i64) -> Result<Pressed<ProfileDeleted>> {
    let Some(at) = home_of(&state.pool, id).await? else {
        return Ok(Pressed::Here);
    };

    let call = Call {
        method: reqwest::Method::POST,
        onwards: format!("{PROFILES}/{}/delete", at.id),
        headers: as_json(),
        body: Streamed::Nothing,
    };

    Ok(
        match relaying::word_from::<ProfileDeleted>(
            state.devices.as_ref(),
            &at.device,
            call,
            MOST_AN_OUTCOME_IS,
        )
        .await
        {
            Ok(said) => {
                if said == ProfileDeleted::Removed {
                    redrawn(state, &at.device).await;
                }

                Pressed::Away(said)
            }
            Err(why) => Pressed::Refused(why),
        },
    )
}

/// Read that member again, now that the press has moved something over there.
///
/// The same read the refresher makes, made here so that the list the human is
/// looking at redraws off the saved row rather than a round trip later: the home
/// device announces its own Profiles moved and this device would hear it and read
/// them anyway, and doing it now is that read brought forward. Idempotent for
/// exactly that reason — the news arriving a moment later finds the rows it would
/// have written already written.
async fn redrawn(state: &AppState, device: &str) {
    read_member(state, device).await;
}

/// Which device a Profile row is at home on and what it is numbered there — or
/// `None` for one of this device's own, which is every row on a Verkstead that is
/// linked to nothing.
///
/// **A row that is not there at all reads as this device's own**, so the press
/// falls through to the local half and is refused in the words a press over a
/// Profile that has gone is always refused in. A second sentence for one finding
/// would be two ways of saying *that profile is gone*.
async fn home_of(pool: &SqlitePool, id: i64) -> Result<Option<store::Mirror>> {
    Ok(store::load_profile(pool, id)
        .await?
        .and_then(|profile| profile.mirror))
}

/// One row a member answered, as this device writes a Profile down.
///
/// What a row is drawn and picked by and no account of this device's own: the
/// name, the models, the memory switch and the account in the shape its harness
/// keeps one. The account's paths are the home machine's and belong to no
/// filesystem here — they are kept because they are what that Profile *is*, and
/// what a session away from home is actually given is another stage's.
///
/// Nothing is resolved and nothing is looked at, which is what tells this apart
/// from the same conversion made of a form: the paths came resolved off the
/// machine they mean something on, and resolving them again here would be asking
/// the wrong filesystem.
fn facts(row: &ProfileEntry) -> store::ProfileFacts {
    store::ProfileFacts {
        name: row.name.clone(),
        account: told(&row.account),
        models: row.models.clone(),
        memory: row.memory,
    }
}

/// And the account inside it, as the store holds one.
///
/// [`crate::profiles::account`] read the other way, arm for arm — the one
/// direction that was missing, every account having travelled outwards until
/// now.
fn told(account: &ProfileAccount) -> store::Account {
    match account {
        ProfileAccount::Claude {
            claude_dir,
            config_file,
        } => store::Account::Claude {
            claude_dir: claude_dir.into(),
            config_file: config_file.into(),
        },
        ProfileAccount::Codex { home } => store::Account::Codex { home: home.into() },
        ProfileAccount::Grok { home } => store::Account::Grok { home: home.into() },
        ProfileAccount::OpenCode { home } => store::Account::OpenCode { home: home.into() },
    }
}
