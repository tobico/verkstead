//! The Question Set: what an agent sends and what the store hands back.
//!
//! The types mirror the YAML wire format one-to-one. They are deliberately
//! permissive about the *shape* of a Set — anything the grammar forbids but
//! serde can represent is caught by [`QuestionSet::validate`] instead, so the
//! refusal can name the offending Question rather than a byte offset.

use serde::{Deserialize, Serialize};

// The direction an agent recommends is drawn by the viewer as well as written by
// an agent, so its TypeScript comes from here — see `Liveness`.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// A batch of Questions submitted together by one agent.
///
/// `title`, `preface`, `questions` and `postscript` come from the agent;
/// `project`, `branch` and `diff` are filled in by the CLI, which derives them
/// from the working directory rather than trusting the agent. The server treats
/// all three as opaque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionSet {
    /// Short line for the row this Set gets on its Conversation's Timeline.
    pub title: String,

    /// Markdown context, enough to answer the Questions without seeing the
    /// agent's session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preface: Option<String>,

    pub questions: Vec<Question>,

    /// Markdown the agent closes the Set with, drawn above the set-level comment
    /// box: suggested discussion topics, or whatever else the human might take
    /// up there. Not a Question — a blank comment beneath it means nothing to
    /// add, never Unanswered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postscript: Option<String>,

    /// The wrap-up proposal this Set carries, on the one Set that carries one.
    ///
    /// What makes a Set the grilling's closing move rather than another round of
    /// it: absent on every ordinary Set, and present only on the one that
    /// proposes the work move on. Answering a Set that carries it is what takes
    /// the Conversation out of Grilling.
    ///
    /// A block of its own rather than a convention read off a label or a title,
    /// so that recognising one is a field being there and nothing subtler. The
    /// human never sees it: what they read is the Questions, and the Preface
    /// that leads into them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<Proposal>,

    /// The self-review this Set carries, on the one Set a wrap-up's review asks.
    ///
    /// What makes a Set the review's findings rather than any other question a
    /// session asked: absent on every ordinary Set, present on the one that puts
    /// a review to the human. Answering a Set that carries it is what turns the
    /// findings into work — and what settles the review as one of the things
    /// wrap-up waits on.
    ///
    /// A block of its own for the reason [`Self::proposal`] is one: recognising
    /// it is a field being there and nothing subtler. The human never sees it
    /// either — what they read is the Questions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<Review>,

    /// Repository the agent is working in, as the CLI saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Branch the agent is working on, as the CLI saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// The repo's uncommitted changes at send time. Absent on a clean tree or
    /// outside a repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

/// The grilling agent's closing move: that the work is understood well enough
/// to build, and how it recommends building it.
///
/// The recommendation travels as data rather than only as prose, because the
/// workbench draws the chooser with the recommended direction marked — a
/// rationale the human reads is not a thing a radio button can be checked from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    /// Which of the three the agent recommends.
    pub direction: Direction,

    /// Why, as markdown. What the chooser shows beside the recommendation, so
    /// the human is deciding against the agent's reasoning rather than against
    /// a bare word.
    pub rationale: String,

    /// The Option that means *go ahead*, in the Guide's own notation: `Q14.1`
    /// for a Question's, `Q14a.1` for a Sub-question's.
    ///
    /// What lets the human disagree. Verkstead has to know whether a proposal
    /// was accepted, and it cannot read that off wording the agent chose — so
    /// the agent names the Option it will take as acceptance, and every other
    /// way of answering leaves the Conversation grilling: another Option, an
    /// answer in the human's own words, or the question left open.
    ///
    /// That is the whole way back. A grilling ends only when the human says so
    /// in the one place they are already looking, and the session that proposed
    /// is still holding the thread when they don't — it takes their Response
    /// and decides for itself whether to keep grilling or propose again.
    pub accepted_by: String,
}

/// One Option, by the name of the question that offers it and its number.
///
/// The Guide's `Q14.1` taken apart. Whitespace-trimmed either side of the dot,
/// because the label a Question answers to is trimmed everywhere else too.
fn option_named(label: &str) -> Option<(&str, u32)> {
    let (name, n) = label.trim().rsplit_once('.')?;
    let name = name.trim();

    if name.is_empty() {
        return None;
    }

    Some((name, n.trim().parse().ok()?))
}

impl Proposal {
    /// The Option this proposal is accepted by, or `None` where `accepted_by`
    /// is not in the notation at all.
    pub fn acceptance(&self) -> Option<(&str, u32)> {
        option_named(&self.accepted_by)
    }

    /// Whether this Response accepts the proposal.
    ///
    /// Only the named Option being selected is acceptance. Free text *beside*
    /// it is the qualification the Guide says it is — the agent reads it, and
    /// the work goes ahead — but free text *instead* of it is an answer of the
    /// human's own, which wins over the Options offered and is therefore not
    /// the one this names. An Unanswered question is never acceptance, exactly
    /// as the Guide says of every other question.
    pub fn accepted(&self, response: &crate::response::Response) -> bool {
        let Some((name, n)) = self.acceptance() else {
            return false;
        };

        response.answers.iter().any(|answer| {
            answer.label.trim() == name && !answer.unanswered && answer.selected == Some(n)
        })
    }
}

/// The wrap-up review's closing move: what it found, and which Answer to each
/// finding means *fix this*.
///
/// One finding per Question, and the two halves of a finding are written for two
/// different readers. The Question is what the human decides from, on a phone;
/// [`Finding::what`] is what the fix session is told, and it is the only thing
/// that session gets. A review that wrote one for both would be asking a phone
/// screen to hold a brief for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// What the review found, in the order it asked about them.
    pub findings: Vec<Finding>,
}

/// One thing the review found, as the Set carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    /// The Option that means *fix this*, in the Guide's own notation: `Q1.1`
    /// for a Question's, `Q1a.1` for a Sub-question's.
    ///
    /// What lets the human decline. Verkstead cannot read acceptance off wording
    /// the agent chose, so the agent names the Option it will take as *fix it* —
    /// and every other way of answering dispatches nothing at all.
    pub fix: String,

    /// The finding as the fix session is told it, as markdown.
    ///
    /// Written for an agent that has not read the diff and will never speak to
    /// the reviewer: where it is, what is wrong, and what done would look like.
    /// Whatever the human wrote alongside their Answer goes with it.
    pub what: String,
}

impl Finding {
    /// The Option this finding is fixed by, or `None` where [`Finding::fix`] is
    /// not in the notation at all.
    pub fn fixing(&self) -> Option<(&str, u32)> {
        option_named(&self.fix)
    }

    /// Whether this Response says to fix it.
    ///
    /// Read exactly as a proposal's acceptance is, and for the same reason: only
    /// the named Option being selected is a yes. Free text *beside* it is the
    /// qualification the Guide says it is, and it travels to the fix session
    /// with the finding; free text *instead* of it is an answer of the human's
    /// own, which wins over the Options offered. An Unanswered question is never
    /// acceptance.
    pub fn accepted(&self, response: &crate::response::Response) -> bool {
        let Some((name, n)) = self.fixing() else {
            return false;
        };

        response.answers.iter().any(|answer| {
            answer.label.trim() == name && !answer.unanswered && answer.selected == Some(n)
        })
    }

    /// What the human wrote alongside their Answer to this finding, trimmed.
    ///
    /// Empty where they wrote nothing, which is the ordinary way of agreeing
    /// with a recommendation.
    pub fn said<'a>(&self, response: &'a crate::response::Response) -> &'a str {
        let Some((name, _)) = self.fixing() else {
            return "";
        };

        response
            .answers
            .iter()
            .find(|answer| answer.label.trim() == name)
            .and_then(|answer| answer.free_text.as_deref())
            .unwrap_or_default()
            .trim()
    }
}

/// One of the three ways the work can be built.
///
/// Named on the wire in the words the design uses for them, so a Set is
/// readable as written: `inline`, `task-list`, `roadmap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    /// One fresh session under the implementation profile, primed with the
    /// handoff document the grilling session writes.
    Inline,

    /// Broken into `.tasks/`, one fresh session per task.
    TaskList,

    /// Staged under `docs/roadmaps/`, a feature per stage.
    Roadmap,
}

/// A single labelled decision put to the human.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Agent-supplied and opaque to the server (e.g. `Q7`). Only the agent
    /// knows its session counter, so only the agent can assign it.
    pub label: String,

    pub text: String,

    /// The axes this question's Options are compared along, one per column of
    /// the Answer Table, as inline markdown. Declaring any is what makes the
    /// Options a table rather than a list; a question without them is drawn as
    /// the list it always was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<QuestionOption>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subquestions: Vec<Subquestion>,
}

/// A leaf Question nested one level under a [`Question`], labelled by letter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subquestion {
    /// Appended to the parent's label to name this question: `Q7` + `a`.
    pub letter: String,

    pub text: String,

    /// The axes this Sub-question's Options are compared along — a Sub-question
    /// declares its own Answer Table exactly as a [`Question`] does, and never
    /// inherits its parent's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<QuestionOption>,

    /// Always empty: Sub-questions are leaves. The field exists only so that a
    /// third level of nesting reaches validation and can be refused by name,
    /// instead of failing as an unknown field at some line and column.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subquestions: Vec<Subquestion>,
}

/// One discrete choice offered on a Question or Sub-question.
///
/// The domain calls this an Option; the name is taken in Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionOption {
    /// The number the human selects, `1`, `2`, …
    pub n: u32,

    pub text: String,

    /// Whether this Option is the Recommendation. At most one per question.
    #[serde(default, skip_serializing_if = "is_false")]
    pub recommended: bool,

    /// This Option's row of the Answer Table: one cell per axis the question
    /// declared, in that order, as inline markdown. Empty on a question that
    /// declared no `columns`, which is every question that is not a table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<String>,
}

pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

impl QuestionSet {
    /// Parse a Set from the YAML wire format.
    ///
    /// This checks that the document *is* a Set, not that it is a legal one —
    /// see [`QuestionSet::validate`].
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_saphyr::Error> {
        serde_saphyr::from_str(yaml)
    }

    /// Render the Set back to YAML. Multi-line strings — the Preface, the
    /// Postscript, the Diff — come out as `|` block scalars.
    pub fn to_yaml(&self) -> Result<String, serde_saphyr::SerializeError> {
        serde_saphyr::to_string(self)
    }
}

impl Question {
    /// The name this Question answers to: its label.
    pub fn name(&self) -> &str {
        &self.label
    }

    /// Whether this Question is a Heading: Sub-questions under it, and no
    /// Options of its own, so its text heads them rather than asking anything.
    /// No Answer comes back for one.
    ///
    /// Read off the shape rather than declared, because the shape already says
    /// it: a Question with Options is answerable whatever else it carries, and
    /// one with neither Options nor Sub-questions is a bare clarifying Question
    /// whose Answer is whatever the human writes. That leaves exactly one
    /// arrangement with nothing to answer at this level, and it is this one.
    pub fn heading(&self) -> bool {
        self.options.is_empty() && !self.subquestions.is_empty()
    }
}

impl Subquestion {
    /// The name this Sub-question answers to, e.g. `Q7a`.
    pub fn name(&self, parent: &Question) -> String {
        format!("{}{}", parent.label.trim(), self.letter.trim())
    }
}
