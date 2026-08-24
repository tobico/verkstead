//! Deferred Asks on this end: folding the Answers to one into the prompt of the
//! next session started on its Conversation.
//!
//! A Deferred Ask idles nobody, so the session that asked it is long finished by
//! the time the human answers — often days finished. What the Answers are *for*
//! is the work that comes after, and the one thing every session is certain to
//! read is the prompt it was started on. So they go there, under the documents
//! the prompt is built from: the Brief says what the work is, and this is newer
//! and less general than the Brief, which is exactly where everything written
//! under the documents goes and for the same reason.
//!
//! **Folded once and never again**, which is why the folding is written down
//! rather than worked out from what happens to be answered — see
//! [`store::record_folded`]. And written down only once a session has actually
//! been started on the prompt, so a launch that came to nothing does not cost
//! the human the one session their Answers were folded into.
//!
//! Where this is *not* done is the sessions that are not building anything: a
//! Manual Task, whose prompt is the instruction the human typed and nothing else
//! (see [`crate::manual`]), and a relaunched grilling, which is already primed
//! with every Set the Conversation has answered (see [`crate::grillings`]).
//! Folding into either would be an Answer spent on a session it was not meant
//! for.

use sqlx::SqlitePool;

use crate::exchanges::exchange;
use crate::store;

/// The Answers waiting to be folded into a prompt, and the Sets they came from.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Folding {
    /// The exchanges as one markdown document, oldest first. Empty where there
    /// is nothing to fold, which is the ordinary case.
    digest: String,

    /// The Sets it was made of, to be recorded as folded once a session is
    /// running on the prompt they went into.
    sets: Vec<i64>,
}

/// What this Conversation has answered on its Deferred Asks that no session has
/// been told about yet.
///
/// A read that fails is nothing to fold rather than a session that does not
/// start: the Answers stay unfolded and reach the session after this one, which
/// is late where losing the launch would be worse.
pub(crate) async fn unfolded(pool: &SqlitePool, conversation_id: i64) -> Folding {
    let unfolded = match store::unfolded(pool, conversation_id).await {
        Ok(unfolded) => unfolded,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the answered Deferred Asks to fold into a prompt failed");
            return Folding::default();
        }
    };

    let mut folding = Folding::default();

    for answered in unfolded {
        // A Set this build cannot read has no exchange to write into a prompt:
        // the Questions it was asked with are in a body nothing here can take
        // apart. Passed over entirely — not folded and not recorded as folded —
        // so a build that can read it again still owes the human nothing but the
        // reading. See [`store::Asked`].
        let Some(set) = answered.set.set() else {
            continue;
        };

        folding.digest.push_str(&exchange(set, &answered.response));
        folding.sets.push(answered.set_id);
    }

    folding
}

impl Folding {
    /// The prompt with the Answers under it, or the prompt as it stands where
    /// there is nothing to fold.
    pub(crate) fn under(&self, prompt: &str) -> String {
        crate::skills::folded(prompt, &self.digest)
    }

    /// Record that these Answers have gone into a session's prompt. Called once
    /// there is a session running on it, and not before.
    pub(crate) async fn recorded(&self, pool: &SqlitePool) {
        if self.sets.is_empty() {
            return;
        }

        if let Err(error) = store::record_folded(pool, &self.sets).await {
            tracing::error!(error = ?error, sets = ?self.sets, "recording Deferred Asks as folded into a prompt failed");
        }
    }
}
