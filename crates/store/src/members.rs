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
//!
//! **And what a member has yet to be told is a table beside it.** An
//! announcement is made over a dial, and a dial to a machine that is switched
//! off reaches nobody — but the thing being announced happened anyway: a human
//! pressed Allow, and the newcomer is a member here whatever some third device
//! made of it. So the telling that did not get through is written down as owed,
//! and the one that did takes the row away. Nothing here retries in a loop; a
//! debt is paid the next time that member is found answering.
//!
//! The same table holds what a renewed certificate is owed on, the device being
//! announced then being this one rather than a newcomer — which is why what a
//! row names is the pair *member owed* and *device it has not heard about*
//! rather than anything about a join.
//!
//! **And what is owed about it is a word beside the pair**, because a cluster
//! has two opposite things to say about a device: that it is one of ours, and
//! that it is not any more. A member that was off when a newcomer joined and
//! off again when the human unlinked it is owed *the last of them* rather than
//! both — the pair is the key, so the later telling replaces the earlier, which
//! is exactly what that member would have ended up holding had it been
//! answering all along.
//!
//! **And a renewal is two columns on the member row besides**, because a
//! changeover is a state two devices are in together and each of them holds one
//! half of it.
//!
//! *What this member has said it holds of us* is [`Member::acknowledged`]: the
//! fingerprint of *this* device's own certificate that member last answered an
//! announcement of. It is what the changeover counts — a member holding anything
//! but the certificate coming in is a member the old one is still going out for
//! — and it is a positive record rather than the absence of a debt on purpose: a
//! device that joined in the middle of a changeover has no debt row and has
//! acknowledged nothing, and a count read off the debts would have let the
//! changeover finish behind its back.
//!
//! *And what this device still accepts of it* is [`Member::renewing_from`]: the
//! fingerprint this member was recorded against before it announced a renewal of
//! its own. A member in the middle of a changeover goes on presenting the
//! certificate it was presenting until the last of *its* members has
//! acknowledged, so a device that had recorded the new fingerprint and nothing
//! else would refuse every call from it — which is the one thing a changeover
//! exists not to cost. So both are accepted, at the gate and at a dial alike,
//! and the one being changed from is let go of at the moment this device meets
//! the new one.

use anyhow::{Context, Result, bail};
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

    /// And the fingerprint this member was recorded against before it announced
    /// a renewal of its own certificate, where it is in the middle of one.
    ///
    /// **Accepted beside the one above for as long as it is here**, at the gate
    /// and at a dial both. A device in the middle of a changeover goes on
    /// presenting the certificate every one of *its* members holds until the
    /// last of them has acknowledged the new one, so a member that had recorded
    /// the new fingerprint and nothing else would refuse every call it made —
    /// which is the one thing a changeover exists not to cost.
    ///
    /// `None` is the ordinary state, and it is two of them: a member that has
    /// never renewed, and one whose changeover this device has seen the end of.
    /// What ends it is meeting the new certificate, which is the only
    /// unambiguous sign that the far end has stopped presenting the old — see
    /// [`changeover_over`]. Left standing it costs nothing anyway: the
    /// certificate it names runs out on its own, and an expired one is refused
    /// at the handshake.
    pub renewing_from: Option<String>,

    /// And the other direction: the fingerprint of *this* device's own
    /// certificate that this member last said it holds.
    ///
    /// **What the changeover counts.** A device that has re-issued its
    /// certificate presents the outgoing one until every member has
    /// acknowledged the incoming one, and this is where each acknowledgement is
    /// written down — see [`renewal_acknowledged`]. A member holding anything
    /// but the certificate coming in is a member that is still owed the
    /// announcement.
    ///
    /// `None` is a member that has answered no announcement of this device's
    /// certificate, which is every member of a device that has never renewed —
    /// and a device that joined in the middle of a changeover, which is why this
    /// is a record of what *was* acknowledged rather than the absence of a debt.
    pub acknowledged: Option<String>,
}

/// What a member has yet to be told about a device.
///
/// **Two opposite things, which is why it is a word rather than a flag on a
/// row that would otherwise mean one of them.** A cluster tells a member that a
/// device is one of ours, and it tells a member that a device is not any more;
/// a debt that did not say which would be a machine coming back to whichever of
/// the two the code happened to assume.
///
/// Stored as its own spelling rather than as a number, so a row read out of the
/// database by a human says what it is — the stance every other word this store
/// keeps takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Telling {
    /// That the device named is one of this cluster's: the announcement an
    /// introducer makes the moment its human presses Allow.
    Joined,

    /// And that it is not any more: the unlink, broadcast to every member.
    Removed,

    /// And that the device this debt is owed *about* — which for this one is the
    /// device doing the telling — has made its certificate again, so the
    /// fingerprint to hold against its id has changed.
    ///
    /// The third thing a cluster has to say about a device, and the reason it is
    /// a word on this same pair rather than a record of its own: a member that
    /// was away for a week may have missed a join, an unlink and a renewal, and
    /// three records would be three things to walk when it came back.
    Renewed,
}

impl Telling {
    /// How it is written down.
    fn spelled(self) -> &'static str {
        match self {
            Telling::Joined => "joined",
            Telling::Removed => "removed",
            Telling::Renewed => "renewed",
        }
    }

    /// And read back, which is fallible because the column is a string: a
    /// database written by a newer Verkstead may hold a word this one has never
    /// heard of, and guessing at one would be a member told the opposite of
    /// what it is owed.
    fn read(said: &str) -> Result<Telling> {
        match said {
            "joined" => Ok(Telling::Joined),
            "removed" => Ok(Telling::Removed),
            "renewed" => Ok(Telling::Renewed),
            _ => bail!("{said} is not something a member can be owed"),
        }
    }
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

/// And a member that has made its certificate again, saying so: the same five
/// things, with the *two* certificates a device in the middle of a changeover
/// has.
///
/// A shape of its own rather than a [`Linking`] with a field added, because the
/// one field that matters is the one they cannot share. A linking carries the
/// certificate the device presents, which is what proves it; a renewal carries
/// that one *and* the one coming in, and which of the two is which is the whole
/// of what the call is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renewal {
    pub device: String,
    pub name: String,
    pub os: String,
    pub addresses: Vec<String>,

    /// The certificate it is still presenting, which is the one this device has
    /// recorded for it and the one the handshake carrying the announcement
    /// handed over. Kept beside the one below for as long as the changeover
    /// lasts — see [`Member::renewing_from`].
    pub presenting: String,

    /// And the one it is changing over to, which is what the row is keyed on
    /// from now on: a member *is* a fingerprint, and this is the one it will be.
    pub incoming: String,
}

/// The members table, and the addresses hanging off it.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS members (
             device        TEXT PRIMARY KEY,
             name          TEXT NOT NULL,
             os            TEXT NOT NULL,
             fingerprint   TEXT NOT NULL,
             last_seen     TEXT NOT NULL,
             reachable     INTEGER NOT NULL DEFAULT 1,
             renewing_from TEXT,
             acknowledged  TEXT
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

    // And the two halves of a changeover, by the same rule and for the same
    // reason — see [`Member::renewing_from`] and [`Member::acknowledged`].
    //
    // Both arriving null, which is what was true of every row before them: no
    // member was in the middle of a renewal of its own, and none had
    // acknowledged one of this device's. A database written by the stage that
    // made the changeover and read by this one is a device that is owed every
    // announcement it never made, which is the right answer to have arrived at.
    for column in ["renewing_from", "acknowledged"] {
        let there: Option<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('members') WHERE name = ?")
                .bind(column)
                .fetch_optional(pool)
                .await
                .with_context(|| format!("looking for the members table's {column} column"))?;

        if there.is_none() {
            // Interpolated rather than bound, because a column name is not a
            // value and `ALTER TABLE` takes no parameters. The two names are
            // written above rather than passed in from anywhere.
            sqlx::query(&format!("ALTER TABLE members ADD COLUMN {column} TEXT"))
                .execute(pool)
                .await
                .with_context(|| format!("adding the members table's {column} column"))?;
        }
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

    // And what each member has yet to be told. A table rather than a column for
    // the reason the addresses are one: a member may be owed several tellings
    // at once — three devices joined while a laptop's lid was shut — and a
    // column could hold one of them.
    //
    // Keyed by the pair, so that owing one telling twice is owing it once: a
    // row says that this member has not heard about that device, and saying it
    // again says nothing new.
    //
    // No reference to `members` from either column, although both hold a device
    // id. The device being announced *about* need not be a member of this one
    // at all — a renewal announces this machine, which is on nobody's own list
    // — and a foreign key that held for one column and not the other would read
    // as a rule somebody meant.
    //
    // And a word for *which* telling it is, because the two a cluster makes are
    // opposites — see [`Telling`]. The pair stays the key: the later telling
    // replaces the earlier, which is what a member that was away the whole time
    // would have ended up holding anyway.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS owed_announcements (
             device  TEXT NOT NULL,
             about   TEXT NOT NULL,
             telling TEXT NOT NULL DEFAULT 'joined',
             PRIMARY KEY (device, about)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the table of what members have yet to be told")?;

    // Through `ALTER TABLE` as well as in the declaration above, the rule the
    // mark on a member row takes and for its reason. Arriving *joined*, which
    // is the only thing a row written before the word existed could have been:
    // an unlink is what put the word there.
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('owed_announcements') WHERE name = ?")
            .bind("telling")
            .fetch_optional(pool)
            .await
            .context("looking for the column that says what a member is owed")?;

    if there.is_none() {
        sqlx::query(
            "ALTER TABLE owed_announcements ADD COLUMN telling TEXT NOT NULL DEFAULT 'joined'",
        )
        .execute(pool)
        .await
        .context("adding the column that says what a member is owed")?;
    }

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

/// One member row as the query hands it back, in the order the `SELECT` names the
/// columns: the device, its name, its OS, the fingerprint it is keyed on, when it
/// was last heard from, whether the last dial got through, what it is changing over
/// from, and what it has acknowledged of this device.
///
/// Named because eight columns written out at the call site is a type nobody reads
/// — and the two nullable ones at the end are exactly the pair it would be easiest
/// to read the wrong way round.
type Row = (
    String,
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
);

/// Every member, with the addresses each advertised in the order it advertised
/// them.
///
/// Ordered by name and then by id, which is what the Devices section draws: a
/// list that reordered itself between two reads would be rows moving under
/// somebody's hand, and the id is what settles two machines that answer to one
/// hostname — a Windows and the WSL on it, which is the case the whole of
/// cluster mode was written for.
pub async fn members(pool: &SqlitePool) -> Result<Vec<Member>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT device, name, os, fingerprint, last_seen, reachable, renewing_from, acknowledged
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
            |(device, name, os, fingerprint, last_seen, reachable, renewing_from, acknowledged)| {
                Member {
                    addresses: addresses.remove(&device).unwrap_or_default(),
                    device,
                    name,
                    os,
                    fingerprint,
                    last_seen,
                    reachable,
                    renewing_from,
                    acknowledged,
                }
            },
        )
        .collect())
}

/// Every member that has yet to say it holds the certificate with this
/// fingerprint, which is the question a changeover asks.
///
/// **The whole rows rather than a count**, because both things that ask this
/// want them: the announcement works down the list dialling each of them, and how
/// many there are is that list's length. Two readings of one table would be two
/// answers to *is the changeover over*.
///
/// A member that has acknowledged nothing is on the list, which is what a
/// changeover starting finds and what a device that joined during one is — see
/// [`Member::acknowledged`].
pub async fn members_yet_to_acknowledge(
    pool: &SqlitePool,
    fingerprint: &str,
) -> Result<Vec<Member>> {
    Ok(members(pool)
        .await
        .context("reading which members have yet to acknowledge a certificate")?
        .into_iter()
        .filter(|member| member.acknowledged.as_deref() != Some(fingerprint))
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
///
/// **Either of the two a member in the middle of a changeover has.** A device
/// that has re-issued its certificate goes on presenting the outgoing one until
/// the last of its own members has acknowledged the new one, so the certificate
/// this end has just recorded for it is not the certificate it is calling with —
/// see [`Member::renewing_from`]. Both get through, and the changeover costs no
/// call.
pub async fn member_holding(pool: &SqlitePool, fingerprint: &str) -> Result<bool> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT device FROM members WHERE fingerprint = ? OR renewing_from = ? LIMIT 1",
    )
    .bind(fingerprint)
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

/// Write down that a member has made its certificate again: the fingerprint to
/// hold against its id from now on, and the one it is still presenting.
///
/// **Keyed by the id, which is what a renewal is *about*.** A device keeps its
/// id for as long as its Data Directory lasts and it is the certificate that is
/// renewed, so this is one member said again rather than a member leaving and
/// another arriving — everything else in a cluster that names that device goes
/// on naming it.
///
/// **Both fingerprints are written, because both are accepted.** The incoming
/// one becomes what the row is keyed on; the one still going out is kept beside
/// it, and every call that arrives under either gets through — see
/// [`Member::renewing_from`] and [`member_holding`]. The far end switches only
/// when the last of *its* members has acknowledged, and this device is in no
/// position to know when that was.
///
/// **And the same call made twice writes the same two strings**, which it has to:
/// an announcement whose answer went missing is announced again, and a second
/// one that took the old fingerprint for a third certificate would leave a
/// member refusing the device it had just recorded.
///
/// The name, the OS and the addresses are written afresh with them, for the
/// reason [`record_member`] writes them afresh: every device advertises all of
/// them on every exchange, and this is one. So is the moment, and so is the
/// un-dimming — an announcement is an exchange that has just got through.
///
/// `false` is a renewal about a device this membership does not hold, which is
/// nothing to write and nothing to fail: the caller has something to say about
/// it and the row is the only thing that could have been changed.
pub async fn record_renewal(pool: &SqlitePool, renewal: &Renewal) -> Result<bool> {
    let mut tx = writing(pool, "recording a member's renewed certificate").await?;

    let changed = sqlx::query(
        "UPDATE members
            SET name          = ?,
                os            = ?,
                fingerprint   = ?,
                renewing_from = ?,
                last_seen     = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                reachable     = 1
          WHERE device = ?",
    )
    .bind(&renewal.name)
    .bind(&renewal.os)
    .bind(&renewal.incoming)
    .bind(&renewal.presenting)
    .bind(&renewal.device)
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "recording the certificate device {} is changing over to",
            renewal.device,
        )
    })?
    .rows_affected();

    if changed == 0 {
        return Ok(false);
    }

    sqlx::query("DELETE FROM member_addresses WHERE device = ?")
        .bind(&renewal.device)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "clearing the addresses device {} was last advertising",
                renewal.device,
            )
        })?;

    for (position, address) in renewal.addresses.iter().enumerate() {
        sqlx::query("INSERT INTO member_addresses (device, position, address) VALUES (?, ?, ?)")
            .bind(&renewal.device)
            .bind(position as i64)
            .bind(address)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "recording the addresses device {} advertised",
                    renewal.device,
                )
            })?;
    }

    tx.commit()
        .await
        .with_context(|| {
            format!(
                "recording the certificate device {} is changing over to",
                renewal.device,
            )
        })
        .map(|()| true)
}

/// And let go of the certificate a member was changing over *from*, which is
/// what meeting the new one says is safe.
///
/// **The only unambiguous sign that a changeover is over.** The far end presents
/// the outgoing certificate until the last of its own members has acknowledged,
/// and it tells nobody when that was — so what says it has stopped is a call or
/// an answer that actually came under the new certificate.
///
/// Nothing is refused, and a member that was changing over from nothing is one
/// that has already stopped. Left undone this costs nothing either: the
/// certificate named runs out on its own, and an expired one is refused at the
/// handshake.
pub async fn changeover_over(pool: &SqlitePool, device: &str) -> Result<()> {
    let mut tx = writing(pool, "letting go of a member's outgoing certificate").await?;

    sqlx::query("UPDATE members SET renewing_from = NULL WHERE device = ?")
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("letting go of what device {device} was changing over from"))?;

    tx.commit()
        .await
        .with_context(|| format!("letting go of what device {device} was changing over from"))
}

/// Write down that a member has said it holds *this* device's certificate with
/// this fingerprint.
///
/// **What takes a member off the list a changeover is waiting on** — see
/// [`members_yet_to_acknowledge`], which is the same fact read the other way
/// round. Written when an announcement of the new fingerprint was answered,
/// because the answer is the acknowledgement: nothing else says that a member
/// holds a certificate.
///
/// The fingerprint rather than a flag, so that the next renewal needs nothing
/// cleared: every member is holding the certificate before the one coming in,
/// and a row saying so is a row that is owed the announcement.
///
/// Nothing is refused. A device that is not a member has nothing to acknowledge
/// and no row to say so on.
pub async fn renewal_acknowledged(
    pool: &SqlitePool,
    device: &str,
    fingerprint: &str,
) -> Result<()> {
    let mut tx = writing(pool, "recording an acknowledged certificate").await?;

    sqlx::query("UPDATE members SET acknowledged = ? WHERE device = ?")
        .bind(fingerprint)
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("recording that device {device} holds this device's new certificate")
        })?;

    tx.commit().await.with_context(|| {
        format!("recording that device {device} holds this device's new certificate")
    })
}

/// Write down that `device` has yet to be told `telling` about `about`.
///
/// **What a failed announcement leaves behind.** The announcement itself is a
/// dial, and a dial to a machine that is off reaches nobody — but a human has
/// pressed Allow or pressed Unlink and the device being named is a member here
/// or is not, whatever that machine made of it, so the telling is owed rather
/// than lost. What pays it is the next thing that finds this member answering.
///
/// Nothing is refused and owing twice is owing once: a row says that this
/// member has not heard about that device, and a second announcement that also
/// failed says the same thing again.
///
/// **And the later telling replaces the earlier**, rather than being ignored
/// for a pair already there. A member that was off when a newcomer joined and
/// off again when the human unlinked it is owed the removal alone: it never
/// heard the join, and telling it about a device the cluster no longer holds
/// would be a row that had to be taken away again on the next call.
pub async fn owe_announcement(
    pool: &SqlitePool,
    device: &str,
    about: &str,
    telling: Telling,
) -> Result<()> {
    let mut tx = writing(pool, "writing down an announcement that was not made").await?;

    sqlx::query(
        "INSERT INTO owed_announcements (device, about, telling) VALUES (?, ?, ?)
         ON CONFLICT (device, about) DO UPDATE SET telling = excluded.telling",
    )
    .bind(device)
    .bind(about)
    .bind(telling.spelled())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("writing down that device {device} has not heard about {about}"))?;

    tx.commit()
        .await
        .with_context(|| format!("writing down that device {device} has not heard about {about}"))
}

/// And take that away, which is what an announcement that got through does.
///
/// Nothing is refused. An announcement that was never owed is one this device
/// made without anything having gone wrong first, which is the ordinary case:
/// the row is cleared either way rather than looked for.
pub async fn announcement_made(pool: &SqlitePool, device: &str, about: &str) -> Result<()> {
    let mut tx = writing(pool, "clearing an announcement that was made").await?;

    sqlx::query("DELETE FROM owed_announcements WHERE device = ? AND about = ?")
        .bind(device)
        .bind(about)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("clearing what device {device} was owed about {about}"))?;

    tx.commit()
        .await
        .with_context(|| format!("clearing what device {device} was owed about {about}"))
}

/// Every member that has yet to be told about `about`, by device id.
///
/// The devices rather than the rows, because what is done with the answer is
/// dialling each of them and a dial reads the member back for itself — and a
/// member that has since been unlinked has no row to read, so a list of ids is
/// the honest shape of a debt.
pub async fn announcements_owed(pool: &SqlitePool, about: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT device FROM owed_announcements WHERE about = ? ORDER BY device")
            .bind(about)
            .fetch_all(pool)
            .await
            .with_context(|| format!("listing the members that have not heard about {about}"))?;

    Ok(rows.into_iter().map(|(device,)| device).collect())
}

/// And the debt read the other way round: everything `device` has yet to be
/// told, as the pair *which device* and *what about it*.
///
/// **The question a dial that got through asks.** The list above is what is
/// asked while a telling is being made — who still has not heard this — and
/// this is what is asked the moment a member turns out to be answering: it is
/// back, so what has it missed. Which is how a debt is paid without anything
/// retrying in a loop; something dials a member whenever the cluster does
/// anything, and that dial is the trigger.
///
/// Ordered by the device named, so that a member coming back after a busy week
/// is caught up in one order rather than in whatever order the rows landed in.
pub async fn announcements_owed_to(
    pool: &SqlitePool,
    device: &str,
) -> Result<Vec<(String, Telling)>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT about, telling FROM owed_announcements WHERE device = ? ORDER BY about",
    )
    .bind(device)
    .fetch_all(pool)
    .await
    .with_context(|| format!("listing what device {device} has yet to be told"))?;

    rows.into_iter()
        .map(|(about, telling)| {
            Ok((
                about,
                Telling::read(&telling)
                    .with_context(|| format!("reading what device {device} is owed"))?,
            ))
        })
        .collect()
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

    // What it was owed goes with it, and so does what anybody was owed about
    // it: a device that is not a member is one nothing here has anything left
    // to tell, and a debt naming it would be a dial nobody would ever make.
    sqlx::query("DELETE FROM owed_announcements WHERE device = ? OR about = ?")
        .bind(device)
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting what device {device} was owed"))?;

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

/// And take the whole membership away, which is what a device does when it is
/// told it has been unlinked.
///
/// **The leaver's own half of an unlink.** A cluster is a membership rather
/// than a set of pairs, so a device dropped from it is dropped by everybody —
/// and the device itself is left holding a list of machines that no longer hold
/// it, every one of which would refuse it at the gate. So it is told, over the
/// link it still has while it still has it, and what it does is this.
///
/// Everything, rather than the devices named: the caller is not handing over a
/// list, and a leaver that kept whichever members the telling happened to name
/// would be a cluster of one that thought it was a cluster of two.
///
/// One transaction, and the debts first for the reason [`forget_member`] takes
/// them first: a device that is in no cluster is owed nothing and owes nothing.
///
/// Nothing is refused. A device that holds no members is one that has already
/// forgotten everybody, and being told twice is not a thing to fail.
pub async fn forget_every_member(pool: &SqlitePool) -> Result<()> {
    let mut tx = writing(pool, "forgetting every member").await?;

    for statement in [
        "DELETE FROM owed_announcements",
        "DELETE FROM member_addresses",
        "DELETE FROM members",
    ] {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .context("forgetting every device this one was linked to")?;
    }

    tx.commit()
        .await
        .context("forgetting every device this one was linked to")
}
