//! The Response: the human's reply to a whole Question Set, and what the
//! waiting agent finally receives.
//!
//! Like the Set, the types mirror the YAML wire format one-to-one and are
//! permissive about shape. Whether a Response actually resolves the Set it
//! claims to answer is [`Response::validate`]'s job, so a refusal can name the
//! question that was missed rather than a byte offset.

use serde::{Deserialize, Serialize};

use crate::set::{Direction, is_false};

// A Response is what the viewer submits, so its TypeScript is generated from
// this definition along with the rest of the viewer's wire — see the note on
// `verkstead-render`'s `typescript` feature. The attributes are gated because
// the emitter has no business in the browser build.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The submitted collection of Answers and Unanswered markers for one Question
/// Set, plus an optional set-level comment — and, on a Set carrying a proposal,
/// the direction the human picked.
///
/// The invariant is explicitness, not completeness: every Question and
/// Sub-question in the Set appears in `answers` one way or the other, so the
/// agent cannot silently miss a question the human left open. A Response whose
/// every entry is Unanswered carries no Answers at all — with a comment, that
/// is the counter-question that redirects the discussion.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(deny_unknown_fields)]
pub struct Response {
    /// One entry per Question and Sub-question in the Set.
    #[serde(default)]
    pub answers: Vec<Answer>,

    /// What the human wants to say about the Set as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// How the human chose to have the work built, on the one Set that carries
    /// a [`crate::set::Proposal`].
    ///
    /// A field of its own rather than an ordinary Answer, because it answers no
    /// Question: the chooser is the viewer's, injected onto any Set carrying a
    /// proposal, and the three directions it offers are the same three every
    /// time. An Answer here would be a Question the agent never asked and would
    /// have to be excluded from every rule that counts them.
    ///
    /// Picking is the whole of accepting the proposal. Every other way of
    /// answering sends it back — an answer in the human's own words, questions
    /// left open, anything without a pick — so `None` on a proposal Set is the
    /// human disagreeing, and `None` anywhere else is every ordinary Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,

    /// The human saying there is nothing else, on a Set asked while its
    /// Conversation is in Follow-up.
    ///
    /// A field of the Response rather than an Answer for the reason
    /// [`Response::direction`] is one: the control is the viewer's, injected
    /// onto the Set's closing section, and it answers no Question anybody
    /// asked.
    ///
    /// **The agent never sees it.** The mark comes off the Response on the way
    /// into the store and is recorded beside it — see `verkstead_store`'s
    /// `endings` — so what a waiting session is handed is byte for byte what it
    /// would have been handed without one. How a follow-up ends is Verkstead's
    /// business rather than the session's: the agent writes an ordinary
    /// Postscript and reads an ordinary Response, and the ending is entirely
    /// the system's.
    #[serde(default, skip_serializing_if = "is_false")]
    pub nothing_else: bool,
}

/// One question's slot in a Response: either an Answer — a selected Option
/// and/or free text — or the Unanswered marker, saying the human chose to
/// leave the question open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The question this resolves, by the name it answers to: `Q7` for a
    /// Question, `Q7a` for a Sub-question.
    pub label: String,

    /// The number of the Option the human picked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<u32>,

    /// What the human wrote in their own words, alongside or instead of an
    /// Option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub free_text: Option<String>,

    /// Left open on purpose. Exclusive with an Answer: the agent must treat
    /// the question as still open.
    #[serde(default, skip_serializing_if = "is_false")]
    pub unanswered: bool,
}

impl Response {
    /// Parse a Response from the YAML wire format.
    ///
    /// This checks that the document *is* a Response, not that it resolves any
    /// particular Set — see [`Response::validate`].
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_saphyr::Error> {
        serde_saphyr::from_str(yaml)
    }

    /// Render the Response back to YAML. Free text and the comment come out as
    /// `|` block scalars when they run to more than one line.
    pub fn to_yaml(&self) -> Result<String, serde_saphyr::SerializeError> {
        serde_saphyr::to_string(self)
    }
}

impl Answer {
    /// Whether this entry carries an Answer at all — an Option was selected or
    /// something was written. An entry that carries none is either the
    /// Unanswered marker or a mistake.
    pub fn is_answer(&self) -> bool {
        self.selected.is_some()
            || self
                .free_text
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
    }
}
