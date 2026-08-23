//! Starting a grilling again on a Conversation whose grilling session died.
//!
//! The one relaunch that cannot pick up where the last one left off. A run's
//! driver reads what is next off the repository and a wrap-up's watchers ask the
//! pull request, so both of those take up work that is written down somewhere —
//! but a grilling is an interview, and an interview lives in the session having
//! it. Nothing survives the process it ran in, so what a retried grilling is, is
//! a fresh one: the same Brief, from the beginning.
//!
//! **Except for what was already settled.** The Questions the dead session asked
//! and the Answers the human gave are on the Timeline, which is the one part of
//! an interview that does outlive it — so they go into the prompt as a digest
//! under the Brief, in the order they were asked. A grilling that opened by
//! asking again what the human had already decided would be a retry that cost
//! them the interview twice.
//!
//! **And except for what was left hanging.** A session that died mid-question
//! leaves a Question Set open with nothing waiting on the Answer: the human can
//! still see it, still answer it, and nothing will ever read what they write.
//! So the relaunch archives it unanswered first — the same archiving the human
//! reaches by hand for a Set whose agent has gone.

use verkstead_schema::{Nudge, QuestionOption, QuestionSet, Response};

use crate::AppState;
use crate::drivers::Driving;
use crate::skills;
use crate::store;

/// What is written where the human left a question open, and where nothing came
/// back for one at all.
///
/// Said rather than left blank, because the two readings are opposite: a
/// question with nothing under it reads as a question nobody put, and this is a
/// question the human saw and chose not to settle. The new grilling is welcome
/// to ask it again, and this is what tells it that it may.
const LEFT_OPEN: &str = "_Left open._";

/// Grill the work again, because the human pressed Retry on the stall that said
/// nothing was.
///
/// `note` is what they wrote beside the Remedy, and it reaches the session where
/// every other retry's note reaches one: last, under everything else — see
/// [`skills::retrying`].
///
/// `driving` is the registration [`crate::runner::retry`] took as the press
/// arrived, held across the launch and let go once the session is registered.
/// What drives a grilling is its session and nothing else — see
/// [`crate::drivers`] — so this is a handover to the session rather than to a
/// task of Verkstead's, and it is over the moment there is one.
pub(crate) async fn again(state: AppState, conversation_id: i64, note: String, driving: Driving) {
    // Waited for rather than tried for, which is the way round every Remedy
    // takes it: the human answers an Interruption whenever they get to it, and
    // nothing is holding a request open on the reply. So whatever is running —
    // a Manual Task they set going while the work stood still — finishes, and
    // this goes next rather than killing it mid-sentence.
    let _turn = state.sessions.turn(conversation_id).await;

    // One read for all three of the things a relaunch needs off the record: the
    // Brief it starts from, what has already been settled, and what was left
    // hanging. A grilling's Timeline runs to hundreds of Events by the time
    // anything has gone wrong with it, and one relaunch is no reason to read it
    // three times.
    let timeline = match store::timeline(&state.pool, conversation_id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a grilling had got to failed");
            return;
        }
    };

    orphaned(&state, conversation_id, &timeline).await;

    let prompt = skills::retrying(
        &skills::grilling_again(&brief(&timeline), &settled(&timeline)),
        &note,
    );

    // Read back here rather than carried from anywhere: a stall may be answered
    // the next morning, and where an agent is about to be let loose is the one
    // thing that must not be guessed at.
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left to grill");
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to grill again failed");
            return;
        }
    };

    // The grilling Profile, which is the one a grilling runs under whatever else
    // has happened since — the implementation one is what the work is built
    // under, and this session is not building anything.
    let Some(profile) = conversation.grilling_profile.clone() else {
        tracing::error!(
            conversation_id,
            "the grilling Profile is gone, so the grilling was not started again"
        );
        return;
    };

    // One Worktree holds one agent. The session this is replacing died rather
    // than being ended, so a register still holding a relay that has not
    // finished unwinding would be two agents editing each other's files.
    state.sessions.end(conversation_id).await;

    let started = state
        .sessions
        .start(&state.pool, &state.nudges, &conversation, &profile, &prompt)
        .await;

    match started {
        Ok(Some(session)) => tracing::info!(
            conversation_id,
            event_id = session.event_id,
            "a stalled grilling is being grilled again"
        ),
        Ok(None) => tracing::error!(
            conversation_id,
            "no session could be started to grill the work again"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a relaunched grilling could not be started");
        }
    }

    // Whatever came of it, the registration goes now: either there is a session
    // driving the Conversation or there is nothing to drive it with, and the
    // second of those is a stall the next sweep should find.
    drop(driving);
}

/// Archive every Question Set the dead session left open, so that nothing is
/// left for the human to answer into.
///
/// An open Set is a question with a reader, and the reader has gone: the badge
/// still says *blocked on you*, the Set still takes an Answer, and what the
/// human writes goes nowhere. Archiving unanswered is what that Set has always
/// meant — see [`store::archive_set`] — and this is Verkstead reaching for it on
/// their behalf, because it knows something they cannot see: the session that
/// asked is not there any more.
///
/// Every one of them rather than the newest. One Worktree holds one agent, so
/// more than one open at once is unusual, but they were all orphaned by the same
/// death and none of them has anybody waiting on it.
///
/// Nothing is refused for. A Set that will not archive is a Set the human can
/// archive themselves from the page it is on, and stopping the relaunch over one
/// would leave the Conversation standing still with the Remedy spent.
async fn orphaned(state: &AppState, conversation_id: i64, timeline: &[store::TimelineEvent]) {
    let mut archived = false;

    for asked in open(timeline) {
        match store::archive_set(&state.pool, &state.settlements, asked).await {
            Ok(store::Archiving::Archived(_)) => {
                archived = true;

                tracing::info!(
                    conversation_id,
                    set_id = asked,
                    "a Question Set the dead grilling left open was archived as orphaned"
                );
            }
            Ok(other) => tracing::info!(
                conversation_id,
                set_id = asked,
                outcome = ?other,
                "a Question Set the dead grilling left open was settled before the relaunch reached it",
            ),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, set_id = asked, "archiving an orphaned Question Set failed");
            }
        }
    }

    // The page the human is looking at is the page the Set was open on, and
    // what has just changed there is that it no longer is. Only where something
    // did change: a nudge is every open page going back to the store.
    if archived {
        state.nudges.announce(Nudge::Set {
            conversation: conversation_id,
        });
    }
}

/// The Sets on the Timeline that are still waiting on the human.
fn open(timeline: &[store::TimelineEvent]) -> Vec<i64> {
    timeline
        .iter()
        .filter_map(|event| match &event.event {
            store::Event::QuestionSet(asked) => asked.settlement.is_none().then_some(asked.set_id),
            _ => None,
        })
        .collect()
}

/// The Brief the Conversation started from, which is what a grilling is a
/// grilling of.
///
/// The first, as [`crate::conversations::documents`] reads it: a Brief is frozen
/// the moment the Conversation moves out of Draft, and the one on the Timeline
/// is the one the dead session was primed with.
fn brief(timeline: &[store::TimelineEvent]) -> String {
    timeline
        .iter()
        .find_map(|event| match &event.event {
            store::Event::Brief(markdown) => Some(markdown.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Everything the human has already answered, as one markdown document in the
/// order it was asked.
///
/// Answered Sets only. One archived unanswered is one nobody ever replied to —
/// including, now, the one this very relaunch archived — and a heading over
/// nothing would tell the new session that something had been said.
///
/// Empty where nothing has been answered, which is a grilling that died before
/// its first Set came back. What that leaves is the Brief alone, which is where
/// every grilling starts.
fn settled(timeline: &[store::TimelineEvent]) -> String {
    let mut digest = String::new();

    for event in timeline {
        let store::Event::QuestionSet(asked) = &event.event else {
            continue;
        };

        let Some(store::Settlement::Answered(answered)) = &asked.settlement else {
            continue;
        };

        digest.push_str(&exchange(&asked.set, &answered.response));
    }

    digest
}

/// One Set and its Answers: what it was called, each question against what
/// became of it, and whatever the human said about the whole of it.
///
/// The agent's own markdown, kept as it was written. What this is going into is
/// a prompt rather than a table on a phone, so the question that was asked with
/// a code block in it is worth having with the code block still in it.
fn exchange(set: &QuestionSet, response: &Response) -> String {
    let mut said = format!("## {}\n\n", set.title.trim());

    for question in &set.questions {
        said.push_str(&format!(
            "**{}** {}\n\n",
            question.name(),
            question.text.trim()
        ));

        // A Heading asks nothing of its own — it heads its Sub-questions — so no
        // Answer ever comes back for one, and nothing is written under it.
        if !question.heading() {
            said.push_str(&format!(
                "{}\n\n",
                decided(response, question.name(), &question.options)
            ));
        }

        for subquestion in &question.subquestions {
            let name = subquestion.name(question);

            said.push_str(&format!(
                "**{name}** {}\n\n{}\n\n",
                subquestion.text.trim(),
                decided(response, &name, &subquestion.options)
            ));
        }
    }

    if let Some(comment) = response
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|comment| !comment.is_empty())
    {
        said.push_str(&format!("**About the Set as a whole** {comment}\n\n"));
    }

    said
}

/// What became of one question: the Option that was chosen, whatever the human
/// wrote, or both.
///
/// The Timeline's own reading of an Answer — see `verkstead_render`'s — with the
/// markdown left in and the empty case spoken aloud. Both differences are the
/// reader: that one is a table a human skims, and this is a paragraph an agent
/// is being brought up to speed by.
fn decided(response: &Response, name: &str, options: &[QuestionOption]) -> String {
    let Some(answer) = response
        .answers
        .iter()
        .find(|answer| answer.label.trim() == name)
    else {
        return LEFT_OPEN.to_owned();
    };

    let chosen = answer
        .selected
        .and_then(|n| options.iter().find(|option| option.n == n))
        .map(|option| option.text.trim());

    let wrote = answer
        .free_text
        .as_deref()
        .map(str::trim)
        .filter(|wrote| !wrote.is_empty());

    // Both where the human picked an Option and said why, which is the ordinary
    // shape of an Answer that carries a qualification.
    match (chosen, wrote) {
        (Some(chosen), Some(wrote)) => format!("{chosen} — {wrote}"),
        (Some(only), None) | (None, Some(only)) => only.to_owned(),
        (None, None) => LEFT_OPEN.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Set with one of everything a digest has to carry: a Question answered
    /// by an Option, one answered in the human's own words beside the Option
    /// they picked, one left open, and a Heading over a Sub-question.
    const ASKED: &str = r#"
title: How the limiter counts
questions:
  - label: Q1
    text: Per key or per address?
    options:
      - n: 1
        text: Per key
      - n: 2
        text: Per address
  - label: Q2
    text: Where does the counter live?
    options:
      - n: 1
        text: In process
      - n: 2
        text: In Redis
  - label: Q3
    text: What happens when it trips?
    options:
      - n: 1
        text: 429
  - label: Q4
    text: The window
    subquestions:
      - letter: a
        text: How long is it?
        options:
          - n: 1
            text: A minute
"#;

    /// And the human's answers to it, in the four shapes an Answer comes in.
    const ANSWERED: &str = r#"
answers:
  - label: Q1
    selected: 1
  - label: Q2
    selected: 1
    free_text: until it needs to survive a restart
  - label: Q3
    unanswered: true
  - label: Q4a
    free_text: whatever the client's plan says
comment: none of this is settled about the burst allowance
"#;

    /// The Set as the store holds it on a Timeline, settled however the caller
    /// says — or still waiting on the human, where they say nothing.
    fn on_timeline(
        id: i64,
        set_id: i64,
        settlement: Option<store::Settlement>,
    ) -> store::TimelineEvent {
        store::TimelineEvent {
            id,
            at: "2026-08-23T12:00:00Z".to_owned(),
            event: store::Event::QuestionSet(Box::new(store::SetOnTimeline {
                set_id,
                set: QuestionSet::from_yaml(ASKED).expect("the example Set parses"),
                settlement,
            })),
        }
    }

    /// Answered, which is the settlement a digest is made of.
    fn answered(set_id: i64) -> store::Settlement {
        store::Settlement::Answered(store::StoredResponse {
            set_id,
            submitted_at: "2026-08-23T12:05:00Z".to_owned(),
            response: Response::from_yaml(ANSWERED).expect("the example Response parses"),
        })
    }

    /// What the new session is brought up to speed by: every question it has
    /// already been round, against what the human decided.
    #[test]
    fn the_digest_carries_each_question_against_what_was_decided() {
        let set = QuestionSet::from_yaml(ASKED).unwrap();
        let response = Response::from_yaml(ANSWERED).unwrap();

        let digest = exchange(&set, &response);

        assert!(
            digest.contains("## How the limiter counts"),
            "the Set is named, so a digest of several reads as several: {digest}"
        );
        assert!(
            digest.contains("**Q1** Per key or per address?\n\nPer key\n"),
            "an Option picked is the Option's own words: {digest}"
        );
        assert!(
            digest.contains("In process — until it needs to survive a restart"),
            "and what they wrote beside it goes with it, being the qualification: {digest}"
        );
        assert!(
            digest.contains("**Q4a** How long is it?\n\nwhatever the client's plan says"),
            "a Sub-question is asked and answered like any other: {digest}"
        );
        assert!(
            digest.contains("none of this is settled about the burst allowance"),
            "and what they said about the Set as a whole is said about it: {digest}"
        );
    }

    /// A question the human deliberately left open is worth more than a blank:
    /// the new grilling may ask it again, and this is what says it may.
    #[test]
    fn a_question_left_open_says_so() {
        let set = QuestionSet::from_yaml(ASKED).unwrap();
        let response = Response::from_yaml(ANSWERED).unwrap();

        let digest = exchange(&set, &response);

        assert!(
            digest.contains("**Q3** What happens when it trips?\n\n_Left open._"),
            "the human saw it and settled nothing: {digest}"
        );
        assert!(
            digest.contains("**Q4** The window\n\n**Q4a**"),
            "and a Heading asks nothing of its own, so nothing is written under it: {digest}"
        );
    }

    /// Answered Sets, in the order they were asked, and nothing else. One
    /// nobody ever replied to is one nothing was said about — including the one
    /// the relaunch has just archived on its way past.
    #[test]
    fn only_what_was_answered_reaches_the_digest() {
        let timeline = vec![
            on_timeline(1, 11, Some(answered(11))),
            on_timeline(2, 12, None),
            on_timeline(
                3,
                13,
                Some(store::Settlement::ArchivedUnanswered(store::SetArchived {
                    set_id: 13,
                    archived_at: "2026-08-23T12:06:00Z".to_owned(),
                })),
            ),
        ];

        assert_eq!(
            settled(&timeline)
                .matches("## How the limiter counts")
                .count(),
            1,
            "one of the three was answered, and it is the one that is said",
        );
        assert_eq!(
            open(&timeline),
            vec![12],
            "and the one still waiting on the human is the one to archive as orphaned",
        );
    }

    /// A grilling that died before its first Set came back has nothing to say,
    /// and says nothing. What that leaves is the Brief alone — see
    /// [`skills::grilling_again`].
    #[test]
    fn a_conversation_with_nothing_answered_yields_no_digest() {
        let timeline = vec![store::TimelineEvent {
            id: 1,
            at: "2026-08-23T12:00:00Z".to_owned(),
            event: store::Event::Brief("# Rate limiting\n".to_owned()),
        }];

        assert_eq!(settled(&timeline), "");
        assert_eq!(brief(&timeline), "# Rate limiting\n");
        assert!(open(&timeline).is_empty());
    }
}
