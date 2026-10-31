//! Which of the two an arrival's session turned out to be — the harness's own
//! resume of the conversation that crossed, or Verkstead's own Resume — said on
//! the Timeline (ADR-0020, *Transfer*).
//!
//! **Because the human sees one press and two outcomes.** *Transfer to…* lands
//! the work on another machine and something starts there; what starts is either
//! a **Carried Conversation**, which picks up mid-sentence, or Verkstead's own
//! **Resume**, which starts the state again from the record. Both are complete
//! ways to take up work that has arrived and neither is a failure — and from the
//! outside they are one session appearing on the Timeline. So each of them says
//! which it was, and the second says why it was not the first.
//!
//! **Every reason is a real situation rather than an error**, which is why each
//! is a sentence of its own rather than one about a resume that did not happen:
//! a Profile that shares no memory carried no store, a Profile changed under the
//! Conversation left a record no other backend can read, a Conversation moved
//! before it had ever run a session has nothing to carry on. Nothing about any of
//! them is the human's to put right, and nothing waits on one: whatever the
//! arrival takes the work up with is already on its way by the time the Notice is
//! written.
//!
//! **Where it is written is where it is known**, and the six reasons are known
//! in three places. The arrival knows there is no session on the record, before
//! anything is launched — see [`crate::peer::transfers`]. The launch knows the
//! other four, the last of them only once the memory sync it runs has had
//! its say — see [`crate::sessions::continuing`]. And the refusal is known only
//! after the harness has been asked and has said no, which is the relay reading
//! the ending of a session that never added a line to the record it was resuming
//! — see [`crate::sessions::relay`].

use sqlx::SqlitePool;

use crate::nudge::Nudges;
use crate::store::{self, AgentType};

/// What left a conversation standing to be carried on: the work arriving from
/// another machine, or an answer that widened the sandbox the agent was waiting
/// in (ADR-0028, *The carrying-on is the Carried Conversation's, on this
/// device*).
///
/// The machinery is one and the occasion is what the human is told: an arrival
/// says where the work came from, and an answer says which question set ended
/// the session to add to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occasion {
    /// A Transfer landed the work here.
    Arrived,

    /// Answering this Set added to the sandbox, so the session waiting on it was
    /// ended to be carried on in a wider one.
    Widened(i64),

    /// This Set of Verkstead's own, asked for the session's call to move the
    /// work, was answered *Keep it here*: the session that called was ended for
    /// the move and is carried on where it is (ADR-0029).
    Kept(i64),
}

impl Occasion {
    /// The occasion a row of the record names — see
    /// [`store::Continued::widened_by`].
    pub(crate) fn of(continued: &store::Continued) -> Self {
        match (continued.widened_by, continued.kept_here_by) {
            (Some(set), _) => Self::Widened(set),
            (None, Some(set)) => Self::Kept(set),
            (None, None) => Self::Arrived,
        }
    }

    /// The opening clause both outcomes say it in.
    fn opening(self) -> String {
        match self {
            Self::Arrived => "This conversation arrived part way through".to_owned(),
            Self::Widened(set) => format!(
                "Answering question set {set} added to the sandbox, so the session waiting on \
                 it was ended part way through"
            ),
            Self::Kept(set) => format!(
                "Question set {set} kept the work here, so the session that asked to move it was \
                 ended part way through"
            ),
        }
    }
}

/// Why the session started for an arriving Conversation is Verkstead's own
/// Resume rather than the harness's resume of the conversation that crossed.
///
/// One variant per real situation, each carrying what its sentence names. None
/// of them is an error and none of them is refused for: the session starts
/// either way, and what this settles is what the Timeline says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Instead {
    /// The Conversation arrived before it had ever run a session — or its
    /// sessions ran on a backend that named none and whose own id was never
    /// written down. Known at the arrival, there being nothing to carry on.
    NoSession,

    /// The Pairing this session runs under is another harness than the one that
    /// conversation was had on, a Profile having been changed under the
    /// Conversation.
    AnotherHarness {
        /// The harness whose record crossed.
        ran: AgentType,

        /// And the one this session runs on, which cannot read it.
        running: AgentType,
    },

    /// Verkstead has no resume line for that harness. None of the four is this
    /// today, and a fifth landing without one is.
    NoResumeLine(AgentType),

    /// The Profile's memory switch is off, so nothing of the harness's store
    /// travelled with the work: a session that runs from a copy starts new exactly
    /// as it does where the Profile is bound, and this is that rule reaching the
    /// arrival.
    MemoryOff(AgentType),

    /// No record where the memory sync should have left one — the part came over
    /// empty, the Worktree's stem came out differently on this device, or the
    /// relocation could not name a destination on it.
    NoRecord(AgentType),

    /// The harness was asked and would not: the one reason known after the
    /// launch rather than before it.
    Refused {
        /// Which harness said no.
        harness: AgentType,

        /// And the session it was told to resume, which is the name the record
        /// knows that conversation by.
        session: String,
    },
}

impl Instead {
    /// The Notice, whole.
    ///
    /// Two sentences either way: what was started, and why it was that rather
    /// than the other. The reason is the clause [`Instead::because`] writes, and
    /// the frame around it is the same for all of them but the first — a
    /// Conversation with no session on its record arrived part way through
    /// nothing, so there is no conversation to name and no harness to name it by.
    fn saying(&self, occasion: Occasion) -> String {
        let Some(harness) = self.harness() else {
            return match occasion {
                Occasion::Arrived => {
                    "This conversation arrived with no session on its record to carry on from. \
                     What starts here is Verkstead's own resume, primed from this timeline."
                        .to_owned()
                }
                Occasion::Widened(set) => format!(
                    "Answering question set {set} added to the sandbox, and there's no session on \
                     the record to carry on from. What starts now is Verkstead's own resume, \
                     primed from this timeline."
                ),
                Occasion::Kept(set) => format!(
                    "Question set {set} kept the work here, and there's no session on the record \
                     to carry on from. What starts now is Verkstead's own resume, primed from \
                     this timeline."
                ),
            };
        };

        format!(
            "{opening} a conversation with {harness}. The session here is Verkstead's own resume, \
             started again from this timeline, because {because}.",
            opening = occasion.opening(),
            harness = harness.word(),
            because = self.because(occasion),
        )
    }

    /// The clause that says which situation this was — in the middle of the
    /// sentence above, and the whole of what the log says.
    fn because(&self, occasion: Occasion) -> String {
        match self {
            Self::NoSession => "there's no session on the record to carry on from".to_owned(),

            Self::AnotherHarness { ran, running } => format!(
                "that conversation was on {ran} and this session runs on {running}, and one \
                 harness can't read another's record",
                ran = ran.word(),
                running = running.word(),
            ),

            Self::NoResumeLine(harness) => format!(
                "Verkstead doesn't carry a conversation on from a {harness} session",
                harness = harness.word(),
            ),

            Self::MemoryOff(harness) => format!(
                "this agent profile doesn't share its account's memory, so {harness}'s record of \
                 that conversation didn't travel with the work",
                harness = harness.word(),
            ),

            Self::NoRecord(harness)
                if matches!(occasion, Occasion::Widened(_) | Occasion::Kept(_)) =>
            {
                format!(
                    "{harness}'s record of that conversation isn't where {harness} looks for it \
                     under this worktree's path",
                    harness = harness.word(),
                )
            }

            Self::NoRecord(harness) => format!(
                "{harness}'s record of that conversation isn't on this device under this \
                 worktree's path. The memory sync carried none of it, or it landed where \
                 {harness} won't look",
                harness = harness.word(),
            ),

            Self::Refused { harness, session } => format!(
                "{harness} wouldn't resume session `{session}`",
                harness = harness.word(),
            ),
        }
    }

    /// Which harness this is about, where it is about one. `None` is
    /// [`Instead::NoSession`] alone, there being no conversation and so no
    /// harness that had it — which is also what gives that one a sentence of its
    /// own above.
    fn harness(&self) -> Option<AgentType> {
        match self {
            Self::NoSession => None,
            Self::AnotherHarness { ran, .. } => Some(*ran),
            Self::NoResumeLine(harness)
            | Self::MemoryOff(harness)
            | Self::NoRecord(harness)
            | Self::Refused { harness, .. } => Some(*harness),
        }
    }

    /// And the same reason for the log, where every one of these is said too — a
    /// server's operator reads the log and the human reads the Timeline.
    pub(crate) fn logged(&self, occasion: Occasion) -> String {
        self.because(occasion)
    }
}

/// Say on `conversation_id`'s Timeline that what was started for it is
/// Verkstead's own Resume, and which of the reasons it was.
///
/// Written whether or not anybody is looking and never refused for: what the
/// arrival is taking the work up with is settled by the time this is called, and
/// a Notice that could not be written costs the telling rather than the work.
///
/// **Which is not to say a session is running.** Two of the three callers are in
/// front of one — the arrival that found no session on the record, and the launch
/// that found it could not resume — and what they are saying is what the session
/// they are about to start, or have just declined to make a resume of, is going to
/// be. The third reads it off a session that has ended.
pub(crate) async fn instead(
    pool: &SqlitePool,
    nudges: &Nudges,
    conversation_id: i64,
    occasion: Occasion,
    why: &Instead,
) {
    tracing::info!(
        conversation_id,
        ?occasion,
        "the conversation this Conversation was part way through is not being carried on, so \
         what was started for it is Verkstead's own Resume: {}",
        why.logged(occasion),
    );

    noted(pool, nudges, conversation_id, &why.saying(occasion)).await;
}

/// And the other outcome: the session started here *is* the harness's resume of
/// the conversation that crossed.
///
/// Said for the reason the refusals are said. A resumed agent picks up
/// mid-sentence and a re-primed one starts again from the record, and the human
/// pressing *Transfer to…* sees a session either way — so the one that worked
/// says so as plainly as the ones that did not.
///
/// **And said once there is a process**, unlike any of them: this is the only one
/// of the two outcomes that a launch can still fail its way out of after it has
/// been decided on, so it is written where a session is certain rather than where
/// the decision was made. See [`crate::sessions::continuing`], which decides it.
pub(crate) async fn carried_on(
    pool: &SqlitePool,
    nudges: &Nudges,
    conversation_id: i64,
    occasion: Occasion,
    harness: AgentType,
    session: &str,
) {
    let from = match occasion {
        Occasion::Arrived => " from the record the memory sync brought over",
        Occasion::Widened(_) => " in the wider sandbox",
        Occasion::Kept(_) => "",
    };

    noted(
        pool,
        nudges,
        conversation_id,
        &format!(
            "{opening} a conversation with {harness}, and the session started here carries it \
             on. Verkstead told {harness} to resume session `{session}`{from}.",
            opening = occasion.opening(),
            harness = harness.word(),
        ),
    )
    .await;
}

/// One sentence on the Conversation's Timeline, and the pages told it is there.
///
/// [`crate::mirroring::memory`]'s own write beside the sync these are mostly
/// about: a Notice rather than a refusal, every time.
async fn noted(pool: &SqlitePool, nudges: &Nudges, conversation_id: i64, line: &str) {
    match store::note(pool, conversation_id, line).await {
        Ok(_) => nudges.announce(verkstead_schema::Nudge::Conversation {
            conversation: conversation_id,
        }),

        Err(error) => tracing::error!(
            error = ?error,
            conversation_id,
            "saying on the Timeline which resume an arriving Conversation was taken up with failed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each reason names itself, and names the harness it is about: what the
    /// human is reading is why the session in front of them is not the one that
    /// was mid-sentence on the other machine.
    #[test]
    fn every_reason_says_which_one_it_was() {
        let said = Instead::MemoryOff(AgentType::Claude).saying(Occasion::Arrived);

        assert!(
            said.contains("doesn't share its account's memory"),
            "the memory switch is named: {said:?}",
        );
        assert!(
            said.contains("Verkstead's own resume"),
            "and what was started instead: {said:?}",
        );

        let said = Instead::AnotherHarness {
            ran: AgentType::Claude,
            running: AgentType::Codex,
        }
        .saying(Occasion::Arrived);

        assert!(
            said.contains("claude") && said.contains("codex"),
            "both harnesses are named, the record's and the session's: {said:?}",
        );

        let said = Instead::NoRecord(AgentType::Grok).saying(Occasion::Arrived);

        assert!(
            said.contains("grok") && said.contains("isn't on this device"),
            "the record that is not here is named by the harness that keeps it: {said:?}",
        );

        let said = Instead::Refused {
            harness: AgentType::OpenCode,
            session: "ses_01".to_owned(),
        }
        .saying(Occasion::Arrived);

        assert!(
            said.contains("opencode wouldn't resume session `ses_01`"),
            "and a refusal names the session it was refused for: {said:?}",
        );
    }

    /// And the one with no conversation behind it says that instead of naming a
    /// harness that never ran.
    #[test]
    fn a_conversation_with_no_session_says_there_was_nothing_to_carry_on() {
        let said = Instead::NoSession.saying(Occasion::Arrived);

        assert!(
            said.contains("no session on its record to carry on from"),
            "the reason is the absence itself: {said:?}",
        );
        assert!(
            !said.contains("claude") && !said.contains("an agent"),
            "and no harness is named, there being no conversation to have had one: {said:?}",
        );
    }

    /// **An answer that widened the sandbox names its question set** rather than
    /// a move: the human answered on this device and nothing arrived (ADR-0028).
    #[test]
    fn a_widened_sandbox_names_the_question_set_rather_than_a_move() {
        let said = Instead::Refused {
            harness: AgentType::Claude,
            session: "ses_01".to_owned(),
        }
        .saying(Occasion::Widened(7));

        assert!(
            said.contains("Answering question set 7 added to the sandbox"),
            "the question set is named: {said:?}",
        );
        assert!(
            said.contains("claude wouldn't resume session `ses_01`"),
            "and the reason as a move says it: {said:?}",
        );
        assert!(
            !said.contains("arrived"),
            "and no move is claimed: {said:?}"
        );

        let said = Instead::NoSession.saying(Occasion::Widened(7));

        assert!(
            said.contains("question set 7") && said.contains("no session on the record"),
            "a widened sandbox with nothing to carry on says both: {said:?}",
        );
    }

    /// **And *Keep it here* names its question set and keeps the work where it
    /// is**: nothing moved and nothing arrived (ADR-0029).
    #[test]
    fn keeping_the_work_here_names_the_question_set_and_no_move() {
        let said = Instead::NoRecord(AgentType::Claude).saying(Occasion::Kept(9));

        assert!(
            said.contains("Question set 9 kept the work here"),
            "the question set is named: {said:?}",
        );
        assert!(
            said.contains("isn't where claude looks for it"),
            "and the record is looked for where the work never left: {said:?}",
        );
        assert!(
            !said.contains("arrived") && !said.contains("memory sync"),
            "and no move is claimed: {said:?}"
        );
    }
}
