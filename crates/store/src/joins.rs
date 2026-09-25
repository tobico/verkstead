//! The joins in flight: a link that has been asked for and not yet settled
//! (ADR-0020, *The join*).
//!
//! **Two tables rather than one, because the two sides of a join are not the
//! same record.** The device that pressed Add is waiting on somebody else's
//! human; the device that was asked is holding a question its own human has
//! yet to see. What each keeps is what each has: the asker knows the address it
//! typed and the certificate it met at the far end, and nothing whatever about
//! the device it is asking; the holder knows the whole of what the asker said
//! about itself and the certificate the handshake took from it. One table
//! pretending to be both would be a row with half its columns null on either
//! side of every join.
//!
//! **Rows rather than memory**, which the human settled: a request lives ten
//! minutes, and a restart inside those ten minutes must not silently drop a
//! question somebody is halfway through confirming. So the moment it runs out
//! at is written down — an absolute moment rather than a length, because the
//! ten minutes are counted from when the request was made rather than from
//! whenever this process last came up.
//!
//! **The certificate is kept as its fingerprint**, which is how a device is
//! pinned everywhere else in a cluster — see [`super::members`]. What the
//! holder's row is *for* is the dial back that answers an Allow: it dials the
//! asker at the addresses the asker gave and refuses any certificate but this
//! one, which is a comparison against the fingerprint. And the asker's row is
//! for the other half of that same check — the device that dials it back has to
//! be the one it met.
//!
//! **Neither table hangs off the members.** A join arrives from a non-member by
//! definition, which is what a join *is*, and a foreign key onto a membership
//! the row exists in order to create would be a row that could never be
//! written.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::writing;

/// A join this device asked for: the asker's own record of a press on Add.
///
/// What it holds is what this end knows. The address is the one somebody typed,
/// which is the only place this device has ever been told to look for the other
/// one; the fingerprint is the certificate that address turned out to be
/// presenting, taken off the handshake rather than off anything the far end
/// said about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskedJoin {
    /// What the device being asked calls this request, which is what a cancel
    /// names and what an answer to it will arrive under.
    ///
    /// Invented at the far end rather than here, because the far end is where
    /// the request really lives: this row is a note about a question somebody
    /// else is holding.
    pub request: String,

    /// The address that was typed, which is where this request was posted and
    /// where a cancel goes.
    pub address: String,

    /// The Device Id the far end answered under, which is what the row will be
    /// keyed by once it is a member.
    pub device: String,

    /// And what it is shown under, so that the pending row can say which device
    /// is being waited on rather than repeating the address back.
    pub name: String,

    /// The fingerprint of the certificate met at that address: what the far end
    /// has to turn out to be presenting when it dials back.
    ///
    /// Pinned here for the same reason a member's is pinned on its row — there
    /// is no authority anywhere in a cluster, so a certificate met once and
    /// written down is the whole of what proves the same machine twice.
    pub fingerprint: String,

    /// When Add was pressed, RFC 3339, which is what the list is ordered by.
    pub asked_at: String,

    /// And when the far end lets go of it, RFC 3339 — the far end's own word
    /// for it rather than this device's reckoning.
    ///
    /// Because the far end is what will refuse a cancel naming an expired
    /// request: a row here that read *waiting* after that moment would be a
    /// Cancel pressed on something there is no longer anything to cancel.
    pub expires_at: String,
}

/// And a join this device is holding: the whole of what a stranger said about
/// itself, and the certificate it said it under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldJoin {
    /// What this device calls the request. Invented here, because this is where
    /// the request lives.
    pub request: String,

    /// The Device Id the asker gave, which is what it will be recorded as if
    /// its human's opposite number presses Allow.
    pub device: String,

    /// What it is shown under, and the word for its operating system: the two
    /// things the modal names it by beside the fingerprint.
    pub name: String,
    pub os: String,

    /// Every address it advertised, in the order it advertised them — which is
    /// the order the dial back works down.
    pub addresses: Vec<String>,

    /// The fingerprint of the certificate the handshake took from it.
    ///
    /// **This is what makes the join safe to accept from a stranger.** The post
    /// stands outside the member gate because a join comes from a non-member by
    /// definition, and it is not un-authenticated for it: everything that
    /// follows this request is matched against the certificate pinned here.
    pub fingerprint: String,

    /// When it arrived, RFC 3339.
    pub asked_at: String,

    /// And when this device lets go of it, RFC 3339: ten minutes on from the
    /// moment above, worked out when the request was made rather than counted
    /// from whenever this process last came up.
    pub expires_at: String,
}

/// The two tables, and the addresses hanging off the held one.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS joins_asked (
             request     TEXT PRIMARY KEY,
             address     TEXT NOT NULL,
             device      TEXT NOT NULL,
             name        TEXT NOT NULL,
             fingerprint TEXT NOT NULL,
             asked_at    TEXT NOT NULL,
             expires_at  TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the table of joins this device has asked for")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS joins_held (
             request     TEXT PRIMARY KEY,
             device      TEXT NOT NULL,
             name        TEXT NOT NULL,
             os          TEXT NOT NULL,
             fingerprint TEXT NOT NULL,
             asked_at    TEXT NOT NULL,
             expires_at  TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the table of joins this device is holding")?;

    // The addresses the asker advertised, a row apiece with the position they
    // came in — the shape `member_addresses` is, and for its reason: the dial
    // back works down the list, and a list that came back in some other order
    // would be a device tried on the LAN before the tailnet.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS join_addresses (
             request  TEXT NOT NULL REFERENCES joins_held(request),
             position INTEGER NOT NULL,
             address  TEXT NOT NULL,
             PRIMARY KEY (request, position)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the table of addresses a join advertised")?;

    Ok(())
}

/// Write down a join this device has just asked for.
pub async fn ask_join(pool: &SqlitePool, asked: &AskedJoin) -> Result<()> {
    let mut tx = writing(pool, "writing down a join this device asked for").await?;

    sqlx::query(
        "INSERT INTO joins_asked
             (request, address, device, name, fingerprint, asked_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&asked.request)
    .bind(&asked.address)
    .bind(&asked.device)
    .bind(&asked.name)
    .bind(&asked.fingerprint)
    .bind(&asked.asked_at)
    .bind(&asked.expires_at)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("writing down the join asked of {}", asked.address))?;

    tx.commit()
        .await
        .with_context(|| format!("writing down the join asked of {}", asked.address))
}

/// Every join this device is waiting on, oldest first — which is the order they
/// were pressed in, and the order the pending rows are drawn in.
pub async fn asked_joins(pool: &SqlitePool) -> Result<Vec<AskedJoin>> {
    let rows: Vec<(String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT request, address, device, name, fingerprint, asked_at, expires_at
         FROM joins_asked
         ORDER BY asked_at, request",
    )
    .fetch_all(pool)
    .await
    .context("listing the joins this device is waiting on")?;

    Ok(rows
        .into_iter()
        .map(
            |(request, address, device, name, fingerprint, asked_at, expires_at)| AskedJoin {
                request,
                address,
                device,
                name,
                fingerprint,
                asked_at,
                expires_at,
            },
        )
        .collect())
}

/// One of them by the name the far end gave it, or nothing where this device is
/// waiting on no such thing.
pub async fn asked_join(pool: &SqlitePool, request: &str) -> Result<Option<AskedJoin>> {
    let row: Option<(String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT request, address, device, name, fingerprint, asked_at, expires_at
         FROM joins_asked
         WHERE request = ?",
    )
    .bind(request)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the join asked under {request}"))?;

    Ok(row.map(
        |(request, address, device, name, fingerprint, asked_at, expires_at)| AskedJoin {
            request,
            address,
            device,
            name,
            fingerprint,
            asked_at,
            expires_at,
        },
    ))
}

/// Take a join this device asked for off its own list, which is what Cancel and
/// a dismissed expiry both come down to here.
///
/// Nothing is refused. A row that is already gone is a row that is already
/// gone, and a Cancel pressed twice is not two things happening — the same
/// stance [`super::members::forget_member`] takes.
pub async fn forget_asked_join(pool: &SqlitePool, request: &str) -> Result<()> {
    let mut tx = writing(pool, "forgetting a join this device asked for").await?;

    sqlx::query("DELETE FROM joins_asked WHERE request = ?")
        .bind(request)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting the join asked under {request}"))?;

    tx.commit()
        .await
        .with_context(|| format!("forgetting the join asked under {request}"))
}

/// Hold a join somebody has just asked of this device, with the addresses it
/// advertised.
///
/// One transaction with those addresses, for the reason a member's are written
/// in one with its row: a request whose addresses are half-written is a dial
/// back that would work down an order nobody advertised.
pub async fn hold_join(pool: &SqlitePool, held: &HeldJoin) -> Result<()> {
    let mut tx = writing(pool, "holding a join somebody asked of this device").await?;

    sqlx::query(
        "INSERT INTO joins_held
             (request, device, name, os, fingerprint, asked_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&held.request)
    .bind(&held.device)
    .bind(&held.name)
    .bind(&held.os)
    .bind(&held.fingerprint)
    .bind(&held.asked_at)
    .bind(&held.expires_at)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("holding the join device {} asked", held.device))?;

    for (position, address) in held.addresses.iter().enumerate() {
        sqlx::query("INSERT INTO join_addresses (request, position, address) VALUES (?, ?, ?)")
            .bind(&held.request)
            .bind(position as i64)
            .bind(address)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("recording the addresses device {} advertised", held.device)
            })?;
    }

    tx.commit()
        .await
        .with_context(|| format!("holding the join device {} asked", held.device))
}

/// One of the joins this device is holding, or nothing where it holds no such
/// request.
///
/// **Whether it has run out is not asked here.** The row comes back as it
/// stands, with the moment it expires at on it, and what to do about that is
/// the caller's — an expired request is refused rather than missing, and the
/// two are told apart by reading the moment rather than by the read coming back
/// empty.
pub async fn held_join(pool: &SqlitePool, request: &str) -> Result<Option<HeldJoin>> {
    let row: Option<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT device, name, os, fingerprint, asked_at, expires_at
         FROM joins_held
         WHERE request = ?",
    )
    .bind(request)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the join held under {request}"))?;

    let Some((device, name, os, fingerprint, asked_at, expires_at)) = row else {
        return Ok(None);
    };

    let advertised: Vec<(String,)> =
        sqlx::query_as("SELECT address FROM join_addresses WHERE request = ? ORDER BY position")
            .bind(request)
            .fetch_all(pool)
            .await
            .with_context(|| {
                format!("reading the addresses the join under {request} advertised")
            })?;

    Ok(Some(HeldJoin {
        request: request.to_owned(),
        device,
        name,
        os,
        addresses: advertised.into_iter().map(|(address,)| address).collect(),
        fingerprint,
        asked_at,
        expires_at,
    }))
}

/// Let go of a join this device was holding: a cancel from the far end, and in
/// the stages to come an Allow or a Deny that has been settled.
///
/// The addresses first, because they point at the row — the order
/// [`super::members::forget_member`] takes them in, and for its reason.
///
/// Nothing is refused, for that function's other reason: a request that is
/// already gone is already gone.
pub async fn let_go_of_join(pool: &SqlitePool, request: &str) -> Result<()> {
    let mut tx = writing(pool, "letting go of a join this device was holding").await?;

    sqlx::query("DELETE FROM join_addresses WHERE request = ?")
        .bind(request)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("letting go of the addresses the join {request} advertised"))?;

    sqlx::query("DELETE FROM joins_held WHERE request = ?")
        .bind(request)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("letting go of the join held under {request}"))?;

    tx.commit()
        .await
        .with_context(|| format!("letting go of the join held under {request}"))
}

/// And let go of every join whose ten minutes ran out before `now`.
///
/// **Housekeeping rather than the refusal.** What refuses a call naming an
/// expired request is the moment on the row, read where the call is answered —
/// so this changes nothing about what any caller is told, which is the whole
/// reason it is safe to run: a request this has swept and one it has not are
/// refused the same way and in the same words.
///
/// Run as a fresh join is held, because that is the moment there is a reason to:
/// a device that is asked to link once a year should not be keeping the
/// question it was asked last year, and a device nobody ever asks has nothing
/// to sweep.
pub async fn let_go_of_expired_joins(pool: &SqlitePool, now: &str) -> Result<()> {
    let mut tx = writing(pool, "letting go of the joins that have run out").await?;

    sqlx::query(
        "DELETE FROM join_addresses
         WHERE request IN (SELECT request FROM joins_held WHERE expires_at <= ?)",
    )
    .bind(now)
    .execute(&mut *tx)
    .await
    .context("letting go of the addresses the joins that have run out advertised")?;

    sqlx::query("DELETE FROM joins_held WHERE expires_at <= ?")
        .bind(now)
        .execute(&mut *tx)
        .await
        .context("letting go of the joins that have run out")?;

    tx.commit()
        .await
        .context("letting go of the joins that have run out")
}
