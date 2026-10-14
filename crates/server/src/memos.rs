//! Leaving a Memo: a building session putting what it learned on the record for
//! the sessions after it, by running `verkstead memo` (ADR-0023).
//!
//! **Only a building session may**, and the server holds that rather than the
//! skills: what each running session was sent for is already written down, on
//! the register the Done signal is checked against — see
//! [`crate::done::Signals::expecting`]. A task, the finish, a stage's planning,
//! an inline run and a steered instruction each have a session after them with
//! none of their context. A grilling has the Handoff and a wrap-up has the pull
//! request, and every other kind is refused by name, saying which may.
//!
//! **Stamped at the moment it is left**, in words rather than as a step — see
//! [`Building::left_by`] — and with the running session's own Event beside
//! them, so a card can say which session that was.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use verkstead_schema::{ApiError, MemoLeft, memo_refused};

use crate::AppState;
use crate::reply::yaml;
use crate::store;

/// What a session that may leave a Memo was sent for.
///
/// Written beside its entry on the Done register by the driver seeing it out,
/// and `None` there for every session that may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Building {
    /// A backlog task, by the number of its entry.
    Task(u32),
    Finish,
    /// A roadmap stage's own planning into a backlog.
    Stage,
    Inline,
    Instruction,
}

/// The kinds that may, as a refusal names them.
const MAY: &str = "a backlog task, the finish, a stage's planning, an inline run or a steered \
                   instruction";

impl Building {
    /// The words a Memo left by this session is stamped with, on the card and
    /// in every later prompt.
    pub(crate) fn left_by(self) -> String {
        match self {
            Building::Task(number) => format!("the session that worked task {number:02}"),
            Building::Finish => "the finish session".to_owned(),
            Building::Stage => "the session that planned the stage".to_owned(),
            Building::Inline => "the inline session".to_owned(),
            Building::Instruction => "the instructed session".to_owned(),
        }
    }
}

/// `POST /conversations/{conversation}/api/v1/memos` — the session running in
/// this Conversation leaves the markdown in the body for the sessions after it.
///
/// 200 with the new Memo's Event id. 409 where the memo is empty or over
/// [`verkstead_schema::MAX_MEMO_BYTES`], where no session is running here, or
/// where the one running is not a building one.
pub(crate) async fn leave(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    markdown: String,
) -> Response {
    if let Some(why) = memo_refused(&markdown) {
        return refused(conversation_id, why);
    }

    let Some(event_id) = state.sessions.writing(conversation_id) else {
        return refused(
            conversation_id,
            "there is no session running in this Conversation, so there is nobody here to leave \
             a memo"
                .to_owned(),
        );
    };

    let Some(building) = crate::done::building(&state, conversation_id, event_id).await else {
        return refused(
            conversation_id,
            format!(
                "only a building session may leave a memo — {MAY} — and this session was sent \
                 for something else"
            ),
        );
    };

    let left_by = building.left_by();

    match store::leave_memo(&state.pool, conversation_id, &markdown, &left_by, event_id).await {
        Ok(Some(id)) => {
            tracing::info!(
                conversation_id,
                id,
                left_by,
                "a session left a memo for the sessions after it"
            );

            yaml(StatusCode::OK, &MemoLeft { id })
        }
        Ok(None) => refused(
            conversation_id,
            "this Conversation has gone, so there is nowhere to leave a memo".to_owned(),
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "leaving a memo failed");

            yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("Verkstead could not write the memo down, so run it again"),
            )
        }
    }
}

fn refused(conversation_id: i64, why: String) -> Response {
    tracing::info!(conversation_id, why, "a session's memo was refused");

    yaml(StatusCode::CONFLICT, &ApiError::new(why))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_is_stamped_with_its_number_as_the_backlog_spells_it() {
        assert_eq!(
            Building::Task(3).left_by(),
            "the session that worked task 03"
        );
        assert_eq!(
            Building::Task(12).left_by(),
            "the session that worked task 12"
        );
    }
}
