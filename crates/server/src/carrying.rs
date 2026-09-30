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
    /// travelled with the work: a session away from home starts new exactly as
    /// it does at home, and this is that rule reaching the arrival.
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
    fn saying(&self) -> String {
        let Some(harness) = self.harness() else {
            return "This Conversation arrived with no session on its record to carry on from, so \
                    whatever is started for it here is Verkstead's own Resume: a session of its \
                    own, primed from this Timeline rather than picking a conversation up mid-turn."
                .to_owned();
        };

        format!(
            "This Conversation arrived part way through a conversation with {harness}, and the \
             session started here is Verkstead's own Resume rather than a resume of it — {because}. \
             It starts the work again from this Timeline rather than picking the conversation up \
             mid-turn.",
            harness = harness.word(),
            because = self.because(),
        )
    }

    /// The clause that says which situation this was — in the middle of the
    /// sentence above, and the whole of what the log says.
    fn because(&self) -> String {
        match self {
            Self::NoSession => "there is no session on the record to carry on from".to_owned(),

            Self::AnotherHarness { ran, running } => format!(
                "that conversation was had on {ran} and this session runs on {running}, and a \
                 record one harness wrote is one no other can read",
                ran = ran.word(),
                running = running.word(),
            ),

            Self::NoResumeLine(harness) => format!(
                "Verkstead does not carry a conversation on from a {harness} session",
                harness = harness.word(),
            ),

            Self::MemoryOff(harness) => format!(
                "this Agent Profile does not share its account's memory, so none of the record \
                 {harness} keeps of that conversation travelled with the work",
                harness = harness.word(),
            ),

            Self::NoRecord(harness) => format!(
                "the record {harness} keeps of that conversation is not on this device under this \
                 device's own name for the Worktree — the memory sync carried none of it, or it \
                 landed somewhere {harness} will not look",
                harness = harness.word(),
            ),

            Self::Refused { harness, session } => format!(
                "{harness} would not resume session `{session}`",
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
    pub(crate) fn logged(&self) -> String {
        self.because()
    }
}

/// Say on `conversation_id`'s Timeline that what was started for it is
/// Verkstead's own Resume, and which of the reasons it was.
///
/// Written whether or not anybody is looking and never refused for: the session
/// is already running by the time this is called, and a Notice that could not be
/// written costs the telling rather than the work.
pub(crate) async fn instead(
    pool: &SqlitePool,
    nudges: &Nudges,
    conversation_id: i64,
    why: &Instead,
) {
    tracing::info!(
        conversation_id,
        "the conversation this Conversation arrived part way through is not being carried on, so \
         what was started for it is Verkstead's own Resume: {}",
        why.logged(),
    );

    noted(pool, nudges, conversation_id, &why.saying()).await;
}

/// And the other outcome: the session started here *is* the harness's resume of
/// the conversation that crossed.
///
/// Said for the reason the refusals are said. A resumed agent picks up
/// mid-sentence and a re-primed one starts again from the record, and the human
/// pressing *Transfer to…* sees a session either way — so the one that worked
/// says so as plainly as the ones that did not.
pub(crate) async fn carried_on(
    pool: &SqlitePool,
    nudges: &Nudges,
    conversation_id: i64,
    harness: AgentType,
    session: &str,
) {
    noted(
        pool,
        nudges,
        conversation_id,
        &format!(
            "This Conversation arrived part way through a conversation with {harness}, and the \
             session started here carries it on: {harness} was told to resume session `{session}`, \
             against the record the memory sync carried over. It picks up where the session on the \
             other device left off rather than starting the work again from this Timeline.",
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
        let said = Instead::MemoryOff(AgentType::Claude).saying();

        assert!(
            said.contains("does not share its account's memory"),
            "the memory switch is named: {said:?}",
        );
        assert!(
            said.contains("Verkstead's own Resume"),
            "and what was started instead: {said:?}",
        );

        let said = Instead::AnotherHarness {
            ran: AgentType::Claude,
            running: AgentType::Codex,
        }
        .saying();

        assert!(
            said.contains("claude") && said.contains("codex"),
            "both harnesses are named, the record's and the session's: {said:?}",
        );

        let said = Instead::NoRecord(AgentType::Grok).saying();

        assert!(
            said.contains("grok") && said.contains("not on this device"),
            "the record that is not here is named by the harness that keeps it: {said:?}",
        );

        let said = Instead::Refused {
            harness: AgentType::OpenCode,
            session: "ses_01".to_owned(),
        }
        .saying();

        assert!(
            said.contains("opencode would not resume session `ses_01`"),
            "and a refusal names the session it was refused for: {said:?}",
        );
    }

    /// And the one with no conversation behind it says that instead of naming a
    /// harness that never ran.
    #[test]
    fn a_conversation_with_no_session_says_there_was_nothing_to_carry_on() {
        let said = Instead::NoSession.saying();

        assert!(
            said.contains("no session on its record to carry on from"),
            "the reason is the absence itself: {said:?}",
        );
        assert!(
            !said.contains("claude") && !said.contains("an agent"),
            "and no harness is named, there being no conversation to have had one: {said:?}",
        );
    }
}
