//! What a stage worked on another device tells its **home**, and what home
//! writes onto its tombstone (ADR-0029, *Home drives a roadmap across devices*).
//!
//! **Home is the stage's birth device.** Every stage is made on the device
//! driving its roadmap, so the **Birth Key** names home and never changes. A
//! stage moved away leaves a tombstone there — see [`super::births`] — and the
//! roadmap is read off the record it always was: `stage_standings`, the places
//! and the chain all read home's rows, the tombstone's among them. So the
//! tombstone is kept true rather than every reader being taught where else to
//! look: the copy holding the work tells home each lifecycle move, and home
//! writes it onto the row that reads.
//!
//! **What is told is the whole standing**, not the move that changed it. The
//! standings read more than the state column — whether the Conversation ever
//! moved to Done, whether it ever moved to Wrapping, and whether it is stopped —
//! so all four travel together, and a report sent twice writes the same row
//! twice. Which is what lets the far end resend freely: [`told`] remembers what
//! home last took, and anything else is still owed.
//!
//! **And the join, which is the chain's fact.** A stage joins its roadmap's
//! chain by opening its pull request, and home's chain is read off the pull
//! requests recorded against its stages — so the one a copy away from home
//! opened travels with the standing, and home records it on the tombstone in the
//! order it learned of it. See [`JoinedAway`].

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::births::Birth;
use super::conversations::{Event, Lifecycle};
use super::stops::Decision;

/// What a roadmap reads of one stage: where its live copy is, whether it has
/// ever been in Done or in Wrapping, and whether it is stopped.
///
/// Exactly the four facts `stage_standings` reads off a Conversation, so a
/// tombstone written with them reads the standing its live copy does — and a
/// fifth, the pull request it joined its chain with, which is what
/// `stage_chain` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStanding {
    /// The state it is in now.
    pub state: Lifecycle,

    /// Whether it has ever moved to Done.
    pub done: bool,

    /// Whether it has ever moved to Wrapping, which is what tells a stage closed
    /// from a wrap-up from one abandoned part way.
    pub wrapped: bool,

    /// And whether it is stopped, waiting on somebody's press.
    pub stopped: bool,

    /// The pull request it joined its roadmap's chain with, once it has.
    pub joined: Option<JoinedAway>,
}

/// The pull request a stage opened as it joined its roadmap's chain, and the
/// branch it joined with, as its live copy's record names them.
///
/// The first pull request in the stage's own Repo, which is the one its finish
/// opened: a companion's is another repository's, and the rest of a stack it
/// was later found on is not what put it in the chain. What home's chain reads
/// of it is the branch; the rest is the row that branch hangs off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedAway {
    /// The branch, as the record names it now — the name a session renamed it
    /// to where one did.
    pub branch: String,

    /// The number GitHub gave the pull request.
    pub number: i64,

    /// Its title and where it is.
    pub title: String,
    pub url: String,

    /// And the branch it goes into, where that was written down.
    pub base: Option<String>,
}

/// The one table: what this device last told a stage's home about it, kept on
/// the copy away from home that did the telling.
///
/// Keyed by the Conversation, one row at most. No row is a copy that has told
/// home nothing yet, which is every copy that has just arrived. The `joined_`
/// columns are all empty together where home was told of no join.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stage_reports (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             state           TEXT NOT NULL,
             done            INTEGER NOT NULL,
             wrapped         INTEGER NOT NULL,
             stopped         INTEGER NOT NULL,
             joined_branch   TEXT,
             joined_number   INTEGER,
             joined_title    TEXT,
             joined_url      TEXT,
             joined_base     TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the stage_reports table")?;

    Ok(())
}

/// Where `conversation_id`'s home is, where it is a stage whose live record is
/// on this device and whose home is not — a **copy away from home**.
///
/// `None` for everything else: a Conversation that is no stage, one born here,
/// and a tombstone, which is no live record at all. `here` is this device's
/// Device Id, which the store has no way of knowing for itself.
pub async fn away_from_home(
    pool: &SqlitePool,
    conversation_id: i64,
    here: &str,
) -> Result<Option<Birth>> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT b.device, b.born_as
         FROM births b
         JOIN stage_roadmaps r ON r.conversation_id = b.conversation_id
         WHERE b.conversation_id = ? AND b.device <> ? AND r.stage IS NOT NULL
           AND NOT EXISTS (SELECT 1 FROM transferred t
                           WHERE t.conversation_id = b.conversation_id)",
    )
    .bind(conversation_id)
    .bind(here)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading whether Conversation {conversation_id} is a stage away from home")
    })?;

    Ok(row.map(|(device, id)| Birth { device, id }))
}

/// Every Conversation [`away_from_home`] would answer for, in id order: what a
/// sweep for reports still owed walks.
pub async fn stages_away_from_home(pool: &SqlitePool, here: &str) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT b.conversation_id
         FROM births b
         JOIN stage_roadmaps r ON r.conversation_id = b.conversation_id
         WHERE b.device <> ? AND r.stage IS NOT NULL
           AND NOT EXISTS (SELECT 1 FROM transferred t
                           WHERE t.conversation_id = b.conversation_id)
         ORDER BY b.conversation_id",
    )
    .bind(here)
    .fetch_all(pool)
    .await
    .context("reading which stages on this device are away from home")?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// What `conversation_id` stands at now, read the way `stage_standings` reads
/// it; `None` where there is no such Conversation.
pub async fn live_standing(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Option<LiveStanding>> {
    let row: Option<(String, bool, bool, bool)> = sqlx::query_as(
        "SELECT c.state,
                EXISTS (
                    SELECT 1 FROM timeline_events e
                    WHERE e.conversation_id = c.id AND e.kind = ? AND e.body = ?
                ),
                EXISTS (
                    SELECT 1 FROM timeline_events e
                    WHERE e.conversation_id = c.id AND e.kind = ? AND e.body = ?
                ),
                c.stopped_at IS NOT NULL
         FROM conversations c WHERE c.id = ?",
    )
    .bind(Event::Moved(Lifecycle::Done).kind())
    .bind(Lifecycle::Done.stored())
    .bind(Event::Moved(Lifecycle::Wrapping).kind())
    .bind(Lifecycle::Wrapping.stored())
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading what Conversation {conversation_id} stands at"))?;

    let Some((state, done, wrapped, stopped)) = row else {
        return Ok(None);
    };

    // The first pull request in its own Repo, and the branch as the record names
    // it — see [`JoinedAway`].
    let joined: Option<(String, i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT COALESCE(c.named_branch, c.branch), p.number, p.title, p.url, p.base_branch
         FROM conversations c
         JOIN pull_requests p ON p.conversation_id = c.id AND p.repo_id = c.repo_id
         WHERE c.id = ?
         ORDER BY p.event_id
         LIMIT 1",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading the pull request Conversation {conversation_id} joined with")
    })?;

    Ok(Some(LiveStanding {
        state: Lifecycle::read(&state)?,
        // In Done now is in Done ever, whether or not the move was written.
        done: done || Lifecycle::reads_as(&state, Lifecycle::Done),
        wrapped,
        stopped,
        joined: joined.map(|(branch, number, title, url, base)| JoinedAway {
            branch,
            number,
            title,
            url,
            base,
        }),
    }))
}

/// What home last took of `conversation_id`'s standing, where it has taken
/// anything.
pub async fn told(pool: &SqlitePool, conversation_id: i64) -> Result<Option<LiveStanding>> {
    /// The columns in the order the query below selects them.
    type Row = (
        String,
        bool,
        bool,
        bool,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT state, done, wrapped, stopped,
                joined_branch, joined_number, joined_title, joined_url, joined_base
         FROM stage_reports WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading what home was last told about Conversation {conversation_id}")
    })?;

    row.map(
        |(state, done, wrapped, stopped, branch, number, title, url, base)| {
            Ok(LiveStanding {
                state: Lifecycle::read(&state)?,
                done,
                wrapped,
                stopped,
                joined: match (branch, number, title, url) {
                    (Some(branch), Some(number), Some(title), Some(url)) => Some(JoinedAway {
                        branch,
                        number,
                        title,
                        url,
                        base,
                    }),
                    _ => None,
                },
            })
        },
    )
    .transpose()
}

/// Write down that home has taken `standing` for `conversation_id`, so it is not
/// told again.
pub async fn tell(pool: &SqlitePool, conversation_id: i64, standing: &LiveStanding) -> Result<()> {
    sqlx::query(
        "INSERT INTO stage_reports (conversation_id, state, done, wrapped, stopped,
                                    joined_branch, joined_number, joined_title, joined_url,
                                    joined_base)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (conversation_id) DO UPDATE SET state = excluded.state,
                                                     done = excluded.done,
                                                     wrapped = excluded.wrapped,
                                                     stopped = excluded.stopped,
                                                     joined_branch = excluded.joined_branch,
                                                     joined_number = excluded.joined_number,
                                                     joined_title = excluded.joined_title,
                                                     joined_url = excluded.joined_url,
                                                     joined_base = excluded.joined_base",
    )
    .bind(conversation_id)
    .bind(standing.state.stored())
    .bind(standing.done)
    .bind(standing.wrapped)
    .bind(standing.stopped)
    .bind(standing.joined.as_ref().map(|joined| &joined.branch))
    .bind(standing.joined.as_ref().map(|joined| joined.number))
    .bind(standing.joined.as_ref().map(|joined| &joined.title))
    .bind(standing.joined.as_ref().map(|joined| &joined.url))
    .bind(
        standing
            .joined
            .as_ref()
            .and_then(|joined| joined.base.as_ref()),
    )
    .execute(pool)
    .await
    .with_context(|| {
        format!("recording what home was told about Conversation {conversation_id}")
    })?;

    Ok(())
}

/// What became of writing a reported standing onto a tombstone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Followed {
    /// Whether this report is what moved the tombstone into Done — the moment a
    /// local settle carries the roadmap on at, and the one a resend of the same
    /// report is not.
    pub settled: bool,
}

/// Write `standing` onto the tombstone `conversation_id`, so that it reads what
/// its live copy reads.
///
/// **The tombstone is otherwise written by nothing**, so this is blunt: the
/// state is set, a move into Done or Wrapping the copy has made and the
/// tombstone has no line for is put on its Timeline, and the stop is put on or
/// taken off. Nobody opens a tombstone — its URL leads to the live copy — so
/// those lines are for the readings, not for a person. One transaction, so a
/// reading never finds the state moved and the line saying it has not.
///
/// **And the join, where the copy has opened its pull request.** The tombstone
/// is put on the branch the copy joined with and the pull request is recorded
/// against it, which is what puts the stage in home's chain — after every link
/// already there, its Event being written now: the order home's chain reads is
/// the order home learned of each join. A pull request it holds already is
/// recorded nothing twice.
///
/// Whether the caller may make this write — that `conversation_id` is a
/// tombstone, and that the device reporting is the one it names — is the
/// caller's to have asked.
pub async fn follow(
    pool: &SqlitePool,
    conversation_id: i64,
    standing: &LiveStanding,
) -> Result<Followed> {
    let mut tx = super::writing(pool, "writing a stage's standing onto its tombstone").await?;

    let before: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT state, stopped_at FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("reading the tombstone of Conversation {conversation_id}"))?;

    let Some((was, stopped_at)) = before else {
        return Ok(Followed { settled: false });
    };

    let was_done = Lifecycle::reads_as(&was, Lifecycle::Done);

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(standing.state.stored())
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "moving the tombstone of Conversation {conversation_id} to {}",
                standing.state.stored(),
            )
        })?;

    for (ever, state) in [
        (standing.wrapped, Lifecycle::Wrapping),
        (standing.done, Lifecycle::Done),
    ] {
        let moved: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM timeline_events
                            WHERE conversation_id = ? AND kind = ? AND body = ?)",
        )
        .bind(conversation_id)
        .bind(Event::Moved(state).kind())
        .bind(state.stored())
        .fetch_one(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "reading whether Conversation {conversation_id} ever moved to {}",
                state.stored()
            )
        })?;

        if ever && !moved.0 {
            super::conversations::moved(&mut tx, conversation_id, state).await?;
        }
    }

    if let Some(joined) = &standing.joined {
        sqlx::query(
            "UPDATE conversations SET named_branch = ?
             WHERE id = ? AND COALESCE(named_branch, branch) <> ?",
        )
        .bind(&joined.branch)
        .bind(conversation_id)
        .bind(&joined.branch)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("putting the tombstone of Conversation {conversation_id} on its branch")
        })?;

        let (repo_id,): (i64,) = sqlx::query_as("SELECT repo_id FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_one(&mut *tx)
            .await
            .with_context(|| format!("reading the Repo of Conversation {conversation_id}"))?;

        super::pull_requests::record(
            &mut tx,
            conversation_id,
            repo_id,
            &super::PullRequest {
                number: joined.number,
                title: joined.title.clone(),
                url: joined.url.clone(),
                head: Some(joined.branch.clone()),
                base: joined.base.clone(),
                repo: None,
            },
        )
        .await?;
    }

    match (standing.stopped, stopped_at.is_some()) {
        (true, false) => {
            let (notice,): (i64,) = sqlx::query_as(
                "INSERT INTO timeline_events (conversation_id, at, kind, body)
                 VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)
                 RETURNING id",
            )
            .bind(conversation_id)
            .bind(Event::Notice(String::new()).kind())
            .bind("Stopped on the device holding the work.")
            .fetch_one(&mut *tx)
            .await
            .with_context(|| {
                format!("putting a stop on the tombstone of Conversation {conversation_id}")
            })?;

            // Decided rather than circumstance, so that a restart here takes
            // nothing up: the work is not here to take up.
            sqlx::query(
                "UPDATE conversations
                    SET stopped_at     = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                        stopped_by     = ?,
                        stopped_notice = ?
                  WHERE id = ?",
            )
            .bind(Decision::Verkstead.stored())
            .bind(notice)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("putting a stop on the tombstone of Conversation {conversation_id}")
            })?;
        }

        (false, true) => {
            sqlx::query(
                "UPDATE conversations
                    SET stopped_at = NULL, stopped_by = NULL,
                        stopped_notice = NULL, stopped_resets = NULL,
                        stopped_signed_out = NULL
                  WHERE id = ?",
            )
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("taking the stop off the tombstone of Conversation {conversation_id}")
            })?;
        }

        _ => {}
    }

    tx.commit()
        .await
        .context("writing a stage's standing onto its tombstone")?;

    Ok(Followed {
        settled: !was_done && standing.state == Lifecycle::Done,
    })
}
