//! The Declared wait: a session saying it is about to end its turn with work of
//! its own still running in the background, by running `verkstead waiting 45m`.
//!
//! An agent that starts a build or a test run in the background and ends its
//! turn is idle by every reading Verkstead has — the shape of a session that has
//! finished, or one that is stuck — and the Rescue would speak to it and then
//! tell the human. So it says so first, naming how long it expects to wait.
//!
//! **A waiting session is active, not Idle**, and that is one change to the one
//! judgement everything reads — see [`crate::sessions::Idle`] — rather than a
//! rule each reader keeps: the card and the Timeline row show it at work, the
//! Rescue never speaks to it, and the long-stop does not call it stopped.
//!
//! **Over when its time runs out**, when the session is next seen at work after
//! going quiet behind it, or on an accepted Done signal. At most an hour, and
//! fifteen minutes where none is named: a wait can always be declared again, so
//! the maximum bounds only how long a wait whose background task died sits
//! before anybody asks. See ADR-0018.

use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use verkstead_schema::ApiError;

use crate::AppState;
use crate::reply::yaml;

/// How long a wait stands where the agent names no length.
pub(crate) const UNNAMED: Duration = Duration::from_secs(15 * 60);

/// The longest wait there is.
pub(crate) const LONGEST: Duration = Duration::from_secs(60 * 60);

/// `POST /conversations/{conversation}/api/v1/waiting` — the session running in
/// this Conversation says it is waiting on work of its own, for as long as the
/// body says: `45m`, `90s`, `1h`, or nothing for [`UNNAMED`].
///
/// 200 with the wait standing, replacing any wait already there. 409 where the
/// length does not parse, is longer than [`LONGEST`], or there is no session
/// running here to be waiting.
pub(crate) async fn declare(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    body: String,
) -> Response {
    let length = match length(&body) {
        Ok(length) => length,
        Err(why) => return refused(conversation_id, why),
    };

    let Some(session) = state.sessions.following(conversation_id) else {
        return refused(
            conversation_id,
            "there is no session running in this Conversation, so there is nothing here to be \
             waiting"
                .to_owned(),
        );
    };

    session.idle.waiting(length);

    tracing::info!(
        conversation_id,
        seconds = length.as_secs(),
        "a session declared a wait on work of its own, so it reads as at work until that is over"
    );

    (StatusCode::OK, format!("waiting {}\n", said(length))).into_response()
}

fn refused(conversation_id: i64, why: String) -> Response {
    tracing::info!(
        conversation_id,
        why,
        "a session's declared wait was refused"
    );

    yaml(StatusCode::CONFLICT, &ApiError::new(why))
}

/// The length a wait was declared for: a whole number of seconds, minutes or
/// hours — `90s`, `45m`, `1h` — or [`UNNAMED`] where nothing was named.
///
/// Refused, in words naming the maximum, where it does not parse, is nothing,
/// or is longer than [`LONGEST`].
pub(crate) fn length(said: &str) -> Result<Duration, String> {
    let said = said.trim();

    if said.is_empty() {
        return Ok(UNNAMED);
    }

    let unparsed = || {
        format!(
            "`{said}` is not a length to wait: give a whole number of seconds, minutes or hours, \
             such as `90s`, `45m` or `1h`, of at most {}",
            self::said(LONGEST)
        )
    };

    let split = said
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(unparsed)?;
    let (number, unit) = said.split_at(split);
    let number = number.parse::<u64>().map_err(|_| unparsed())?;

    let seconds = match unit {
        "s" => Some(number),
        "m" => number.checked_mul(60),
        "h" => number.checked_mul(60 * 60),
        _ => return Err(unparsed()),
    };

    match seconds.map(Duration::from_secs) {
        Some(length) if length.is_zero() => Err(unparsed()),
        Some(length) if length <= LONGEST => Ok(length),
        _ => Err(format!(
            "a wait of `{said}` is longer than the maximum of {}: declare {} and declare it again \
             if the work runs over",
            self::said(LONGEST),
            self::said(LONGEST)
        )),
    }
}

/// A length in the words a reply uses: `1h`, `45m`, `90s`.
fn said(length: Duration) -> String {
    let seconds = length.as_secs();

    if seconds % 3600 == 0 {
        format!("{}h", seconds / 3600)
    } else if seconds % 60 == 0 {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_is_seconds_minutes_or_hours() {
        assert_eq!(length("90s"), Ok(Duration::from_secs(90)));
        assert_eq!(length("45m"), Ok(Duration::from_secs(45 * 60)));
        assert_eq!(length("1h"), Ok(Duration::from_secs(3600)));
        assert_eq!(length(" 45m\n"), Ok(Duration::from_secs(45 * 60)));
    }

    #[test]
    fn no_length_is_fifteen_minutes() {
        assert_eq!(length(""), Ok(Duration::from_secs(15 * 60)));
        assert_eq!(length("\n"), Ok(Duration::from_secs(15 * 60)));
    }

    #[test]
    fn longer_than_an_hour_is_refused_naming_the_maximum() {
        for longer in ["61m", "2h", "3601s", "99999999999999999h"] {
            let why = length(longer).unwrap_err();

            assert!(
                why.contains("maximum of 1h"),
                "{longer} is refused naming the maximum: {why}"
            );
        }
    }

    #[test]
    fn a_length_that_does_not_parse_is_refused_naming_the_maximum() {
        for unparsed in ["45", "m", "45 minutes", "1.5h", "-5m", "0m", "soon"] {
            let why = length(unparsed).unwrap_err();

            assert!(
                why.contains("not a length to wait") && why.contains("at most 1h"),
                "{unparsed} is refused naming the maximum: {why}"
            );
        }
    }
}
