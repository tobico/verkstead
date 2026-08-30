//! The Transcript made readable: the lines a session's backend wrote about its
//! own conversation, turned into the turns the details pane draws.
//!
//! The store keeps those lines verbatim and reads none of them, so this is the
//! one place that knows the shape of a file somebody else owns (ADR 0006). What
//! that buys is a format change costing a rendering rather than a record: a line
//! nothing here recognises is still on the Transcript, and is still shown — as
//! the JSON it is.
//!
//! **Two backends, and the line says which.** Claude Code writes a line per
//! thing said; codex writes a rollout, whose lines are a timestamp, an ordinal
//! and a `type` around a payload. Nothing has to be told which is which: the
//! kinds are disjoint, so a line falls to the reader that knows its own kind
//! and to the fold below both where neither does. The same lines are rendered
//! in three places and none of them carries the agent type, which is what makes
//! reading it off the line the only answer that works everywhere.
//!
//! **A rollout writes every turn down twice, and one of the two is the
//! conversation.** Its `response_item` lines are what the model was sent — the
//! developer preamble and the environment block among them — and its `event_msg`
//! lines are what the TUI drew. The drawn one is what the pane draws, because it
//! is what the human at that terminal would have seen; rendering both would
//! double every turn and open every Transcript on pages of injected prompt. The
//! other stream folds away under its own name, where nothing is hidden and it
//! opens for whoever wants it.
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
//! nothing is hidden and nothing is in the way. A whole line of a kind nothing
//! here knows goes to the same group, under the name the log gave it: it used
//! to stand in the conversation as the JSON it is, which put `atis-latch` — a
//! type the backend added and never announced — between two turns of a talk.
//!
//! **A line folds, a block does not.** The other half of that boundary is
//! inside a turn, and it goes the other way: a block of a type nothing here
//! knows is part of what somebody said, so it stays where it was said, as the
//! JSON it is. One folded away silently would be a turn with a hole in it,
//! which is what ADR 0006's treatment is there to prevent. A line that is not
//! JSON at all, and a line that does not say what type it is, stay inline for
//! the same reason — neither has a name to be filed under. A rollout's item is
//! a block by this reckoning: it is one thing the screen drew inside the event
//! that says it was drawn.
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

    /// Something inside a turn that nothing here knows, or a line that never
    /// said what it was.
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

/// Something nothing here knows how to draw, in the conversation where it was
/// found: a block of an unknown type inside a turn, a line that is not JSON at
/// all, or one that does not say what type it is. A whole line whose type is
/// merely unknown is not one of these — it folds away as [`Bookkeeping`].
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
/// A closed list, and a fall-back past it: a whole line of a kind nobody here
/// has heard of is folded away too, under whatever the log called it. The list
/// is what says a kind was expected, and the fall-back is what keeps a type the
/// backend added without announcing it — `atis-latch` was the one — out of the
/// conversation while still showing it (ADR 0006). Nothing is hidden by that,
/// since the group opens, and the name it is filed under is what makes a new
/// kind findable to whoever comes looking.
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

/// The same for a rollout: the kinds of line codex writes that are not the
/// conversation it drew.
///
/// `response_item` is the largest of them and the one that matters most, since
/// it is the whole of the talk over again as the model was sent it — see this
/// module's own documentation. The rest are the session's own record of itself:
/// the meta line it opens with, the context each turn ran under, the state of
/// the world it was given, what it scored a command's risk at, what it said to
/// another agent, and the summary it replaced a long conversation with.
///
/// A closed list and a fall-back past it, exactly as above: codex adds kinds
/// without announcing them — `world_state` was not in the brief this backend was
/// built from — and one nobody here has heard of folds away under its own name
/// rather than standing in the conversation (ADR 0006).
const ROLLOUT: &[&str] = &[
    "compacted",
    "inter_agent_communication",
    "inter_agent_communication_metadata",
    "response_item",
    "security_risk_score",
    "session_meta",
    "turn_context",
    "world_state",
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
/// its Timeline row and the evidence a stop's Notice carries, and both of those
/// are asking what the session last *said*.
///
/// The same reading the pane draws, as [`turns`] is: the lines go through
/// [`read`], and [`Reads::Quoting`] takes the prose out of it unrendered.
/// Reading it a second way would be a second answer to what counts as the
/// agent's own words, and two backends' worth of that is two places for the two
/// to come apart.
///
/// Markdown as it stands rather than rendered, because neither of those two
/// draws HTML: a Timeline row is one line of text and evidence is a block of it.
pub fn statements(lines: &[String]) -> Vec<String> {
    let mut reading = Reading::new(Cursor::default(), Reads::Quoting);

    for line in lines {
        read(line, &mut reading);
    }

    reading
        .turns
        .into_iter()
        .filter_map(|turn| match turn {
            Turn::Prose(Prose { html, .. }) => Some(html),
            _ => None,
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

/// The directory a Codex rollout's opening line says its session was working
/// in — and `None` for any other line, and for anything that is not one.
///
/// The one thing about a rollout that is read anywhere but here, and it is read
/// here for the reason everything else about somebody else's file format is:
/// this crate is the one place that knows the shape of it (ADR 0006). What
/// wants it is the finder, because a Codex session's log is found rather than
/// named — codex takes no session id at launch, so what identifies its log is
/// the `cwd` it wrote in its own first line.
pub fn rollout_cwd(line: &str) -> Option<String> {
    let line: Value = serde_json::from_str(line).ok()?;

    if line.get("type")?.as_str()? != SESSION_META {
        return None;
    }

    Some(line.get("payload")?.get("cwd")?.as_str()?.to_owned())
}

/// What codex calls the line it opens a rollout with. Named because it is
/// somebody else's spelling, the same bargain the usage-limit phrase and the
/// idle signature make: one place to edit when it moves.
const SESSION_META: &str = "session_meta";

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

    /// The prose as the agent wrote it, for the row and the Notice that quote
    /// the session in a line. Markdown rather than HTML, and everything that is
    /// not prose comes out empty the way counting leaves it — see
    /// [`statements`].
    Quoting,
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
        Some(EVENT_MSG) => drawn(&entry, into),
        Some(kind) if BOOKKEEPING.contains(&kind) || ROLLOUT.contains(&kind) => {
            into.keep(kind.to_owned(), raw(&entry, reads))
        }
        // And a whole line of a kind nobody here has heard of, the same way:
        // folded under the name the log gave it rather than stood in the
        // conversation — see [`BOOKKEEPING`].
        Some(kind) => into.keep(kind.to_owned(), raw(&entry, reads)),
        // A line that does not say what it is has no name to file it under, so
        // it is shown, the way a line that is not JSON at all is.
        None => into.take(unread(&entry, reads)),
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
        (false, Reads::Quoting) => Some(markdown.trim().to_owned()),
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
            _ => String::new(),
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
    if reads != Reads::Drawing {
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

/// What codex types the lines its TUI drew from, and what it types the event
/// saying it finished drawing one item of a turn.
///
/// Somebody else's spellings, named for the reason [`SESSION_META`] is: one
/// place to edit when they move.
const EVENT_MSG: &str = "event_msg";
const ITEM_COMPLETED: &str = "item_completed";

/// What a tool item calls the status of a call that went through. Codex spells
/// the rest of its statuses differently from item to item, and this is the one
/// word all of them share.
const COMPLETED: &str = "completed";

/// One line of what codex drew: an item of the conversation, or the session's
/// own bookkeeping about the turn it was drawing.
///
/// Filed under the event's own name rather than the line's. Every one of these
/// lines is typed `event_msg`, so a group of them all called that would say
/// nothing about any of them — where `token_count` and `task_complete` say what
/// they are and are findable by it. A line whose payload never said what it was
/// keeps the name the line had, which is the only name it has.
fn drawn(entry: &Value, into: &mut Reading) {
    let reads = into.reads;
    let payload = &entry["payload"];

    match payload["type"].as_str() {
        Some(ITEM_COMPLETED) => match payload.get("item") {
            Some(item) => completed(item, into),
            // An event that says an item was drawn and names no item drew
            // nothing, whatever else it is.
            None => into.keep(ITEM_COMPLETED.to_owned(), raw(entry, reads)),
        },
        Some(event) => into.keep(event.to_owned(), raw(entry, reads)),
        None => into.keep(EVENT_MSG.to_owned(), raw(entry, reads)),
    }
}

/// One item of the conversation as the TUI drew it, put where it belongs.
///
/// The kinds are codex's own spellings, and an item of one nothing here knows
/// stays where it was drawn as the JSON it is. That is the opposite of what a
/// line of an unknown kind does, and deliberately: an item is part of a turn
/// rather than a line beside it, and one folded away silently would leave a
/// hole in the conversation (ADR 0006).
fn completed(item: &Value, into: &mut Reading) {
    let reads = into.reads;

    match item["type"].as_str() {
        Some("UserMessage") => {
            if let Some(html) = rendered(&spoken(item), reads) {
                into.take(Turn::Put(Put { id: UNPLACED, html }));
            }
        }
        Some("AgentMessage") => {
            if let Some(html) = rendered(&spoken(item), reads) {
                into.take(Turn::Prose(Prose { id: UNPLACED, html }));
            }
        }
        Some("Reasoning") => {
            if let Some(html) = rendered(&thought(item), reads) {
                into.take(Turn::Reasoning(Reasoning { id: UNPLACED, html }));
            }
        }
        _ => match tool(item, reads) {
            Some(called) => called.take(item, into),
            None => into.take(unread(item, reads)),
        },
    }
}

/// The words of a message item, whichever of the two kinds it is.
///
/// What is read is the `text` an element of the content carries rather than
/// what the element calls itself, because the two kinds spell that differently
/// — codex types a person's element `text` and the agent's `Text` — and the
/// words are in the same place either way. An element with no words in it says
/// what it was instead, the way a tool's answer does: a picture nobody can draw
/// here is still something that was said.
fn spoken(item: &Value) -> String {
    let Some(content) = item["content"].as_array() else {
        return String::new();
    };

    content
        .iter()
        .map(
            |element| match (element["text"].as_str(), element["type"].as_str()) {
                (Some(said), _) => said.to_owned(),
                (None, Some(kind)) => format!("[{kind}]"),
                (None, None) => String::new(),
            },
        )
        .collect::<Vec<String>>()
        .join("")
}

/// The agent's thinking out of a reasoning item: the summary it wrote for the
/// screen, and the raw chain where the model was asked for that instead and
/// there is no summary to show.
fn thought(item: &Value) -> String {
    let summary = words(&item["summary_text"], "\n\n");

    match summary.is_empty() {
        true => words(&item["raw_content"], "\n\n"),
        false => summary,
    }
}

/// A tool item read into the two turns the pane draws it as.
///
/// Codex writes a call and the answer to it as one item — the command and what
/// it printed, the tool and what it returned — where Claude Code writes two
/// lines. The pane draws a card out of a call and an answer naming each other,
/// so the one item becomes that pair, both carrying the item's own id as the
/// name that joins them.
struct Tool {
    /// What the log calls the tool: the name it gives where it gives one, and
    /// the item's own kind where the item *is* the tool.
    name: String,

    /// The one line about it, uncut — [`Tool::take`] cuts it to a row.
    about: String,

    /// What it was called with.
    input: String,

    /// What it said back.
    answer: String,

    /// Whether it failed.
    failed: bool,
}

impl Tool {
    /// The call and the answer to it, taken into the conversation in that
    /// order.
    fn take(self, item: &Value, into: &mut Reading) {
        let call = text(item["id"].as_str().unwrap_or_default(), into.reads);

        into.take(Turn::ToolUse(ToolUse {
            id: UNPLACED,
            name: self.name,
            call: call.clone(),
            about: one_line(&self.about),
            input: self.input,
        }));
        into.take(Turn::ToolResult(ToolResult {
            id: UNPLACED,
            call,
            failed: self.failed,
            text: self.answer,
        }));
    }
}

/// A tool item read — and nothing at all for an item that is not a tool, which
/// is what leaves everything else to be drawn as what it is.
fn tool(item: &Value, reads: Reads) -> Option<Tool> {
    let kind = item["type"].as_str()?;

    // The first of `keys` the item carries, which is how an item that says the
    // same thing under two names — a command's aggregated output and the
    // formatted output beside it — is read without asking for both.
    let first = |keys: &[&str]| {
        text(
            keys.iter()
                .find_map(|key| item[*key].as_str())
                .unwrap_or_default(),
            reads,
        )
    };

    let tool = match kind {
        "CommandExecution" => Tool {
            name: text(kind, reads),
            about: summarised(|| ran(item), reads),
            input: raw(&item["command"], reads),
            answer: first(&["aggregated_output", "formatted_output", "stdout"]),
            failed: failed(item),
        },
        "FileChange" => Tool {
            name: text(kind, reads),
            about: summarised(|| changed(item), reads),
            input: raw(&item["changes"], reads),
            answer: first(&["stdout", "stderr"]),
            failed: failed(item),
        },
        "WebSearch" => Tool {
            name: text(kind, reads),
            about: first(&["query"]),
            input: raw(&item["action"], reads),
            answer: shown(&item["results"], reads),
            failed: false,
        },
        "McpToolCall" => Tool {
            name: qualified(item, "server", reads),
            about: summarised(|| about(&item["arguments"]), reads),
            input: raw(&item["arguments"], reads),
            answer: answer(item, "result", reads),
            failed: failed(item),
        },
        "DynamicToolCall" => Tool {
            name: qualified(item, "namespace", reads),
            about: summarised(|| about(&item["arguments"]), reads),
            input: raw(&item["arguments"], reads),
            answer: answer(item, "content_items", reads),
            failed: failed(item),
        },
        _ => return None,
    };

    Some(tool)
}

/// A summary worked out only where somebody is going to read it, which is what
/// keeps a reading that is only counting from walking a call's arguments.
fn summarised(said: impl FnOnce() -> String, reads: Reads) -> String {
    match reads {
        Reads::Drawing => said(),
        _ => String::new(),
    }
}

/// The name a tool item gives itself: the two halves the log names it in, and
/// the tool alone where the log named no first half.
fn qualified(item: &Value, first: &str, reads: Reads) -> String {
    let tool = item["tool"].as_str().unwrap_or_default();

    summarised(
        || match item[first].as_str() {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}.{tool}"),
            _ => tool.to_owned(),
        },
        reads,
    )
}

/// What a tool that answers in structure said back: its error where it failed
/// with one, and what it returned under `key` where it did not.
fn answer(item: &Value, key: &str, reads: Reads) -> String {
    match item["error"]["message"].as_str() {
        Some(message) => text(message, reads),
        None => shown(&item[key], reads),
    }
}

/// JSON laid out to be read, and nothing at all where there is nothing to read
/// — which is what keeps a tool that answered with nothing from answering
/// `null`.
fn shown(value: &Value, reads: Reads) -> String {
    match value.is_null() {
        true => String::new(),
        false => raw(value, reads),
    }
}

/// Whether a tool item failed, which is anything the item did not call
/// completed: a command that came back non-zero, a call the human declined, a
/// tool that answered with an error.
fn failed(item: &Value) -> bool {
    item["status"].as_str() != Some(COMPLETED) || item["success"] == Value::Bool(false)
}

/// The one line a command execution is summarised by: the command as codex
/// parsed it for its own screen, and the argv it actually ran where it parsed
/// nothing — which is a shell, a flag, and the command as one string.
fn ran(item: &Value) -> String {
    if let Some(parsed) = item["parsed_cmd"][0]["cmd"].as_str() {
        return parsed.to_owned();
    }

    words(&item["command"], " ")
}

/// And the one line a patch is summarised by: the files it changed.
fn changed(item: &Value) -> String {
    item["changes"]
        .as_object()
        .map(|changes| {
            changes
                .keys()
                .map(String::as_str)
                .collect::<Vec<&str>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// The strings of an array, joined by `between`.
fn words(value: &Value, between: &str) -> String {
    value
        .as_array()
        .map(|said| {
            said.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
                .join(between)
        })
        .unwrap_or_default()
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
        _ => String::new(),
    }
}

/// Text a turn carries as it stands — and nothing at all where the reading is
/// only counting, which is what keeps a count from copying a log.
fn text(said: &str, reads: Reads) -> String {
    match reads {
        Reads::Drawing => said.to_owned(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A log with one of everything in it: the agent's prose, its thinking, a
    /// tool called and the tool's answer, a turn put to it, a line of the
    /// backend's bookkeeping, and a line of a kind nobody here has heard of —
    /// which is bookkeeping too, and the second of the two.
    const FIXTURE: &[&str] = &[
        r#"{"type":"user","message":{"role":"user","content":"Rename the **capture**."}}"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"The tables are the awkward part.","signature":"xx"}]}}"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I'll start with the *tables*."}]}}"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls .tasks","description":"List the task files"}}]}}"#,
        r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","is_error":false,"content":"04-render.md\n05-summaries.md"}]}}"#,
        r#"{"type":"attachment","attachment":{"type":"todos","content":"nothing a reader came for"}}"#,
        r#"{"type":"atis-latch","latched":"a kind from a later version"}"#,
    ];

    fn fixture() -> TranscriptView {
        transcript_view(&lines(FIXTURE))
    }

    fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|line| (*line).to_owned()).collect()
    }

    /// The same JSON as the reading lays it out, which is what a fold shows.
    fn pretty(line: &str) -> String {
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(line).unwrap()).unwrap()
    }

    /// The count the Timeline row shows is the turns the pane draws, and the one
    /// fixture with one of everything in it is where the two are held to that.
    #[test]
    fn the_count_is_the_turns_the_pane_would_draw() {
        assert_eq!(turns(&lines(FIXTURE)), fixture().turns.len());
        assert_eq!(
            turns(&lines(FIXTURE)),
            5,
            "one of everything, and neither line of bookkeeping counted as any of it"
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
            ],
            "and the two lines that were nobody talking are not among them"
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
            vec![
                Bookkeeping {
                    id: 1,
                    kind: "attachment".to_owned(),
                    line: pretty(FIXTURE[5]),
                },
                Bookkeeping {
                    id: 2,
                    kind: "atis-latch".to_owned(),
                    line: pretty(FIXTURE[6]),
                },
            ]
        );
        assert!(
            !view
                .turns
                .iter()
                .any(|turn| matches!(turn, Turn::Unread(unread) if unread.line.contains("todos"))),
            "bookkeeping is known, not unrecognised"
        );
    }

    /// The line that prompted the boundary: `atis-latch` is a type the backend
    /// added without announcing it, and it used to stand between two turns of
    /// a talk saying only that this version had never met it. It folds away
    /// under the name the log gave it — nothing lost, nothing in the way.
    #[test]
    fn a_line_of_a_kind_nobody_knows_folds_under_its_own_name() {
        let view = transcript_view(&[
            r#"{"type":"telepathy","thought":"a kind from a later version"}"#.to_owned(),
        ]);

        assert!(view.turns.is_empty(), "nobody said this: {:?}", view.turns);
        assert_eq!(
            view.bookkeeping,
            vec![Bookkeeping {
                id: 1,
                kind: "telepathy".to_owned(),
                line: pretty(r#"{"type":"telepathy","thought":"a kind from a later version"}"#),
            }]
        );
    }

    /// And a line that never said what it was has no name to be filed under,
    /// so it is shown where it fell — the same answer as a line that is not
    /// JSON at all.
    #[test]
    fn a_line_that_says_nothing_about_its_kind_is_shown() {
        let view = transcript_view(&[r#"{"thought":"and no type to go with it"}"#.to_owned()]);

        assert!(
            matches!(&view.turns[..], [Turn::Unread(unread)] if unread.line.contains("no type")),
            "{:?}",
            view.turns
        );
        assert!(view.bookkeeping.is_empty());
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

    /// The other half of the boundary: a whole line of an unknown kind folds
    /// away, and a block of one inside a turn does not. A block is part of
    /// what somebody said, and one folded away silently would leave a hole in
    /// the turn it was said in.
    #[test]
    fn a_block_of_a_kind_nobody_knows_is_shown_where_it_was_said() {
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

    /// A Codex rollout's opening line, as codex 0.149.0 writes one — cut off
    /// after the field the finder is after, because the rest of it is the
    /// session's whole system prompt and eighteen kilobytes of it.
    const SESSION_META_LINE: &str = r#"{"timestamp":"2026-08-30T07:47:01.017Z","ordinal":0,"type":"session_meta","payload":{"session_id":"01a051a2-d4e0-7f03-8839-d771ca4d0e73","cwd":"/srv/worktrees/rate-limiting","originator":"codex_exec"}}"#;

    /// What says a rollout is this session's: the directory codex says it was
    /// launched in, which for a Verkstead session is the Conversation's
    /// Worktree.
    #[test]
    fn a_rollouts_opening_line_says_where_its_session_was_working() {
        assert_eq!(
            rollout_cwd(SESSION_META_LINE).as_deref(),
            Some("/srv/worktrees/rate-limiting")
        );
    }

    /// And every other line of the file says nothing about it. The rest of a
    /// rollout carries a `cwd` of its own in places — a `turn_context` does —
    /// and reading one of those would be identifying a session by a directory
    /// it happened to be in for a turn.
    #[test]
    fn no_other_line_of_a_rollout_says_where_the_session_was_working() {
        for line in [
            r#"{"type":"turn_context","payload":{"cwd":"/srv/worktrees/rate-limiting"}}"#,
            r#"{"type":"session_meta","payload":{"session_id":"01a051a2"}}"#,
            r#"{"type":"session_meta"}"#,
            r#"{"payload":{"cwd":"/srv/worktrees/rate-limiting"}}"#,
            "not JSON at all",
            "",
        ] {
            assert_eq!(rollout_cwd(line), None, "{line:?}");
        }
    }

    /// A rollout codex 0.149.0 wrote of a session that was asked something,
    /// thought about it, said what it was going to do, ran a command and
    /// answered — every line of it as codex wrote it, but for the four bulky
    /// ones, which are cut to what is read of them: the meta line's whole
    /// system prompt, the world it was given, the turn's context and the
    /// developer preamble each run to kilobytes, and the worktree's path is
    /// shortened throughout so that the fixture reads.
    ///
    /// The shape to see in it is the doubling. Ordinals 7 to 19 are the turn
    /// said twice — the `event_msg` lines are what the screen drew, and the
    /// `response_item` lines beside them are the same turn as the model was
    /// sent it, in among the preamble nobody said.
    const ROLLOUT: &[&str] = &[
        r#"{"timestamp":"2026-08-30T08:11:27.307Z","ordinal":0,"type":"session_meta","payload":{"session_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","cwd":"/srv/worktrees/tables","originator":"codex_exec"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.307Z","ordinal":1,"type":"event_msg","payload":{"type":"task_started","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","started_at":1788077487,"model_context_window":258400,"collaboration_mode_kind":"default"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":2,"type":"response_item","payload":{"type":"message","id":"msg_01a051b9-34cf-75b2-93a0-acaca50f73a3","role":"developer","content":[{"type":"input_text","text":"<skills_instructions>\n## Skills\nA skill is a set of instructions provided through a `SKILL.md` source."}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":3,"type":"response_item","payload":{"type":"message","id":"msg_01a051b9-34cf-75b2-93a0-acb06c799843","role":"user","content":[{"type":"input_text","text":"<environment_context>\n  <cwd>/srv/worktrees/tables</cwd>\n  <shell>bash</shell>\n</environment_context>"}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":4,"type":"world_state","payload":{"full":true,"state":{"agents_md":{},"apps_instructions":false}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":5,"type":"turn_context","payload":{"turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","cwd":"/srv/worktrees/tables"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.316Z","ordinal":6,"type":"response_item","payload":{"type":"message","id":"msg_01a051b9-34d4-7612-95dc-1001c4fb2884","role":"user","content":[{"type":"input_text","text":"What is in the task list?"}],"internal_chat_message_metadata_passthrough":{"turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","create_time":1788077487.3162456}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.316Z","ordinal":7,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"UserMessage","id":"01a051b9-34d4-7612-95dc-10185af451bf","content":[{"type":"text","text":"What is in the task list?","text_elements":[]}]},"started_at_ms":1788077487316,"completed_at_ms":1788077487316}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.335Z","ordinal":8,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"Reasoning","id":"rs-1","summary_text":["**Reading the task list**\n\nThe backlog is where to start."],"raw_content":[]},"started_at_ms":1788077487335,"completed_at_ms":1788077487335}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.335Z","ordinal":9,"type":"response_item","payload":{"type":"reasoning","id":"rs-1","summary":[{"type":"summary_text","text":"**Reading the task list**\n\nThe backlog is where to start."}],"content":null,"encrypted_content":null}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.336Z","ordinal":10,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"AgentMessage","id":"msg-1","content":[{"type":"Text","text":"I'll look at the **task list** first."}]},"started_at_ms":1788077487336,"completed_at_ms":1788077487336}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.336Z","ordinal":11,"type":"response_item","payload":{"type":"message","id":"msg-1","role":"assistant","content":[{"type":"output_text","text":"I'll look at the **task list** first."}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.337Z","ordinal":12,"type":"response_item","payload":{"type":"function_call","id":"fc_01a051b9-34e9-7e33-b5b0-7ec74bfd68c9","name":"exec_command","arguments":"{\"cmd\":\"ls .tasks\"}","call_id":"call-1"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.369Z","ordinal":13,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"CommandExecution","id":"call-1","process_id":"45936","command":["/bin/bash","-lc","ls .tasks"],"cwd":"file:///srv/worktrees/tables","parsed_cmd":[{"type":"list_files","cmd":"ls .tasks","path":".tasks"}],"source":"unified_exec_startup","status":"completed","stdout":"04-render.md\n05-summaries.md\n","stderr":"","aggregated_output":"04-render.md\n05-summaries.md\n","exit_code":0,"duration":{"secs":0,"nanos":2425},"formatted_output":"04-render.md\n05-summaries.md\n"},"started_at_ms":1788077487369,"completed_at_ms":1788077487369}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.370Z","ordinal":14,"type":"response_item","payload":{"type":"function_call_output","id":"fco_01a051b9-350a-7a20-932c-94ffaad12aba","call_id":"call-1","output":"Chunk ID: 57be28\nProcess exited with code 0\nOutput:\n04-render.md\n05-summaries.md\n"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.371Z","ordinal":15,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1200,"output_tokens":40,"total_tokens":1240},"model_context_window":258400}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.388Z","ordinal":16,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"AgentMessage","id":"msg-2","content":[{"type":"Text","text":"Two files: `04-render.md` and `05-summaries.md`."}]},"started_at_ms":1788077487388,"completed_at_ms":1788077487388}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.388Z","ordinal":17,"type":"response_item","payload":{"type":"message","id":"msg-2","role":"assistant","content":[{"type":"output_text","text":"Two files: `04-render.md` and `05-summaries.md`."}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.390Z","ordinal":19,"type":"event_msg","payload":{"type":"task_complete","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","last_agent_message":"Two files: `04-render.md` and `05-summaries.md`.","started_at":1788077487,"completed_at":1788077487,"duration_ms":85}}"#,
    ];

    fn rollout() -> TranscriptView {
        transcript_view(&lines(ROLLOUT))
    }

    /// What the human at that terminal saw, and only that: the turn put to it,
    /// its thinking, its prose, the command it ran and what the command printed.
    #[test]
    fn a_real_rollout_draws_as_the_conversation_the_screen_drew() {
        let view = rollout();

        assert_eq!(
            view.turns,
            vec![
                Turn::Put(Put {
                    id: 1,
                    html: "<p>What is in the task list?</p>\n".to_owned()
                }),
                Turn::Reasoning(Reasoning {
                    id: 2,
                    html: "<p><strong>Reading the task list</strong></p>\n<p>The backlog is where \
                           to start.</p>\n"
                        .to_owned()
                }),
                Turn::Prose(Prose {
                    id: 3,
                    html: "<p>I'll look at the <strong>task list</strong> first.</p>\n".to_owned()
                }),
                Turn::ToolUse(ToolUse {
                    id: 4,
                    name: "CommandExecution".to_owned(),
                    call: "call-1".to_owned(),
                    about: "ls .tasks".to_owned(),
                    input: serde_json::to_string_pretty(&serde_json::json!([
                        "/bin/bash",
                        "-lc",
                        "ls .tasks",
                    ]))
                    .unwrap(),
                }),
                Turn::ToolResult(ToolResult {
                    id: 5,
                    call: "call-1".to_owned(),
                    failed: false,
                    text: "04-render.md\n05-summaries.md\n".to_owned(),
                }),
                Turn::Prose(Prose {
                    id: 6,
                    html: "<p>Two files: <code>04-render.md</code> and \
                           <code>05-summaries.md</code>.</p>\n"
                        .to_owned()
                }),
            ]
        );
    }

    /// The other half of the doubling: the turn as the model was sent it, which
    /// is the same words again with pages of injected prompt around them.
    /// Drawing both would draw every turn twice and open every Transcript on
    /// the preamble.
    #[test]
    fn the_stream_the_model_was_sent_folds_away_under_its_own_name() {
        let view = rollout();

        assert!(
            !view
                .turns
                .iter()
                .any(|turn| format!("{turn:?}").contains("environment_context")),
            "nobody said the environment block: {:?}",
            view.turns
        );

        let folded: Vec<&str> = view
            .bookkeeping
            .iter()
            .map(|line| line.kind.as_str())
            .collect();

        assert_eq!(
            folded,
            [
                "session_meta",
                "task_started",
                "response_item",
                "response_item",
                "world_state",
                "turn_context",
                "response_item",
                "response_item",
                "response_item",
                "response_item",
                "response_item",
                "token_count",
                "response_item",
                "task_complete",
            ],
            "and every line of it is on the Transcript, under the name codex gave it"
        );
    }

    /// The count the Timeline row shows and the turns the pane draws are the
    /// one reading on a rollout too — and so is the last thing the session said,
    /// which is the other half of what the row is summarised by.
    #[test]
    fn a_rollouts_row_is_counted_and_quoted_off_the_reading_the_pane_draws() {
        let said = lines(ROLLOUT);

        assert_eq!(turns(&said), rollout().turns.len());
        assert_eq!(turns(&said), 6, "and the bookkeeping is none of it");
        assert_eq!(
            statements(&said),
            vec![
                "I'll look at the **task list** first.",
                "Two files: `04-render.md` and `05-summaries.md`.",
            ],
            "the agent's prose, as it wrote it"
        );
    }

    /// A command that came back non-zero is an answer that failed, whatever
    /// else it printed — which is what the pane draws the card in red on. And a
    /// command codex parsed nothing out of is summarised by the argv it ran,
    /// which is a shell, a flag and the command as one string.
    #[test]
    fn a_command_that_failed_is_an_answer_that_says_so() {
        let view = transcript_view(&[
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","id":"call-9","command":["/bin/bash","-lc","cargo test"],"status":"failed","aggregated_output":"error: could not compile","exit_code":101}}}"#
                .to_owned(),
        ]);

        assert!(
            matches!(
                &view.turns[..],
                [Turn::ToolUse(call), Turn::ToolResult(answer)]
                    if call.about == "/bin/bash -lc cargo test"
                        && call.call == answer.call
                        && answer.failed
                        && answer.text == "error: could not compile"
            ),
            "{:?}",
            view.turns
        );
    }

    /// A tool the log gives a name of its own is called by that name rather
    /// than by the kind of item it arrived in, and what it answered with is
    /// what it said — which for a call that failed is the error rather than the
    /// nothing it returned.
    #[test]
    fn a_tool_the_log_names_is_called_what_the_log_calls_it() {
        let view = transcript_view(&[
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"McpToolCall","id":"mcp-1","server":"github","tool":"list_issues","arguments":{"query":"is:open label:bug"},"status":"failed","error":{"message":"the token is not authorised"}}}}"#
                .to_owned(),
        ]);

        assert!(
            matches!(
                &view.turns[..],
                [Turn::ToolUse(call), Turn::ToolResult(answer)]
                    if call.name == "github.list_issues"
                        && call.about == "is:open label:bug"
                        && answer.failed
                        && answer.text == "the token is not authorised"
            ),
            "{:?}",
            view.turns
        );
    }

    /// Codex adds kinds without announcing them, and the two halves of ADR
    /// 0006's rule are what that costs. A whole line of one folds away under its
    /// own name…
    #[test]
    fn a_rollout_line_of_a_kind_nobody_knows_folds_under_its_own_name() {
        let view = transcript_view(&[
            r#"{"timestamp":"2026-08-30T08:11:27.390Z","ordinal":20,"type":"seance","payload":{"heard":"a kind from a later version"}}"#
                .to_owned(),
        ]);

        assert!(view.turns.is_empty(), "nobody said this: {:?}", view.turns);
        assert_eq!(
            view.bookkeeping.first().map(|line| line.kind.as_str()),
            Some("seance")
        );
    }

    /// …and an event of one folds under the event's own name rather than the
    /// line's, since every one of these lines is called `event_msg` and a group
    /// of thirty rows all called that would say nothing about any of them.
    #[test]
    fn an_event_folds_under_its_own_name_rather_than_the_lines() {
        let view = transcript_view(&[
            r#"{"type":"event_msg","payload":{"type":"stream_error","message":"retrying"}}"#
                .to_owned(),
        ]);

        assert!(view.turns.is_empty(), "{:?}", view.turns);
        assert_eq!(
            view.bookkeeping.first().map(|line| line.kind.as_str()),
            Some("stream_error")
        );
    }

    /// …while an item of one stays in the conversation as the JSON it is. An
    /// item is part of a turn rather than a line beside it, and one folded away
    /// silently would leave a hole where something was drawn.
    #[test]
    fn an_item_of_a_kind_nobody_knows_is_drawn_where_it_was_drawn() {
        let view = transcript_view(&[
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"Telepathy","id":"tp-1","thought":"a kind from a later version"}}}"#
                .to_owned(),
        ]);

        assert!(
            matches!(&view.turns[..], [Turn::Unread(unread)] if unread.line.contains("Telepathy")),
            "{:?}",
            view.turns
        );
        assert!(view.bookkeeping.is_empty());
        assert_eq!(
            turns(&lines(&[
                r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"Telepathy","id":"tp-1","thought":"a kind from a later version"}}}"#,
            ])),
            1,
            "and the count does not drop it either"
        );
    }

    /// Which reader a line gets is decided by the line. Nothing a Transcript
    /// carries says which backend wrote it — the same lines are rendered in
    /// three places and none of them is told the agent type — so the two kinds
    /// are told apart by their own kinds, and read side by side when they are
    /// put side by side.
    #[test]
    fn which_reader_a_line_gets_is_decided_by_what_the_line_is() {
        let view = transcript_view(&lines(&[
            FIXTURE[2],
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"msg-1","content":[{"type":"Text","text":"And the *rollout* was read too."}]}}}"#,
        ]));

        assert_eq!(
            view.turns,
            vec![
                Turn::Prose(Prose {
                    id: 1,
                    html: "<p>I'll start with the <em>tables</em>.</p>\n".to_owned()
                }),
                Turn::Prose(Prose {
                    id: 2,
                    html: "<p>And the <em>rollout</em> was read too.</p>\n".to_owned()
                }),
            ]
        );
    }
}
