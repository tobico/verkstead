//! The cross-device drag: one **Rank** minted on the device the browser opened,
//! and written to the device that owns the row that moved (ADR-0020, *Ranks*).
//!
//! **The hub mints, because the hub is the only machine that can.** A drag on
//! the merged sidebar lands a row between two neighbours that may belong to any
//! device in the cluster, and the sentence stage 05 saved a drag with — *this
//! row, under that one*, by an id — cannot cross a device boundary: ids are
//! numbered per device, so the `7` a hub means is not the `7` the far end would
//! find. What does cross intact is the key itself. So the hub reads both
//! neighbours off the merged list it already holds — its own ranks out of its
//! store and each member's out of the list held of it, see [`crate::merging`] —
//! mints between them, and tells the owning device its row's new rank.
//!
//! **One route, and the local case is not a second one.** What the owner is told
//! is `PUT /api/ui/conversations/{id}/rank`, which is reached on a member through
//! the Relay at `/api/ui/members/{device}/…` like every other call and reached on
//! this device by [`stated`], which is the very function that route's handler
//! calls. So there is one way a rank is written and one place a Nudge goes out
//! about one.
//!
//! **The suffix is the owner's**, whoever the neighbours belong to: a rank
//! carries the device that issued it, and the device that issued this one is the
//! device the row is going to live on. That is what keeps every rank in the
//! cluster distinct, which is what makes the merged order total.
//!
//! **And the mint is serialised here.** Stage 05 read both neighbours and wrote
//! inside one transaction, so two drops into one gap a moment apart could not
//! read the same pair and mint the same key. Minting off a held merge gives that
//! transaction up, so this module holds the two apart itself — see [`Minting`] —
//! and puts the rank it has just written onto the held list at once, that list
//! being a Nudge's round trip behind the member it describes.
//!
//! What no lock reaches is a *start* racing a drop: a Conversation started on a
//! member while this device is minting for that member's list is a second key
//! computed over there, off the same top row, with the same suffix. Two rows at
//! one rank is what that costs, and the merge draws them in a stated order all
//! the same — the sort is stable — while a later drag between the two of them is
//! refused the way any pair at one key is until one of them is re-ranked. It is
//! the price of serving the merge from memory, and it is a great deal smaller
//! than a round trip per row of the sidebar.
//!
//! **Two rows at one key is the exception, and the gap is opened first.** Two
//! devices each ranking above their own top mint the *same* key — the first rows
//! of two devices rather than a rare case, and they sit at the top of the list
//! where cards are dropped most. The two sort apart, which is what the merge
//! needs of them, but no rank sits between them: every rank at that key reads
//! `key-<device>`, and the row being moved carries its own. So `ranks::between`
//! refuses, and this module re-ranks the *lower* of the pair through its own
//! device first — a key between the pair's shared one and whatever is under that
//! row — and then mints the dropped row into the gap that opened. Two devices
//! are written to in that one case and one in every other, the drop lands where
//! the human put it, and it does not recur at that spot: the pair no longer share
//! a key. Where the lower row's own device cannot be reached the gap cannot be
//! opened, so the drag is refused and the sidebar says which device it was.

use std::sync::Arc;

use tokio::sync::Mutex;
use verkstead_render::{DroppedRow, MergedRow, NewRank};
use verkstead_schema::Nudge;
use verkstead_store::ranks;

use crate::merging::{self, Ranked};
use crate::relaying::{self, Call, Refusal, Streamed};
use crate::{AppState, store};

/// What holds two drops apart on this device.
///
/// A mint is *read the merged list, compute a key, write it* and nothing in the
/// middle of that is atomic any more — the reading is a lock over a table in
/// memory and the writing may be a dial across the room. Two drops into one gap
/// in quick succession are the ordinary case, so they take this in turn.
///
/// An async mutex rather than a plain one, because the whole of a mint is held
/// across it and a relayed write is an await. Nothing else takes it, and a drag
/// is a human's hand: the contention is two presses, not two hundred.
#[derive(Debug, Clone, Default)]
pub(crate) struct Minting(Arc<Mutex<()>>);

impl Minting {
    /// One of these per server, which is one per merged list.
    pub(crate) fn new() -> Minting {
        Minting::default()
    }
}

/// Where the human has just dropped one row of the merged list: mint its rank
/// and write it to the device that owns it.
///
/// **A neighbour that has gone since the list was drawn is not a refusal.** A
/// viewer sends the list it drew and a Conversation can be closed and swept from
/// under it, so there is nothing left to rank against and the order stays as the
/// rest of the list says. Nothing is written and the call is taken, which is the
/// stance the route it replaced took for the same reason.
///
/// **What a refusal says is the human's sentence rather than a log line's**, and
/// it is written as the tail of the line the pane already draws — *The order
/// could not be saved: …* — so a member that could not be reached is named in it
/// by the name the human gave the machine. See [`crate::relaying::Refusal`],
/// which is the shape every press this device puts to a member is refused in.
pub(crate) async fn dropped(state: &AppState, moved: DroppedRow) -> Result<(), Refusal> {
    // Taken for the whole of the mint — the read, the arithmetic and the write
    // — rather than for the write alone: what two drops must not share is the
    // pair of neighbours they read.
    let _minting = state.minting.0.lock().await;

    let rows = merging::ranks(state)
        .await
        .map_err(|why| Refusal::ours(format!("the sidebar's order could not be read: {why:#}")))?;

    let moved = DroppedRow {
        row: here(state, moved.row),
        below: moved.below.map(|below| here(state, below)),
    };

    let above = match &moved.below {
        None => None,
        Some(below) => match rank_of(&rows, below) {
            Some(rank) => Some(rank),

            // Overtaken — see this function's own documentation.
            None => return Ok(()),
        },
    };

    let under = under_the_gap(&rows, above.as_deref(), &moved.row);

    let rank = match (above.as_deref(), under.as_deref()) {
        // The one pair with nothing between them, which is the first rows of two
        // devices: the lower of the two moves down first, and the dropped row
        // goes into what that opened.
        (Some(above), Some(under)) if ranks::at_one_key(above, under) => {
            let opened = open_the_gap(state, &rows, under, &moved.row).await?;

            mint(state, Some(above), Some(&opened), &moved.row)?
        }

        (above, under) => mint(state, above, under, &moved.row)?,
    };

    write(state, &moved.row, &rank).await
}

/// Move the lower of a pair at one key down, so that something can sit between
/// the two of them.
///
/// `under` is that lower row's rank, and what it is given is a key strictly
/// between the pair's shared one and whatever is under *it* — suffixed with its
/// own device, because the rank a row carries is always its own device's. What
/// comes back is that new rank, which is the bottom of the gap the caller then
/// mints into.
///
/// **The row under it is the next row of the merged list**, the moved row left
/// out. Where that row is at the shared key too — three devices' first rows,
/// which takes a third machine and a drop between the first two of them — there
/// is no gap to open either, and the refusal says so rather than shuffling a row
/// past a sibling the human can see.
async fn open_the_gap(
    state: &AppState,
    rows: &[Ranked],
    under: &str,
    moved: &MergedRow,
) -> Result<String, Refusal> {
    let lower = rows
        .iter()
        .find(|row| row.rank == under)
        .expect("the rank under the gap was read off this very list");

    let next = under_the_gap(rows, Some(under), moved);

    let opened = ranks::between(Some(under), next.as_deref(), owner(state, &lower.device))
        .map_err(|why| Refusal::ours(format!("the gap could not be opened: {why:#}")))?;

    write(
        state,
        &MergedRow {
            device: lower.device.clone(),
            id: lower.id,
        },
        &opened,
    )
    .await?;

    Ok(opened)
}

/// The rank strictly between two, carrying the device that owns the row that
/// moved.
fn mint(
    state: &AppState,
    above: Option<&str>,
    under: Option<&str>,
    moved: &MergedRow,
) -> Result<String, Refusal> {
    ranks::between(above, under, owner(state, &moved.device)).map_err(|why| {
        Refusal::ours(format!(
            "the row's new place could not be worked out: {why:#}"
        ))
    })
}

/// Write one row's rank, on whichever device owns it.
///
/// The one place a rank is written: [`stated`] for this device's own rows and
/// the same route over the Relay for a member's — see this module's own
/// documentation.
async fn write(state: &AppState, row: &MergedRow, rank: &str) -> Result<(), Refusal> {
    match row.device.as_deref() {
        None => stated(state, row.id, rank)
            .await
            .map_err(|why| Refusal::ours(format!("the order could not be saved: {why:#}"))),

        Some(device) => relayed(state, device, row.id, rank).await,
    }
}

/// Write one of this device's own rows, and say so.
///
/// **Both halves of what that route's handler does**, because it is what that
/// handler calls: the row is written and a Nudge goes out about the list, which
/// is what every open sidebar reads it back on.
pub(crate) async fn stated(state: &AppState, id: i64, rank: &str) -> anyhow::Result<()> {
    store::rank_conversation(&state.pool, id, rank).await?;

    state.nudges.announce(Nudge::Conversations);

    Ok(())
}

/// And write a member's row, over the link this device holds to it.
///
/// The same route on the far end, reached the way every other press this device
/// makes of its own accord is — see [`crate::relaying::put_to`], which is where
/// the four findings of a dial become the one sentence the sidebar draws. What
/// comes back is a status and nothing else, so a success is the whole of the
/// answer.
///
/// **The held list is told at once.** The member will say the new rank itself on
/// its next Nudge, which is a round trip away — and the next drop is minted off
/// what is held right now. See [`crate::merging::MemberLists::ranked`].
async fn relayed(state: &AppState, device: &str, id: i64, rank: &str) -> Result<(), Refusal> {
    let saying = serde_json::to_vec(&NewRank {
        rank: rank.to_owned(),
    })
    .map_err(|why| Refusal::ours(format!("the rank could not be written down: {why}")))?;

    relaying::put_to(
        state.devices.as_ref(),
        device,
        Call {
            method: reqwest::Method::PUT,
            onwards: format!("/api/ui/conversations/{id}/rank"),
            headers: relaying::as_json(),
            body: Streamed::saying(saying),
        },
    )
    .await?;

    state.merged.ranked(device, id, rank);
    state.nudges.announce(Nudge::Conversations);

    Ok(())
}

/// A row as this device names one: nothing at all where it is this device's own,
/// whatever the caller wrote.
///
/// Local URLs keep their shape, so a page has no id for the device it opened and
/// writes `null` — but a page walking a merged list has that id on every row of
/// its own device too, and the two spellings are one row. Said once here rather
/// than guarded for at each of the four places a device is compared.
fn here(state: &AppState, row: MergedRow) -> MergedRow {
    MergedRow {
        device: row.device.filter(|device| device != &state.device),
        id: row.id,
    }
}

/// Which device a rank is to carry: the one that owns the row, this device's own
/// id standing for a row of its own.
fn owner<'a>(state: &'a AppState, device: &'a Option<String>) -> &'a str {
    device.as_deref().unwrap_or(&state.device)
}

/// The rank of one named row, or nothing where it is not on the merged list any
/// more.
fn rank_of(rows: &[Ranked], row: &MergedRow) -> Option<String> {
    rows.iter()
        .find(|held| held.id == row.id && held.device == row.device)
        .map(|held| held.rank.clone())
}

/// And the rank under a gap: the lowest rank on the merged list that sorts above
/// `above`, with the row that is moving left out.
///
/// **The moved row is left out** because it may be sitting in that gap already —
/// a card put back roughly where it came from — and a row ranked between its
/// neighbour and itself would have moved nowhere at all.
///
/// `None` above is the top of the list, where the gap's underside is simply the
/// first rank there is; `None` back is the foot of it, where there is nothing
/// under the gap at all.
fn under_the_gap(rows: &[Ranked], above: Option<&str>, moved: &MergedRow) -> Option<String> {
    rows.iter()
        .filter(|row| !(row.id == moved.id && row.device == moved.device))
        .filter(|row| above.is_none_or(|above| row.rank.as_str() > above))
        .map(|row| row.rank.clone())
        .min()
}
