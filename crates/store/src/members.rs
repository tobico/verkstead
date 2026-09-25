//! The devices this one is linked to: the cluster, as rows (ADR-0020, *A
//! cluster is a membership*).
//!
//! **A membership rather than a set of pairs.** One row per device this one is
//! in a cluster with, and every member holds the same set — a join writes the
//! newcomer on every one of them, rather than a link being a thing two machines
//! hold between themselves. So what is kept here is what a device *is* to the
//! others: the id every record in a cluster names it by, what it is shown as,
//! where to reach it, the certificate that proves it, and when it was last
//! heard from.
//!
//! **The Device Id is the key**, because that is what a record in a cluster
//! names a device by and what survives a certificate being renewed: a device
//! keeps its id for as long as its Data Directory lasts, and the announcement a
//! renewal makes records a new fingerprint against the same row.
//!
//! **The fingerprint is beside it because that is what a caller *is*.** On the
//! peer listener there is no bearer token and no name to go by — a caller is
//! the certificate the handshake took from it, and the membership is asked
//! whether it holds that fingerprint. Which is why it is spelled here exactly
//! as `crates/server/src/device.rs` spells one: the comparison is a string
//! comparison, and two spellings of one certificate would be two devices.
//!
//! **The addresses are a list and the order is load-bearing.** A device
//! advertises all of them on every exchange — the tailnet name and its
//! addresses first, then the LAN — and a peer dials them in that order, so a
//! machine that moved is still reached on a later one. A table of its own
//! keyed by position, which is the shape `profile_models` is and for the same
//! reason: a list that came back in some other order would be a peer trying the
//! LAN before the tailnet, which is the half that crosses.
//!
//! **Last seen is written by the recording**, because a member is recorded off
//! an exchange that has just got through: there is no way to learn a device's
//! certificate but to have met it. What moves it afterwards is a dial that
//! answered.
//!
//! **And whether it answered at all is a column beside it.** A dial that found
//! nothing at any of a member's addresses marks it on that first failure, and
//! the row is drawn dimmed, reading *unreachable*, from then until a dial gets
//! through. A mark rather than a reading of how old `last_seen` is, because the
//! human settled that over a grace period: a laptop with its lid shut is a
//! machine that is not there, whatever o'clock it stopped being there at.
//! Nothing else about the row moves — it is not removed, and its addresses and
//! its fingerprint are exactly what they were, which is what the next dial
//! works down.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::writing;

/// One device this one is linked to, as the table holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The Device Id: what every record and URL in a cluster names it by, and
    /// what a renewed certificate is recorded against.
    ///
    /// Spelled `device` rather than `id` for the reason the wire type spells it
    /// that way: an `id` on a row about a device could be a Conversation's or a
    /// Repo's.
    pub device: String,

    /// What it is shown under, which is the hostname of the machine it is on as
    /// that machine last said it. Nothing here is configured — a name somebody
    /// could set would be a name two devices could be given.
    pub name: String,

    /// And the word for its operating system, which is what draws the mark
    /// beside the name. *Linux (WSL)* is the one word that is not a bare
    /// platform, and it is the whole reason this is kept: a Windows machine and
    /// the WSL on it share a hostname.
    pub os: String,

    /// Every address it advertised, in the order it advertised them — which is
    /// the order a peer dials them in.
    pub addresses: Vec<String>,

    /// The fingerprint of the certificate it presents, which is what proves it
    /// on the peer listener: a member *is* a fingerprint, there being no
    /// authority anywhere in a cluster to check a chain against.
    pub fingerprint: String,

    /// And when this device last heard from it, RFC 3339. Written by the
    /// recording, because a member is recorded off an exchange that has just
    /// got through.
    pub last_seen: String,

    /// And whether the last dial to it got through: false is the row the pane
    /// draws dimmed, reading *unreachable*.
    ///
    /// True as a member is recorded, because a recording is an exchange that
    /// has just got through — see [`record_member`], which is also what puts a
    /// dimmed row back. What clears it is [`member_unreachable`], on the first
    /// dial that answers at none of the addresses above.
    pub reachable: bool,
}

/// A device about to be written down as a member: everything it said about
/// itself.
///
/// What it does *not* carry is the moment, which is this device's own reading
/// rather than anything the far end said — see [`record_member`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linking {
    pub device: String,
    pub name: String,
    pub os: String,
    pub addresses: Vec<String>,
    pub fingerprint: String,
}

/// The members table, and the addresses hanging off it.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS members (
             device      TEXT PRIMARY KEY,
             name        TEXT NOT NULL,
             os          TEXT NOT NULL,
             fingerprint TEXT NOT NULL,
             last_seen   TEXT NOT NULL,
             reachable   INTEGER NOT NULL DEFAULT 1
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the members table")?;

    // And whether the last dial got through, through `ALTER TABLE` as well as in
    // the declaration above — [`super::companions::apply_schema`]'s rule, for
    // its reason: a database made this morning and one written before the mark
    // existed take the same path and end with the same shape.
    //
    // Arriving true, which is what was true of every row before it: a member
    // recorded by an exchange and never dialled since is a member nothing has
    // found out anything bad about.
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('members') WHERE name = ?")
            .bind("reachable")
            .fetch_optional(pool)
            .await
            .context("looking for the column that says whether a member is answering")?;

    if there.is_none() {
        sqlx::query("ALTER TABLE members ADD COLUMN reachable INTEGER NOT NULL DEFAULT 1")
            .execute(pool)
            .await
            .context("adding the column that says whether a member is answering")?;
    }

    // The gate asks one question of this table on every call a member makes —
    // is there a member holding this fingerprint — so the column it asks by is
    // the one column worth an index of its own.
    sqlx::query("CREATE INDEX IF NOT EXISTS members_by_fingerprint ON members (fingerprint)")
        .execute(pool)
        .await
        .context("creating the index the member gate reads by")?;

    // And the addresses, one row apiece with the position they were advertised
    // at. A table rather than a column for the reason `profile_models` is one,
    // and `position` because the order is what a dial works down: the tailnet
    // half first, because it is the half that crosses.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS member_addresses (
             device   TEXT NOT NULL REFERENCES members(device),
             position INTEGER NOT NULL,
             address  TEXT NOT NULL,
             PRIMARY KEY (device, position)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the member addresses table")?;

    Ok(())
}

/// Write a device down as a member, replacing whatever was recorded for that
/// id before.
///
/// **Keyed by the id rather than by the certificate**, so a device that renewed
/// or moved is the same member said again rather than a second row: the id
/// outlives both the fingerprint and every address.
///
/// The addresses are written afresh each time — they are a list and the list is
/// what the far end just advertised, so what was there is replaced rather than
/// merged. In one transaction with the row, because a member whose addresses
/// are half the old ones and half the new is a member a dial would work down in
/// an order nobody advertised.
///
/// And the moment is this device's own reading rather than anything the far end
/// said: a member is recorded off an exchange that has just got through, and
/// there is no way to learn a device's certificate but to have met it. Which is
/// why this is also what puts a dimmed row back: the exchange behind it *is*
/// the member answering, so a row that was marked unreachable is reachable
/// again by the same fact that moves the moment.
pub async fn record_member(pool: &SqlitePool, linking: &Linking) -> Result<()> {
    let mut tx = writing(pool, "recording a member").await?;

    sqlx::query(
        "INSERT INTO members (device, name, os, fingerprint, last_seen, reachable)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
         ON CONFLICT (device) DO UPDATE
           SET name        = excluded.name,
               os          = excluded.os,
               fingerprint = excluded.fingerprint,
               last_seen   = excluded.last_seen,
               reachable   = 1",
    )
    .bind(&linking.device)
    .bind(&linking.name)
    .bind(&linking.os)
    .bind(&linking.fingerprint)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording device {} as a member", linking.device))?;

    sqlx::query("DELETE FROM member_addresses WHERE device = ?")
        .bind(&linking.device)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "clearing the addresses device {} was last advertising",
                linking.device,
            )
        })?;

    for (position, address) in linking.addresses.iter().enumerate() {
        sqlx::query("INSERT INTO member_addresses (device, position, address) VALUES (?, ?, ?)")
            .bind(&linking.device)
            .bind(position as i64)
            .bind(address)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "recording the addresses device {} advertised",
                    linking.device
                )
            })?;
    }

    tx.commit()
        .await
        .with_context(|| format!("recording device {} as a member", linking.device))
}

/// Every member, with the addresses each advertised in the order it advertised
/// them.
///
/// Ordered by name and then by id, which is what the Devices section draws: a
/// list that reordered itself between two reads would be rows moving under
/// somebody's hand, and the id is what settles two machines that answer to one
/// hostname — a Windows and the WSL on it, which is the case the whole of
/// cluster mode was written for.
pub async fn members(pool: &SqlitePool) -> Result<Vec<Member>> {
    let rows: Vec<(String, String, String, String, String, bool)> = sqlx::query_as(
        "SELECT device, name, os, fingerprint, last_seen, reachable
         FROM members
         ORDER BY name, device",
    )
    .fetch_all(pool)
    .await
    .context("listing the devices this one is linked to")?;

    // The whole of the little table at once rather than a query per member, the
    // way the Agent Profiles read their models: a cluster is a handful of
    // machines, and one hop is the shape of the answer.
    let advertised: Vec<(String, String)> =
        sqlx::query_as("SELECT device, address FROM member_addresses ORDER BY device, position")
            .fetch_all(pool)
            .await
            .context("listing the addresses those devices advertised")?;

    let mut addresses: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (device, address) in advertised {
        addresses.entry(device).or_default().push(address);
    }

    Ok(rows
        .into_iter()
        .map(
            |(device, name, os, fingerprint, last_seen, reachable)| Member {
                addresses: addresses.remove(&device).unwrap_or_default(),
                device,
                name,
                os,
                fingerprint,
                last_seen,
                reachable,
            },
        )
        .collect())
}

/// Whether any member's certificate has this fingerprint, which is the one
/// question the member gate asks.
///
/// The fingerprint rather than the id, because on the peer listener a caller
/// *is* the certificate it presented: an id is a string in a payload and
/// anybody may write one.
///
/// Asked of the table at every call rather than of anything held, so a member
/// taken out of it is refused on the next one — a membership that was cached
/// would be a device that went on being admitted after it was unlinked.
pub async fn member_holding(pool: &SqlitePool, fingerprint: &str) -> Result<bool> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT device FROM members WHERE fingerprint = ? LIMIT 1")
            .bind(fingerprint)
            .fetch_optional(pool)
            .await
            .context("asking whether a caller's certificate is a member's")?;

    Ok(row.is_some())
}

/// How many members there are.
///
/// Counted rather than read as a list where a list is not what is wanted — the
/// changeover asks how many are owed an announcement, and a cluster's worth of
/// names and addresses is not the answer to that.
pub async fn member_count(pool: &SqlitePool) -> Result<usize> {
    let (counted,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM members")
        .fetch_one(pool)
        .await
        .context("counting the devices this one is linked to")?;

    Ok(counted as usize)
}

/// Mark a member as answering nothing, which is what a dial that found it at
/// none of its addresses does.
///
/// **The row is left exactly as it stands otherwise.** Nothing is forgotten and
/// nothing is removed: the name, the OS, the addresses and the fingerprint are
/// what was last true of that device, the moment it was last heard from is when
/// it really was, and the whole of what this writes is that the last dial did
/// not get through. The Devices list goes on drawing it, dimmed, and an unlink
/// still works on it.
///
/// **On the first failure**, because that is what the mark means — the human
/// chose this over a grace period, a row that dims the moment a machine stops
/// answering being a row that tells the truth about what a press on it would
/// do. What puts it back is [`record_member`], which is the next dial that got
/// through.
///
/// Nothing is refused. A device that is not a member is a device no dial has
/// anything to say about, and marking one that has just been unlinked is not a
/// thing to fail.
pub async fn member_unreachable(pool: &SqlitePool, device: &str) -> Result<()> {
    let mut tx = writing(pool, "marking a member unreachable").await?;

    sqlx::query("UPDATE members SET reachable = 0 WHERE device = ?")
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("marking device {device} as answering nothing"))?;

    tx.commit()
        .await
        .with_context(|| format!("marking device {device} as answering nothing"))
}

/// Take a device out of the membership, with the addresses it advertised.
///
/// The addresses first, because they point at the row: a member row dropped out
/// from under them would be rows naming a device the membership no longer
/// holds. One transaction, so there is no moment at which half a member is
/// there.
///
/// Nothing is refused. A device that is not a member is a device that is
/// already not a member, and unlinking one twice is not a thing to fail.
pub async fn forget_member(pool: &SqlitePool, device: &str) -> Result<()> {
    let mut tx = writing(pool, "forgetting a member").await?;

    sqlx::query("DELETE FROM member_addresses WHERE device = ?")
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting the addresses device {device} advertised"))?;

    sqlx::query("DELETE FROM members WHERE device = ?")
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting device {device}"))?;

    tx.commit()
        .await
        .with_context(|| format!("forgetting device {device}"))
}
