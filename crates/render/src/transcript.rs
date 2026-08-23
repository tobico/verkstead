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
//! **A call and its answer are linked, not merged.** The pane draws the two as
//! one card, and what says which answer belongs to which call is the name the
//! backend gave the call, carried on both turns. Joining them here would not
//! survive an incremental reading: a batch ends wherever the log had got to, so
//! a call whose answer falls in the next batch would have to be held back or
//! sent twice. The link crosses the wire and the pane does the joining, over
//! the whole record it has accumulated.
//!
//! **Three classes of line, not two.** Beside the conversation a log carries the
//! backend's own bookkeeping — modes, reminders, attachments, snapshots — which
//! is roughly a third of every log and none of it anything a reader came for. It
//! is kept out of the turns and put in one group of its own, expandable, so that
//! nothing is hidden and nothing is in the way. Only a line that is neither the
//! conversation nor known bookkeeping is unrecognised, and those get ADR 0006's
//! treatment.
//!
//! **A reading can be carried on.** A running session's Transcript is re-read
//! every time it says anything, which late in a session is megabytes twice a
//! second — so a reading says where it got to, and the next one begins there
//! (ADR 0009). What that takes is a [`Cursor`], because the numbering cannot be
//! worked out from the lines: one line is any number of turns, so how far the
//! count had got is a thing to remember rather than a thing to derive.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One session's Transcript as the details pane receives it — the whole of it,
/// or whatever of it lies past where the pane's last reading stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TranscriptView {
    /// The conversation, in the order it happened.
    pub turns: Vec<Turn>,

    /// Everything that was not the conversation.
    pub bookkeeping: Vec<Bookkeeping>,

    /// Whether this is the record from its beginning, rather than a piece of it
    /// to add to what the reader already has.
    ///
    /// The reader cannot tell from the payload and must not guess: a reading
    /// that was asked to carry on from a cursor and could not falls back to the
    /// whole record, which is always a correct answer — and appending one of
    /// those to what was already drawn would be drawing the beginning twice.
    pub whole: bool,

    /// Where this reading stopped, to be handed back to ask for what comes
    /// after it. Opaque to whoever holds it: it is the server's own bookmark,
    /// and the shape of it is [`Cursor`]'s business alone.
    pub cursor: String,
}

/// How far a reading of a Transcript got.
///
/// Three counts rather than one, because a reading has to be resumed as well as
/// stopped: the lines say where to start reading again, and the two numberings
/// say what to call the first turn and the first bookkeeping line found there.
/// None of the three follows from the others — one log line is any number of
/// turns — so all three are remembered.
///
/// Written as a string on the wire and read back here, which is what makes it
/// opaque to the viewer: what the viewer does with one is hand it back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cursor {
    /// How many of the Transcript's lines the reading consumed, which is also
    /// the sequence number of the last of them.
    pub lines: u32,

    /// How many turns it had numbered by the end.
    turns: u32,

    /// And how many lines of the backend's own bookkeeping.
    bookkeeping: u32,
}

impl Display for Cursor {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        write!(out, "{}.{}.{}", self.lines, self.turns, self.bookkeeping)
    }
}

impl FromStr for Cursor {
    type Err = ();

    /// A cursor this wrote, read back — or nothing, for anything else.
    ///
    /// Nothing rather than a guess: a cursor is a URL parameter, which is to say
    /// something anybody can type, and one that was not written here says
    /// nothing about where a reading should carry on from. The caller's answer
    /// to that is to read the record whole, which is always correct.
    fn from_str(text: &str) -> Result<Cursor, ()> {
        let counts: Vec<&str> = text.split('.').collect();

        let [lines, turns, bookkeeping] = counts[..] else {
            return Err(());
        };

        Ok(Cursor {
            lines: lines.parse().map_err(|_| ())?,
            turns: turns.parse().map_err(|_| ())?,
            bookkeeping: bookkeeping.parse().map_err(|_| ())?,
        })
    }
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

    /// The name the backend gave this call, which its answer names back. Empty
    /// where the log gave none, which leaves the call with nothing to pair it
    /// to — still a call, still shown.
    pub call: String,

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

    /// The call this answers, by the name the backend gave it. Empty where the
    /// log gave none, which leaves the answer standing on its own.
    pub call: String,

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

/// How many turns a batch of the Transcript's lines is.
///
/// What the Timeline row counts. A turn is whatever the pane would draw as one
/// — the prose, the thinking, the tool call, the answer, the turn put to it —
/// and the backend's own bookkeeping is not among them, so this is the same
/// reading rather than a count of anything simpler.
///
/// The same reading and not a second opinion of it: the lines go through
/// [`read`] exactly as they do for the pane, and what [`Reads::Counting`] takes
/// out is the rendering rather than any of the judgement. A count worked out
/// another way would be a second definition of what a turn is, and two of those
/// would come apart the first time either moved.
///
/// Additive, which is what lets the relay keep a running total: a line is read
/// on its own, so the turns of two batches are the turns of both.
pub fn turns(lines: &[String]) -> usize {
    let mut reading = Reading::new(Cursor::default(), Reads::Counting);

    for line in lines {
        read(line, &mut reading);
    }

    reading.turns.len()
}

/// The Transcript's lines, read into the conversation they record.
///
/// One line can be more than one turn — an agent that wrote prose and then
/// called two tools wrote all three into a single line — so what comes back is
/// longer than what went in as often as not.
pub fn transcript_view(lines: &[String]) -> TranscriptView {
    transcript_after(Cursor::default(), lines)
}

/// The same, carried on from where a reading stopped: `lines` are the ones past
/// `from`, and what comes back is numbered as though the whole record had been
/// read in one go.
///
/// Which is the property the two halves of an incremental read rest on. Nothing
/// here looks back at what came before — every line is read on its own, and the
/// numbering is the only thing carried across — so a record accumulated a batch
/// at a time is the record read whole, turn for turn.
pub fn transcript_after(from: Cursor, lines: &[String]) -> TranscriptView {
    let mut reading = Reading::new(from, Reads::Drawing);

    for line in lines {
        read(line, &mut reading);
    }

    reading.into_view(lines.len() as u32)
}

/// The id every turn is built with, and none keeps: the real one is stamped by
/// [`Reading::take`] as the reading takes the turn.
const UNPLACED: u32 = 0;

/// One reading of a Transcript being made: what it has found, and where it
/// began.
struct Reading {
    /// Where the reading before this one stopped, which is what everything
    /// found here is numbered on from. All zeroes for a reading of the whole
    /// record, which is the same thing said of a record nothing has read yet.
    from: Cursor,

    /// Whether the turns are being drawn or only counted.
    reads: Reads,

    turns: Vec<Turn>,
    bookkeeping: Vec<Bookkeeping>,
}

/// What a reading is for: the pane, which draws the turns, or the Timeline row,
/// which counts them.
///
/// The same reading either way — the same lines, the same blocks, the same
/// judgements about which of them is a turn and which is the backend talking to
/// itself. What counting leaves out is only the rendering: the markdown, the
/// JSON laid out to be read, the text of what a tool answered. All of that is a
/// row's worth of work per line of a log, done on the loop that is following it
/// while a session runs, for a number nobody was going to read the HTML of.
///
/// One code path rather than two, because the count the row shows and the turns
/// the pane draws have to be the same number. Two readings kept in step by hand
/// would fall out of it the first time a kind of block was added to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reads {
    /// Everything, rendered, for the pane to draw.
    Drawing,

    /// The turns alone. Everything rendered comes out empty, and nothing else
    /// about the reading changes.
    Counting,
}

impl Reading {
    /// A reading about to begin, carrying on from `from`.
    fn new(from: Cursor, reads: Reads) -> Reading {
        Reading {
            from,
            reads,
            turns: Vec::new(),
            bookkeeping: Vec::new(),
        }
    }

    /// The conversation taking one more turn, numbered as it is taken.
    ///
    /// The `id` is the turn's place in the conversation, and it is counted
    /// here because the reading is the one thing that knows how far that has
    /// got: one log line can be several turns, so no count over the lines
    /// could say.
    fn take(&mut self, mut turn: Turn) {
        *turn.place() = self.from.turns + self.turns.len() as u32 + 1;
        self.turns.push(turn);
    }

    /// One more line of bookkeeping, numbered the same way — by its own count,
    /// since it was never part of the conversation.
    fn keep(&mut self, kind: String, line: String) {
        self.bookkeeping.push(Bookkeeping {
            id: self.from.bookkeeping + self.bookkeeping.len() as u32 + 1,
            kind,
            line,
        });
    }

    /// The reading finished, with the cursor whoever asks for the rest hands
    /// back.
    fn into_view(self, read: u32) -> TranscriptView {
        let at = Cursor {
            lines: self.from.lines + read,
            turns: self.from.turns + self.turns.len() as u32,
            bookkeeping: self.from.bookkeeping + self.bookkeeping.len() as u32,
        };

        TranscriptView {
            turns: self.turns,
            bookkeeping: self.bookkeeping,
            // A reading that began at the beginning is the record itself, and
            // there is nothing for the reader to add it to.
            whole: self.from == Cursor::default(),
            cursor: at.to_string(),
        }
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
fn read(line: &str, into: &mut Reading) {
    let reads = into.reads;

    let Ok(entry) = serde_json::from_str::<Value>(line) else {
        // Not JSON at all, which is a line torn by something or a format that
        // has stopped being JSONL. Either way it is what the session wrote, so
        // it is shown as it stands.
        into.take(Turn::Unread(Unread {
            id: UNPLACED,
            line: text(line, reads),
        }));
        return;
    };

    match entry["type"].as_str() {
        Some("assistant") => said(&entry, into),
        Some("user") => put(&entry, into),
        Some(kind) if BOOKKEEPING.contains(&kind) => into.keep(kind.to_owned(), raw(&entry, reads)),
        _ => into.take(unread(&entry, reads)),
    }
}

/// What the agent said: its prose, its reasoning, and the tools it called.
fn said(entry: &Value, into: &mut Reading) {
    let reads = into.reads;

    let Some(blocks) = entry["message"]["content"].as_array() else {
        into.take(unread(entry, reads));
        return;
    };

    for block in blocks {
        match block["type"].as_str() {
            Some("text") => {
                if let Some(prose) = prose(block, "text", reads) {
                    into.take(Turn::Prose(prose));
                }
            }
            Some("thinking") => {
                if let Some(Prose { html, .. }) = prose(block, "thinking", reads) {
                    into.take(Turn::Reasoning(Reasoning { id: UNPLACED, html }));
                }
            }
            Some("tool_use") => into.take(Turn::ToolUse(called(block, reads))),
            _ => into.take(unread(block, reads)),
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
fn put(entry: &Value, into: &mut Reading) {
    let reads = into.reads;

    if entry["isMeta"].as_bool().unwrap_or(false) {
        into.keep("user".to_owned(), raw(entry, reads));
        return;
    }

    match &entry["message"]["content"] {
        // A turn typed by the human arrives as the words themselves.
        Value::String(said) => {
            if let Some(html) = rendered(said, reads) {
                into.take(Turn::Put(Put { id: UNPLACED, html }));
            }
        }
        Value::Array(blocks) => {
            for block in blocks {
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(Prose { html, .. }) = prose(block, "text", reads) {
                            into.take(Turn::Put(Put { id: UNPLACED, html }));
                        }
                    }
                    Some("tool_result") => into.take(Turn::ToolResult(answered(block, reads))),
                    _ => into.take(unread(block, reads)),
                }
            }
        }
        _ => into.take(unread(entry, reads)),
    }
}

/// Markdown out of `key` of `block`, rendered — or nothing, where the block
/// said nothing.
///
/// Nothing rather than an empty rendering, because an empty turn draws as a
/// collapsed row with nothing behind it. Reasoning arrives this way when the
/// backend redacted it: a signature, and no thinking to go with it.
fn prose(block: &Value, key: &str, reads: Reads) -> Option<Prose> {
    rendered(block[key].as_str().unwrap_or_default(), reads)
        .map(|html| Prose { id: UNPLACED, html })
}

/// The same for markdown that is already in hand.
///
/// Whether there is a turn here at all is decided the same way for a reading
/// that is only counting — an empty block is no turn either way — and it is the
/// rendering that is skipped.
fn rendered(markdown: &str, reads: Reads) -> Option<String> {
    match (markdown.trim().is_empty(), reads) {
        (true, _) => None,
        (false, Reads::Counting) => Some(String::new()),
        (false, Reads::Drawing) => Some(crate::markdown::to_html(markdown)),
    }
}

/// A tool call: which tool, the one line about it, and what it was called with.
fn called(block: &Value, reads: Reads) -> ToolUse {
    let input = &block["input"];

    ToolUse {
        id: UNPLACED,
        name: text(block["name"].as_str().unwrap_or_default(), reads),
        call: text(block["id"].as_str().unwrap_or_default(), reads),
        about: match reads {
            Reads::Drawing => about(input),
            Reads::Counting => String::new(),
        },
        input: raw(input, reads),
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
fn answered(block: &Value, reads: Reads) -> ToolResult {
    if reads == Reads::Counting {
        return ToolResult {
            id: UNPLACED,
            call: String::new(),
            failed: false,
            text: String::new(),
        };
    }

    ToolResult {
        id: UNPLACED,
        call: text(block["tool_use_id"].as_str().unwrap_or_default(), reads),
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
fn unread(value: &Value, reads: Reads) -> Turn {
    Turn::Unread(Unread {
        id: UNPLACED,
        line: raw(value, reads),
    })
}

/// JSON laid out to be read. Collapsed in the pane either way — but a reader
/// who opens one has opened it to read it, and a format change is a thing to
/// understand rather than to squint at.
fn raw(value: &Value, reads: Reads) -> String {
    match reads {
        Reads::Drawing => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
        Reads::Counting => String::new(),
    }
}

/// Text a turn carries as it stands — and nothing at all where the reading is
/// only counting, which is what keeps a count from copying a log.
fn text(said: &str, reads: Reads) -> String {
    match reads {
        Reads::Drawing => said.to_owned(),
        Reads::Counting => String::new(),
    }
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
        transcript_view(&lines(FIXTURE))
    }

    fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|line| (*line).to_owned()).collect()
    }

    /// The count the Timeline row shows is the turns the pane draws, and the one
    /// fixture with one of everything in it is where the two are held to that.
    #[test]
    fn the_count_is_the_turns_the_pane_would_draw() {
        assert_eq!(turns(&lines(FIXTURE)), fixture().turns.len());
        assert_eq!(
            turns(&lines(FIXTURE)),
            6,
            "one of everything, and the backend's own bookkeeping counted as none of it"
        );
    }

    /// One line is any number of turns and a batch is any number of lines, so
    /// what the relay adds up as it follows a log has to be the count of the
    /// whole record — see [`crate::turns`].
    #[test]
    fn the_counts_of_two_batches_are_the_count_of_both() {
        let whole = lines(FIXTURE);
        let (first, rest) = whole.split_at(3);

        assert_eq!(turns(first) + turns(rest), turns(&whole));
    }

    /// An empty block is no turn to either reading: a redacted thought is a
    /// signature with no thinking to go with it, and the pane draws nothing for
    /// one.
    #[test]
    fn a_block_that_said_nothing_is_counted_as_no_turn() {
        let said = lines(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"","signature":"xx"}]}}"#,
        ]);

        assert_eq!(turns(&said), 0);
        assert_eq!(turns(&said), transcript_view(&said).turns.len());
    }

    /// And a session that keeps no log has nothing to count, which is what the
    /// row shows no metric for.
    #[test]
    fn a_transcript_with_nothing_on_it_is_no_turns() {
        assert_eq!(turns(&[]), 0);
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
                    call: "toolu_1".to_owned(),
                    about: "List the task files".to_owned(),
                    input: serde_json::to_string_pretty(&serde_json::json!({
                        "command": "ls .tasks",
                        "description": "List the task files",
                    }))
                    .unwrap(),
                }),
                Turn::ToolResult(ToolResult {
                    id: 5,
                    call: "toolu_1".to_owned(),
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

    /// A call and the answer to it name each other, which is what the pane
    /// draws the two as one card on. The names come from the log rather than
    /// from the order: an agent that called three tools at once wrote three
    /// calls and then three answers, and only the names say which answered
    /// which.
    #[test]
    fn a_call_and_its_answer_carry_the_name_that_joins_them() {
        let view = transcript_view(&lines(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_a","name":"Read","input":{"file_path":"one.rs"}},{"type":"tool_use","id":"toolu_b","name":"Read","input":{"file_path":"two.rs"}}]}}"#,
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_b","content":"two"},{"type":"tool_result","tool_use_id":"toolu_a","content":"one"}]}}"#,
        ]));

        let named: Vec<&str> = view
            .turns
            .iter()
            .map(|turn| match turn {
                Turn::ToolUse(call) => call.call.as_str(),
                Turn::ToolResult(answer) => answer.call.as_str(),
                _ => "",
            })
            .collect();

        assert_eq!(
            named,
            ["toolu_a", "toolu_b", "toolu_b", "toolu_a"],
            "the answers came back the other way round, and say so"
        );
    }

    /// A log that named neither leaves both standing on their own rather than
    /// pairing everything nameless with everything else.
    #[test]
    fn a_call_the_log_did_not_name_has_nothing_to_pair_it_to() {
        let view = transcript_view(&lines(&[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{"file_path":"one.rs"}}]}}"#,
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"one"}]}}"#,
        ]));

        assert!(
            matches!(
                &view.turns[..],
                [Turn::ToolUse(call), Turn::ToolResult(answer)]
                    if call.call.is_empty() && answer.call.is_empty()
            ),
            "{:?}",
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
                call: "toolu_3".to_owned(),
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

    /// The whole of what an incremental read rests on: a record accumulated a
    /// batch at a time is the record read in one go, turn for turn and
    /// numbering included — the ending among them, since the fixture's last
    /// line falls in the second half.
    #[test]
    fn a_record_read_in_two_goes_is_the_record_read_whole() {
        let lines: Vec<String> = FIXTURE.iter().map(|line| (*line).to_owned()).collect();

        let whole = transcript_view(&lines);
        let first = transcript_view(&lines[..3]);
        let rest = transcript_after(first.cursor.parse().unwrap(), &lines[3..]);

        assert_eq!(
            [first.turns.clone(), rest.turns].concat(),
            whole.turns,
            "the turns of the two readings, in order, are the turns of the one"
        );
        assert_eq!(
            [first.bookkeeping.clone(), rest.bookkeeping].concat(),
            whole.bookkeeping,
            "and the bookkeeping goes on being numbered across the join, so it \
             folds into the one group at the end"
        );
        assert_eq!(
            rest.cursor, whole.cursor,
            "and the two readings have got to the same place"
        );
    }

    /// Which reading arrived is the server's to say rather than the reader's to
    /// work out: appending a whole record to what was already drawn would draw
    /// the beginning of it twice.
    #[test]
    fn a_reading_says_whether_it_is_the_record_or_a_piece_of_one() {
        let lines: Vec<String> = FIXTURE.iter().map(|line| (*line).to_owned()).collect();

        assert!(transcript_view(&lines).whole);
        assert!(!transcript_after("3.3.0".parse().unwrap(), &lines[3..]).whole);
    }

    /// A cursor is a URL parameter, which is to say something anybody can type.
    /// One that was not written here says nothing about where to carry on from,
    /// and the caller's answer to that is to read the record whole.
    #[test]
    fn a_cursor_this_did_not_write_is_refused() {
        assert_eq!(
            "7.4.2".parse(),
            Ok(Cursor {
                lines: 7,
                turns: 4,
                bookkeeping: 2
            }),
            "and one it did write is read back as it was written"
        );

        for typed in ["", "7", "7.4", "7.4.2.1", "7.4.x", "-1.0.0", "7 . 4 . 2"] {
            assert_eq!(typed.parse::<Cursor>(), Err(()), "{typed:?}");
        }
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
