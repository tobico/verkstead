//! The Transcript made readable: the lines a session's backend wrote about its
//! own conversation, turned into the turns the details pane draws.
//!
//! The store keeps those lines verbatim and reads none of them, so this is the
//! one place that knows the shape of a file somebody else owns (ADR 0006). What
//! that buys is a format change costing a rendering rather than a record: a line
//! nothing here recognises is still on the Transcript, and is still shown — as
//! the JSON it is.
//!
//! **Split on the content, not the line.** A tool's answer arrives inside a line
//! the log types `user`, which is the same type a turn put by the human arrives
//! under. Keying off the type alone would draw a directory listing as though a
//! person had read it out, so what decides is the block inside.
//!
//! **Three classes of line, not two.** Beside the conversation a log carries the
//! backend's own bookkeeping — modes, reminders, attachments, snapshots — which
//! is roughly a third of every log and none of it anything a reader came for. It
//! is kept out of the turns and put in one group of its own, expandable, so that
//! nothing is hidden and nothing is in the way. Only a line that is neither the
//! conversation nor known bookkeeping is unrecognised, and those get ADR 0006's
//! treatment.

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One session's Transcript as the details pane receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TranscriptView {
    /// The conversation, in the order it happened.
    pub turns: Vec<Turn>,

    /// Everything that was not the conversation.
    pub bookkeeping: Vec<Bookkeeping>,
}

/// One thing that was said, or done, or put.
///
/// Flat on the wire — `{"id": 3, "kind": "Prose", "html": "…"}` — rather than
/// wrapped in the variant's name, because the viewer reconciles turns by `id`
/// and reconcile reads its key off the element itself: an id one level down
/// would match nothing and fall back to matching by position, silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Turn {
    /// The agent's own words.
    Prose(Prose),

    /// Its thinking.
    Reasoning(Reasoning),

    /// A tool it called.
    ToolUse(ToolUse),

    /// What the tool said back.
    ToolResult(ToolResult),

    /// A turn put to it.
    Put(Put),

    /// A line of a kind nothing here knows.
    Unread(Unread),
}

/// The agent's prose, rendered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Prose {
    /// The turn's place in the conversation, counted from 1 — what the viewer
    /// reconciles rows by, so a fold opened on one survives a re-read.
    pub id: u32,

    pub html: String,
}

/// The agent's reasoning, rendered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Reasoning {
    /// The turn's place in the conversation, counted from 1.
    pub id: u32,

    pub html: String,
}

/// A tool call, as one line plus what it was called with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ToolUse {
    /// The turn's place in the conversation, counted from 1.
    pub id: u32,

    /// What the tool is called.
    pub name: String,

    /// The one line about it. Empty where the call said nothing this could
    /// summarise, which leaves the name standing on its own.
    pub about: String,

    /// What it was called with, whole.
    pub input: String,
}

/// What a tool answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ToolResult {
    /// The turn's place in the conversation, counted from 1.
    pub id: u32,

    /// Whether the tool failed.
    pub failed: bool,

    /// What it said, as it said it.
    pub text: String,
}

/// A turn put to the agent, rendered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Put {
    /// The turn's place in the conversation, counted from 1.
    pub id: u32,

    pub html: String,
}

/// A line nothing here knows how to draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Unread {
    /// The turn's place in the conversation, counted from 1.
    pub id: u32,

    pub line: String,
}

/// One line of the backend's own bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Bookkeeping {
    /// The line's place among the bookkeeping, counted from 1 — its own count,
    /// not the conversation's.
    pub id: u32,

    /// What the log called it.
    pub kind: String,

    /// The line itself.
    pub line: String,
}

/// The kinds of line that are the backend's own bookkeeping rather than
/// anything anybody said: modes it switched to, reminders it attached, titles
/// it invented, snapshots it took of the files. Roughly a third of every log,
/// and none of it what a reader came for.
///
/// A closed list rather than "everything that is not the conversation", because
/// the two answers are different: bookkeeping is known and folded away, and a
/// kind nobody here has heard of is unrecognised and shown (ADR 0006). A list
/// makes a new kind announce itself; a catch-all would file it silently under
/// the noise.
const BOOKKEEPING: &[&str] = &[
    "agent-name",
    "ai-title",
    "attachment",
    "file-history-delta",
    "file-history-snapshot",
    "last-prompt",
    "mode",
    "permission-mode",
    "pr-link",
    "queue-operation",
    "system",
];

/// The keys of a tool's input that say what the call was about, best first.
///
/// Keys rather than tools. Verkstead does not know what tools a session has —
/// the set is the agent's, and half of it is whatever the project added — but
/// the conventions for naming an input are few and shared, so a tool nobody
/// here has heard of still gets a line out of this.
const ABOUT: &[&str] = &[
    "description",
    "command",
    "file_path",
    "path",
    "pattern",
    "prompt",
    "query",
    "url",
];

/// How much of that line is worth carrying. A summary is one row of a pane, and
/// a call whose description ran to a paragraph has not said a paragraph's worth
/// about itself.
const ABOUT_LIMIT: usize = 120;

/// What the agent itself said, out of a Transcript's lines: one entry per
/// statement, in the order it made them and in the markdown it wrote.
///
/// The prose alone — not its thinking, not the tools it called, not what was put
/// to it. What this is for is the two places a session is quoted in miniature,
/// its Timeline row and the evidence an Interruption carries, and both of those
/// are asking what the session last *said*.
///
/// Markdown as it stands rather than rendered, because neither of those two
/// draws HTML: a Timeline row is one line of text and evidence is a block of it.
pub fn statements(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|entry| entry["type"].as_str() == Some("assistant"))
        .flat_map(|entry| {
            let Some(blocks) = entry["message"]["content"].as_array() else {
                return Vec::new();
            };

            blocks
                .iter()
                .filter(|block| block["type"].as_str() == Some("text"))
                .map(|block| block["text"].as_str().unwrap_or_default().trim().to_owned())
                .filter(|said| !said.is_empty())
                .collect()
        })
        .collect()
}

/// The Transcript's lines, read into the conversation they record.
///
/// One line can be more than one turn — an agent that wrote prose and then
/// called two tools wrote all three into a single line — so what comes back is
/// longer than what went in as often as not.
pub fn transcript_view(lines: &[String]) -> TranscriptView {
    let mut view = TranscriptView {
        turns: Vec::new(),
        bookkeeping: Vec::new(),
    };

    for line in lines {
        read(line, &mut view);
    }

    view
}

/// The id every turn is built with, and none keeps: the real one is stamped by
/// [`TranscriptView::take`] as the view takes the turn.
const UNPLACED: u32 = 0;

impl TranscriptView {
    /// The conversation taking one more turn, numbered as it is taken.
    ///
    /// The `id` is the turn's place in the conversation, and it is counted
    /// here because the view is the one thing that knows how far that has got:
    /// one log line can be several turns, so no count over the lines could say.
    fn take(&mut self, mut turn: Turn) {
        *turn.place() = self.turns.len() as u32 + 1;
        self.turns.push(turn);
    }

    /// One more line of bookkeeping, numbered the same way — by its own count,
    /// since it was never part of the conversation.
    fn keep(&mut self, kind: String, line: String) {
        self.bookkeeping.push(Bookkeeping {
            id: self.bookkeeping.len() as u32 + 1,
            kind,
            line,
        });
    }
}

impl Turn {
    /// Where a turn's place in the conversation is written, whichever of the
    /// six it is.
    fn place(&mut self) -> &mut u32 {
        match self {
            Turn::Prose(Prose { id, .. })
            | Turn::Reasoning(Reasoning { id, .. })
            | Turn::ToolUse(ToolUse { id, .. })
            | Turn::ToolResult(ToolResult { id, .. })
            | Turn::Put(Put { id, .. })
            | Turn::Unread(Unread { id, .. }) => id,
        }
    }
}

/// One line, put wherever it belongs.
fn read(line: &str, view: &mut TranscriptView) {
    let Ok(entry) = serde_json::from_str::<Value>(line) else {
        // Not JSON at all, which is a line torn by something or a format that
        // has stopped being JSONL. Either way it is what the session wrote, so
        // it is shown as it stands.
        view.take(Turn::Unread(Unread {
            id: UNPLACED,
            line: line.to_owned(),
        }));
        return;
    };

    match entry["type"].as_str() {
        Some("assistant") => said(&entry, view),
        Some("user") => put(&entry, view),
        Some(kind) if BOOKKEEPING.contains(&kind) => view.keep(kind.to_owned(), raw(&entry)),
        _ => view.take(unread(&entry)),
    }
}

/// What the agent said: its prose, its reasoning, and the tools it called.
fn said(entry: &Value, view: &mut TranscriptView) {
    let Some(blocks) = entry["message"]["content"].as_array() else {
        view.take(unread(entry));
        return;
    };

    for block in blocks {
        match block["type"].as_str() {
            Some("text") => {
                if let Some(prose) = prose(block, "text") {
                    view.take(Turn::Prose(prose));
                }
            }
            Some("thinking") => {
                if let Some(Prose { html, .. }) = prose(block, "thinking") {
                    view.take(Turn::Reasoning(Reasoning { id: UNPLACED, html }));
                }
            }
            Some("tool_use") => view.take(Turn::ToolUse(called(block))),
            _ => view.take(unread(block)),
        }
    }
}

/// What was put to the agent: a turn from the human, or a tool answering the
/// call above.
///
/// The two arrive under the same type, which is why nothing here reads the
/// line's own — see the module's own documentation. `isMeta` is the third
/// thing arriving under it: a line the backend wrote to itself in the human's
/// voice, which is bookkeeping wearing a turn's clothes.
fn put(entry: &Value, view: &mut TranscriptView) {
    if entry["isMeta"].as_bool().unwrap_or(false) {
        view.keep("user".to_owned(), raw(entry));
        return;
    }

    match &entry["message"]["content"] {
        // A turn typed by the human arrives as the words themselves.
        Value::String(said) => {
            if let Some(html) = rendered(said) {
                view.take(Turn::Put(Put { id: UNPLACED, html }));
            }
        }
        Value::Array(blocks) => {
            for block in blocks {
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(Prose { html, .. }) = prose(block, "text") {
                            view.take(Turn::Put(Put { id: UNPLACED, html }));
                        }
                    }
                    Some("tool_result") => view.take(Turn::ToolResult(answered(block))),
                    _ => view.take(unread(block)),
                }
            }
        }
        _ => view.take(unread(entry)),
    }
}

/// Markdown out of `key` of `block`, rendered — or nothing, where the block
/// said nothing.
///
/// Nothing rather than an empty rendering, because an empty turn draws as a
/// collapsed row with nothing behind it. Reasoning arrives this way when the
/// backend redacted it: a signature, and no thinking to go with it.
fn prose(block: &Value, key: &str) -> Option<Prose> {
    rendered(block[key].as_str().unwrap_or_default()).map(|html| Prose { id: UNPLACED, html })
}

/// The same for markdown that is already in hand.
fn rendered(markdown: &str) -> Option<String> {
    match markdown.trim().is_empty() {
        true => None,
        false => Some(crate::markdown::to_html(markdown)),
    }
}

/// A tool call: which tool, the one line about it, and what it was called with.
fn called(block: &Value) -> ToolUse {
    let input = &block["input"];

    ToolUse {
        id: UNPLACED,
        name: block["name"].as_str().unwrap_or_default().to_owned(),
        about: about(input),
        input: raw(input),
    }
}

/// The one line a call is collapsed to, out of whichever of [`ABOUT`] the call
/// has. Empty where it has none, which leaves the tool's name standing alone —
/// still a call, still shown.
fn about(input: &Value) -> String {
    ABOUT
        .iter()
        .find_map(|key| input.get(key)?.as_str())
        .map(one_line)
        .unwrap_or_default()
}

/// Text cut to a row: its first line, and no more of that than fits.
fn one_line(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default().trim();

    match first.chars().count() > ABOUT_LIMIT {
        true => first.chars().take(ABOUT_LIMIT).chain(['…']).collect(),
        false => first.to_owned(),
    }
}

/// What a tool answered, as text.
///
/// Text rather than markdown: a tool's answer is output, and running a
/// directory listing through a markdown parser would turn whatever it happened
/// to contain into headings. An answer that came back as blocks — which is how
/// a screenshot arrives — keeps the text of it and says what the rest was,
/// because a picture nobody can draw here is still something that happened.
fn answered(block: &Value) -> ToolResult {
    ToolResult {
        id: UNPLACED,
        failed: block["is_error"].as_bool().unwrap_or(false),
        text: match &block["content"] {
            Value::String(text) => text.clone(),
            Value::Array(blocks) => blocks
                .iter()
                .map(|block| match block["type"].as_str() {
                    Some("text") => block["text"].as_str().unwrap_or_default().to_owned(),
                    Some(kind) => format!("[{kind}]"),
                    None => String::new(),
                })
                .collect::<Vec<String>>()
                .join("\n"),
            _ => String::new(),
        },
    }
}

/// Whatever this is, shown as the JSON it is.
fn unread(value: &Value) -> Turn {
    Turn::Unread(Unread {
        id: UNPLACED,
        line: raw(value),
    })
}

/// JSON laid out to be read. Collapsed in the pane either way — but a reader
/// who opens one has opened it to read it, and a format change is a thing to
/// understand rather than to squint at.
fn raw(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A log with one of everything in it: the agent's prose, its thinking, a
    /// tool called and the tool's answer, a turn put to it, a line of the
    /// backend's bookkeeping, and a line of a kind nobody here has heard of.
    const FIXTURE: &[&str] = &[
        r#"{"type":"user","message":{"role":"user","content":"Rename the **capture**."}}"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"The tables are the awkward part.","signature":"xx"}]}}"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I'll start with the *tables*."}]}}"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls .tasks","description":"List the task files"}}]}}"#,
        r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","is_error":false,"content":"04-render.md\n05-summaries.md"}]}}"#,
        r#"{"type":"attachment","attachment":{"type":"todos","content":"nothing a reader came for"}}"#,
        r#"{"type":"telepathy","thought":"a kind from a later version"}"#,
    ];

    fn fixture() -> TranscriptView {
        transcript_view(
            &FIXTURE
                .iter()
                .map(|line| (*line).to_owned())
                .collect::<Vec<String>>(),
        )
    }

    #[test]
    fn the_conversation_is_read_back_in_the_order_it_happened() {
        let view = fixture();

        assert_eq!(
            view.turns,
            vec![
                Turn::Put(Put {
                    id: 1,
                    html: "<p>Rename the <strong>capture</strong>.</p>\n".to_owned()
                }),
                Turn::Reasoning(Reasoning {
                    id: 2,
                    html: "<p>The tables are the awkward part.</p>\n".to_owned()
                }),
                Turn::Prose(Prose {
                    id: 3,
                    html: "<p>I'll start with the <em>tables</em>.</p>\n".to_owned()
                }),
                Turn::ToolUse(ToolUse {
                    id: 4,
                    name: "Bash".to_owned(),
                    about: "List the task files".to_owned(),
                    input: serde_json::to_string_pretty(&serde_json::json!({
                        "command": "ls .tasks",
                        "description": "List the task files",
                    }))
                    .unwrap(),
                }),
                Turn::ToolResult(ToolResult {
                    id: 5,
                    failed: false,
                    text: "04-render.md\n05-summaries.md".to_owned()
                }),
                Turn::Unread(Unread {
                    id: 6,
                    line: serde_json::to_string_pretty(
                        &serde_json::from_str::<Value>(FIXTURE[6]).unwrap()
                    )
                    .unwrap()
                }),
            ]
        );
    }

    /// What the viewer reconciles on: a turn's id is its place in the
    /// conversation, flat beside the `kind` tag rather than a level down inside
    /// it — reconcile reads the key off the element itself, and an id it cannot
    /// see falls back to matching by position, silently.
    #[test]
    fn a_turn_goes_on_the_wire_flat_with_its_place_and_kind() {
        let view = fixture();

        assert_eq!(
            serde_json::to_value(&view.turns[2]).unwrap(),
            serde_json::json!({
                "kind": "Prose",
                "id": 3,
                "html": "<p>I'll start with the <em>tables</em>.</p>\n",
            })
        );
    }

    /// The one that would be got wrong by reading the line's own type: a tool's
    /// answer and a human's turn arrive under the same one.
    #[test]
    fn a_tools_answer_is_not_something_a_person_said() {
        let view = fixture();

        let put = view
            .turns
            .iter()
            .filter(|turn| matches!(turn, Turn::Put(_)))
            .count();
        let answered = view
            .turns
            .iter()
            .filter(|turn| matches!(turn, Turn::ToolResult(_)))
            .count();

        assert_eq!(put, 1, "one turn was put to the agent");
        assert_eq!(answered, 1, "and one tool answered it");
    }

    #[test]
    fn the_backends_bookkeeping_is_kept_out_of_the_conversation() {
        let view = fixture();

        assert_eq!(
            view.bookkeeping,
            vec![Bookkeeping {
                id: 1,
                kind: "attachment".to_owned(),
                line: serde_json::to_string_pretty(
                    &serde_json::from_str::<Value>(FIXTURE[5]).unwrap()
                )
                .unwrap(),
            }]
        );
        assert!(
            !view
                .turns
                .iter()
                .any(|turn| matches!(turn, Turn::Unread(unread) if unread.line.contains("todos"))),
            "bookkeeping is known, not unrecognised"
        );
    }

    /// A line the agent's backend wrote for its own purposes rather than
    /// something anybody said, arriving under the type a human turn arrives
    /// under. What tells them apart is inside the line, as ever.
    #[test]
    fn a_turn_the_backend_put_to_itself_is_bookkeeping() {
        let view = transcript_view(&[
            r#"{"type":"user","isMeta":true,"message":{"role":"user","content":"Caveat: this was generated while running a local command."}}"#
                .to_owned(),
        ]);

        assert!(view.turns.is_empty(), "nobody said this");
        assert_eq!(view.bookkeeping.len(), 1);
    }

    /// ADR 0006's containment: a format that changes can leave a line nothing
    /// knows how to draw, and it can never leave the pane empty.
    #[test]
    fn a_line_that_is_not_json_at_all_is_still_shown() {
        let view = transcript_view(&["{ this was never JSON".to_owned()]);

        assert_eq!(
            view.turns,
            vec![Turn::Unread(Unread {
                id: 1,
                line: "{ this was never JSON".to_owned()
            })]
        );
    }

    /// The same rule one level down: the line is one this knows and the block
    /// inside it is not.
    #[test]
    fn a_block_of_a_kind_nobody_knows_is_shown_too() {
        let view = transcript_view(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"divination","omen":"a raven"}]}}"#
                .to_owned(),
        ]);

        assert!(
            matches!(&view.turns[..], [Turn::Unread(unread)] if unread.line.contains("divination")),
            "an unknown block is shown as what it is: {:?}",
            view.turns
        );
    }

    /// Every word of a Transcript was written by an agent, and half of what an
    /// agent writes is quoting something it read.
    #[test]
    fn what_the_agent_wrote_cannot_act_on_the_page() {
        let view = transcript_view(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Found: <script>alert(1)</script>"}]}}"#
                .to_owned(),
        ]);

        assert!(
            matches!(&view.turns[..], [Turn::Prose(prose)] if !prose.html.contains("<script")),
            "the sanitizer runs over log content too: {:?}",
            view.turns
        );
    }

    /// A tool named and nothing else worth a line — which is a call with no
    /// input this recognises, not a reason to say nothing.
    #[test]
    fn a_call_with_nothing_to_summarise_is_still_the_tool_that_was_called() {
        let view = transcript_view(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_2","name":"TodoWrite","input":{"todos":[]}}]}}"#
                .to_owned(),
        ]);

        assert!(
            matches!(&view.turns[..], [Turn::ToolUse(call)] if call.name == "TodoWrite" && call.about.is_empty()),
            "the name stands on its own: {:?}",
            view.turns
        );
    }

    /// An answer that came back as blocks rather than as text, which is how a
    /// screenshot arrives. Nothing of it is dropped: what cannot be drawn as
    /// text says what it was instead.
    #[test]
    fn an_answer_that_is_not_text_says_what_it_was() {
        let view = transcript_view(&[
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_3","is_error":true,"content":[{"type":"text","text":"could not read it"},{"type":"image","source":{}}]}]}}"#
                .to_owned(),
        ]);

        assert_eq!(
            view.turns,
            vec![Turn::ToolResult(ToolResult {
                id: 1,
                failed: true,
                text: "could not read it\n[image]".to_owned(),
            })]
        );
    }

    /// A block with nothing in it is nothing to draw. Redacted reasoning
    /// arrives this way — a signature and no thinking — and a collapsed
    /// nothing is worse than no row at all.
    #[test]
    fn a_block_that_said_nothing_is_not_a_turn() {
        let view = transcript_view(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"","signature":"xx"}]}}"#
                .to_owned(),
        ]);

        assert!(view.turns.is_empty(), "{:?}", view.turns);
    }

    #[test]
    fn a_session_that_left_no_log_has_no_conversation_to_show() {
        let view = transcript_view(&[]);

        assert!(view.turns.is_empty());
        assert!(view.bookkeeping.is_empty());
    }

    /// The prose and nothing else. A summary of a session is what the session
    /// said, and what a tool answered is neither said nor the session's.
    #[test]
    fn what_the_agent_said_is_its_prose_alone() {
        let lines: Vec<String> = FIXTURE.iter().map(|line| (*line).to_owned()).collect();

        assert_eq!(statements(&lines), vec!["I'll start with the *tables*."]);
    }

    #[test]
    fn a_session_that_has_only_called_tools_has_said_nothing_yet() {
        let lines: Vec<String> = FIXTURE[..2]
            .iter()
            .chain(FIXTURE[3..].iter())
            .map(|line| (*line).to_owned())
            .collect();

        assert!(statements(&lines).is_empty(), "{:?}", statements(&lines));
    }
}
