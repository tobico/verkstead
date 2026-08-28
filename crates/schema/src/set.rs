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
/// `project` and `branch` are filled in by the CLI, which derives them from the
/// working directory rather than trusting the agent, and the server treats both
/// as opaque. The Diff is the server's own — see [`QuestionSet::diffs`].
///
/// Read off the wire through [`Wire`], which is where the one field a Set no
/// longer has is accepted and thrown away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Wire")]
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

    /// Repository the agent is working in, as the CLI saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Branch the agent is working on, as the CLI saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// The uncommitted changes of one repository, on a Set stored before the
    /// Diff became a list of them.
    ///
    /// Nothing writes one again: what carries a Set's Diff now is
    /// [`Self::diffs`], which says which repository each patch came out of. It
    /// is still read, because a Set is a record and the ones already stored have
    /// theirs here — a Set with this and no list is drawn from this, exactly as
    /// it always was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,

    /// The uncommitted changes of every repository the Conversation may write
    /// to: its own first, then each read-write companion. Composed by the server
    /// as the Set arrives — see `verkstead_server::diffs`.
    ///
    /// A list rather than one patch, because work is done in more than one
    /// repository and a patch that did not say which one it came out of is
    /// evidence nobody can place. A repository with nothing uncommitted is not
    /// in it, so an empty list is every worktree clean.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diffs: Vec<RepoDiff>,
}

/// One repository's uncommitted changes, named by the repository they came out
/// of.
///
/// The name is the Repo's registered one — what the workbench calls that
/// repository everywhere else, a commit card's own label included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoDiff {
    pub repo: String,

    /// Whether this is the Conversation's own repository rather than one of its
    /// companions.
    ///
    /// True of exactly one block, and of the first, because that is the order
    /// they are composed in. It says the one thing a name cannot: which of
    /// these repositories the work itself is in. That is what decides whether a
    /// block drawn on its own is labeled — the same question a commit card is
    /// labeled by, asked the same way round.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,

    /// The patch, in the unified format git writes: everything not in the last
    /// commit, staged or not, plus the contents of untracked files.
    pub diff: String,
}

/// A Set as it comes off the wire, which is a Set plus the one key an older
/// agent may still be writing.
///
/// [`QuestionSet`] is deserialized through this rather than directly, so that
/// the strictness and the one exception to it live in the same place: every
/// field a Set has is named here and an unknown one is still refused by name,
/// while `review` — the findings block a Set used to carry — is read and thrown
/// away. Which is the whole of the compatibility this change needs: a Set in
/// flight from an agent that has not been rewritten yet lands as an ordinary
/// Set, and a body stored before the block left the schema is still readable.
/// Nothing ever writes one again, so the exception is one-way and expires by
/// itself.
///
/// **Every field of a Set appears here**, and a field added to one is added to
/// both. Forgetting is loud rather than quiet — a field this does not name is
/// refused as unknown, which every round trip in `tests/question_set.rs` fails
/// on — but it is two lists to keep in step, and that is what the one key costs.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    title: String,

    #[serde(default)]
    preface: Option<String>,

    questions: Vec<Question>,

    #[serde(default)]
    postscript: Option<String>,

    #[serde(default)]
    proposal: Option<Proposal>,

    /// Accepted and thrown away — see the type's own note. Named with the
    /// underscore because it is written to and never read: what it is for is
    /// the key being known, so that a Set carrying one is not refused.
    #[serde(default, rename = "review")]
    _review: Option<serde::de::IgnoredAny>,

    #[serde(default)]
    project: Option<String>,

    #[serde(default)]
    branch: Option<String>,

    #[serde(default)]
    diff: Option<String>,

    #[serde(default)]
    diffs: Vec<RepoDiff>,
}

impl From<Wire> for QuestionSet {
    fn from(wire: Wire) -> Self {
        let Wire {
            title,
            preface,
            questions,
            postscript,
            proposal,
            _review: _,
            project,
            branch,
            diff,
            diffs,
        } = wire;

        Self {
            title,
            preface,
            questions,
            postscript,
            proposal,
            project,
            branch,
            diff,
            diffs,
        }
    }
}

/// The grilling agent's closing move: that the work is understood well enough
/// to build, and how it recommends building it.
///
/// Two parts and no third, because the whole of accepting is the pick the viewer
/// injects onto this Set — see [`crate::response::Response::direction`]. The
/// recommendation travels as data rather than only as prose, because the chooser
/// draws it marked, and a rationale the human reads is not a thing a radio button
/// can be checked from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    /// Which of the three the agent recommends.
    pub direction: Direction,

    /// Why, as markdown. What the chooser shows beside the recommendation, so
    /// the human is deciding against the agent's reasoning rather than against
    /// a bare word.
    pub rationale: String,
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
