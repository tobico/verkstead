//! The one-time rewrites a database written by an older Verkstead needs, run as
//! it is opened.
//!
//! Every other schema statement here is a `CREATE TABLE IF NOT EXISTS`: the
//! shape is declared, and a database that already has it is left alone. What
//! lives in this module is the other kind — a rewrite of rows that are already
//! there, run once because the thing they described has been replaced by
//! something else.
//!
//! Each is written to be safe against a database that has already had it, and
//! what says whether there is anything to do is the presence of what it
//! rewrites rather than a version number kept somewhere. So a database opened
//! twice is rewritten once, and one made from scratch this morning has nothing
//! here to do at all.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::stops::Decision;

/// Run whatever this database still needs, in the order it needs them.
pub(crate) async fn apply(pool: &SqlitePool) -> Result<()> {
    stops_recorded_the_old_way(pool).await
}

/// The table the stops of before are kept in, named once: it is gone by the end
/// of this module, and nothing else in the codebase knows the word.
const OLD_TABLE: &str = "interruptions";

/// Rewrite the stops a Verkstead of before kept in a table of their own into the
/// Notices and stops they are now, and take the table away.
///
/// A stop used to be a row of separate facts joined onto its Timeline Event —
/// which step failed, how it ended, what git made of the Worktree, the tail of
/// what the session said, and whichever of three remedies the human chose. A
/// stop is now four columns on the Conversation and an ordinary **Notice** on
/// its Timeline: prose somebody reads, and one durable fact saying the
/// Conversation is stopped *now*. So the columns become the markdown they would
/// have been written as, and each Event becomes the Notice it would have been.
///
/// The ones still open become stops as well, and deliberate ones: an open stop
/// was a run waiting on the human, which is exactly the stop a restart leaves
/// alone. A Conversation that is stopped already keeps the stop it has — there
/// is one per Conversation, and the first Notice is the one that explains it.
///
/// One transaction, and the table is dropped inside it: a Timeline holding rows
/// that have been rewritten beside a table that still says otherwise would be
/// two accounts of the same stop.
async fn stops_recorded_the_old_way(pool: &SqlitePool) -> Result<()> {
    let table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(OLD_TABLE)
            .fetch_optional(pool)
            .await
            .context("looking for the table the stops of before are kept in")?;

    if table.is_none() {
        return Ok(());
    }

    /// The columns in the order the query below selects them.
    type Row = (
        i64,
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    );

    let mut tx = pool
        .begin()
        .await
        .context("rewriting the stops of before")?;

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT i.event_id, i.conversation_id, i.what, i.how, i.git_status, i.tail,
                i.remedy, i.note, i.settled_at, e.at
         FROM interruptions i
         JOIN timeline_events e ON e.id = i.event_id
         ORDER BY i.event_id",
    )
    .fetch_all(&mut *tx)
    .await
    .context("reading the stops of before")?;

    for (event_id, conversation_id, what, how, git_status, tail, remedy, note, settled_at, at) in
        &rows
    {
        let said = said(
            what,
            how,
            git_status,
            tail,
            remedy.as_deref(),
            note.as_deref(),
            settled_at.as_deref(),
        );

        sqlx::query("UPDATE timeline_events SET kind = 'notice', body = ? WHERE id = ?")
            .bind(&said)
            .bind(event_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("rewriting the stop of Event {event_id} as a Notice"))?;

        // Only the open ones. A settled stop is a run that got going again, or
        // one the human ended, and nothing is waiting on either now.
        //
        // The Event's own time rather than this morning's: the stop happened
        // whenever it happened, and a stop stamped with the migration would say
        // the work stopped when the server was upgraded.
        if remedy.is_none() {
            // Only where the Conversation is not stopped already, which is the
            // one stop per Conversation said as a `WHERE`: a Conversation that
            // has one keeps it, and the first Notice is the one that explains it.
            sqlx::query(
                "UPDATE conversations
                    SET stopped_at = ?, stopped_by = ?, stopped_notice = ?
                  WHERE id = ? AND stopped_at IS NULL",
            )
            .bind(at)
            .bind(Decision::Deliberate.stored())
            .bind(event_id)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("recording that Conversation {conversation_id} is stopped"))?;
        }
    }

    sqlx::query(&format!("DROP TABLE {OLD_TABLE}"))
        .execute(&mut *tx)
        .await
        .context("taking away the table the stops of before are kept in")?;

    tx.commit().await.context("rewriting the stops of before")
}

/// One old row as the markdown its Notice holds.
///
/// The same shape a stop's Notice is written in today — the stop, the reason,
/// and both pieces of evidence under headings of their own — because it is the
/// same thing being said, and the human reads the two in one list. Written here
/// rather than borrowed from the half of the server that writes the live ones:
/// this is a shape rows are put into once and never again, and a migration that
/// moved with the prose around it would rewrite yesterday's database differently
/// from today's.
///
/// The remedy is noted at the end where one was chosen, because it is the rest
/// of what happened: a stop somebody answered reads wrong without the answer.
fn said(
    what: &str,
    how: &str,
    git_status: &str,
    tail: &str,
    remedy: Option<&str>,
    note: Option<&str>,
    settled_at: Option<&str>,
) -> String {
    let mut said = format!(
        "**{}** stopped.\n\n{how}\n\n### The worktree\n\n{}\n\n### What the last session said\n\n{}\n",
        opening(what),
        indented(
            git_status,
            "Git had nothing pending, or the repository would not answer.",
        ),
        indented(tail, "It said nothing at all."),
    );

    let Some(remedy) = remedy else {
        return said;
    };

    said.push_str("\n### What was done about it\n\n");
    said.push_str(&chosen(remedy, settled_at));

    if let Some(note) = note.map(str::trim).filter(|note| !note.is_empty()) {
        said.push_str("\n\n");
        said.push_str(note);
    }

    said.push('\n');

    said
}

/// The remedy in a sentence, and when it was chosen.
///
/// A word this Verkstead does not know is written down as it was found rather
/// than refused on, unlike every other stored word here: the row is history by
/// the time this runs, and a database that would not open on account of one
/// would be worse than a Notice naming a remedy nobody remembers.
fn chosen(remedy: &str, settled_at: Option<&str>) -> String {
    let named = match remedy {
        "retry" => "The step was run again",
        "take-over" => "Verkstead stopped driving, and the Worktree was the human's",
        "abort" => "The run was ended here",
        other => return format!("The stop was settled by `{other}`.\n"),
    };

    match settled_at {
        Some(at) => format!("{named}, on {at}."),
        None => format!("{named}."),
    }
}

/// The stop with its first letter up, because it opens the sentence the Notice
/// is.
fn opening(what: &str) -> String {
    let mut letters = what.chars();

    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// A block of terminal output as markdown holds it: every line indented by
/// four, which is the one form nothing inside it can break out of.
///
/// `empty` is the sentence that stands in its place where there is nothing to
/// show, and it is prose rather than a block: there is nothing to preserve the
/// columns of.
fn indented(said: &str, empty: &str) -> String {
    let block: Vec<String> = said.lines().map(|line| format!("    {line}")).collect();

    if block.is_empty() {
        return empty.to_owned();
    }

    block.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An open stop reads as the Notices written today read: what stopped,
    /// why, and both blocks of evidence set apart from the prose.
    #[test]
    fn an_open_stop_becomes_the_notice_it_would_have_been() {
        let said = said(
            "the task in .tasks/03-commit-events.md",
            "the session exited with status 1",
            "## task-runner\n M commits.rs",
            "error: could not compile",
            None,
            None,
            None,
        );

        assert_eq!(
            said,
            "**The task in .tasks/03-commit-events.md** stopped.\n\n\
             the session exited with status 1\n\n\
             ### The worktree\n\n\
             \x20   ## task-runner\n    \x20M commits.rs\n\n\
             ### What the last session said\n\n\
             \x20   error: could not compile\n",
        );
        assert!(
            !said.contains("What was done about it"),
            "nothing was done about it: {said:?}",
        );
    }

    /// And a settled one carries what the human chose, with whatever they wrote
    /// beside it — the rest of what happened, which a Notice without it reads
    /// wrong without.
    #[test]
    fn a_settled_stop_says_what_was_done_about_it() {
        let said = said(
            "implementing the work",
            "the session exited with status 1",
            "",
            "",
            Some("retry"),
            Some("try again but leave the migration alone"),
            Some("2026-08-01T09:14:22.000Z"),
        );

        assert!(
            said.contains(
                "### What was done about it\n\n\
                 The step was run again, on 2026-08-01T09:14:22.000Z.\n\n\
                 try again but leave the migration alone\n"
            ),
            "{said:?}",
        );
    }

    /// A remedy word this Verkstead does not know is written down as it was
    /// found. The row is history, and a database that would not open on account
    /// of one would be worse than a Notice nobody quite understands.
    #[test]
    fn a_remedy_nobody_remembers_is_still_written_down() {
        let said = said(
            "grilling the work",
            "it went quiet",
            "",
            "",
            Some("sleep"),
            None,
            None,
        );

        assert!(said.contains("settled by `sleep`"), "{said:?}");
    }
}
