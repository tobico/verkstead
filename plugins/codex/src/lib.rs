//! Codex's component: the two things about Codex that are code, written
//! against `wit/plugin.wit`'s `agent` world (ADR-0030, *Agent*). Everything
//! else about Codex is data, in `plugin.yaml` beside this crate.
//!
//! Carried over from the in-process implementation rather than re-decided —
//! ADR-0011's *As built for Codex* says why each piece is what it is:
//!
//! - **`parse-transcript`** finds a session's rollout under the account's
//!   `sessions/`, which codex files by date, `YYYY/MM/DD/rollout-*.jsonl`. Codex
//!   takes no session id at launch, so until a rollout has said which session
//!   it is, one is found by the Worktree its opening `session_meta` line names
//!   and the moment it was written — see [`launched_in`] — and the id that line
//!   gives is the answer's, which a resume names. Handed that id, it is found by
//!   it — see [`declaring`]. Then each whole line past the cursor is read into
//!   the Transcript pane's kinds — see [`read`].
//! - **`write-configuration`** writes a `config.toml` carrying the two keys of
//!   the account's own that reach a model, and the attached servers under
//!   `mcp_servers` — see [`carrying`].
//! - **`login`** is never called: Verkstead runs no Codex login, and the
//!   manifest declares none.

wit_bindgen::generate!({
    path: "../../crates/server/wit",
    world: "agent",
});

use std::io::{Read, Seek};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

struct Codex;

impl Guest for Codex {
    fn parse_transcript(account: String, looking: Looking) -> Option<Transcript> {
        let sessions = format!("{account}/{SESSIONS}");

        let (session, rollout) = match looking.session {
            Some(session) => {
                let rollout = remembered(&session).or_else(|| declaring(&sessions, &session))?;
                (Some(session), rollout)
            }
            None => {
                let rollout =
                    launched_in(&sessions, looking.worktree.as_deref()?, looking.launched)?;
                (declares(&rollout), rollout)
            }
        };
        if let Some(session) = &session {
            remember(session, &rollout);
        }

        let read: u64 = looking
            .cursor
            .and_then(|cursor| cursor.parse().ok())
            .unwrap_or(0);
        let mut file = std::fs::File::open(&rollout).ok()?;
        file.seek(std::io::SeekFrom::Start(read)).ok()?;
        let mut arrived = Vec::new();
        file.read_to_end(&mut arrived).ok()?;

        // Whole lines only: the rest is a line codex is still writing.
        let whole = arrived
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at + 1);
        let mut entries = Vec::new();
        for line in String::from_utf8_lossy(&arrived[..whole]).lines() {
            read_line(line, &mut entries);
        }

        Some(Transcript {
            session,
            entries,
            cursor: (read + whole as u64).to_string(),
        })
    }

    fn write_configuration(found: Vec<Found>, servers: Vec<Server>) -> Configuration {
        let account = found
            .into_iter()
            .find(|found| found.path == CONFIG)
            .and_then(|found| found.contents)
            .and_then(|contents| String::from_utf8(contents).ok());

        Configuration {
            files: vec![File {
                path: CONFIG.to_owned(),
                contents: carrying(account.as_deref(), &servers),
            }],
            settings: None,
        }
    }

    /// Never called: the manifest declares no login.
    fn login(_written: String) -> LoginStep {
        LoginStep::Done
    }
}

/// The configuration file codex reads, inside `~/.codex`.
const CONFIG: &str = "config.toml";

/// Where codex keeps its rollouts, inside `~/.codex`.
const SESSIONS: &str = "sessions";

/// Of the account's own `config.toml`, the keys a root's carries over: how an
/// account on a provider of its own reaches the model. `model_provider` names
/// it, and `model_providers` says where it is. Everything else is how the
/// human works — `mcp_servers`, `profiles`, `projects`, hooks, `notify`,
/// `[features]` — and none of it is a session's.
const CARRIED: [&str; 2] = ["model_provider", "model_providers"];

/// The table the attached servers are written under, each server's URL as
/// `url` and its headers as `http_headers`, which is codex's spelling.
const MCP_SERVERS: &str = "mcp_servers";
const URL: &str = "url";
const HEADERS: &str = "http_headers";

/// A `config.toml` carrying the account's [`CARRIED`] keys, where its own
/// reads as TOML, and the Conversation's servers under [`MCP_SERVERS`] — after
/// the carry rather than over it, and carried by nothing of the account's.
///
/// An account with no such file, or one that does not read, is given one
/// with nothing of its own in it.
fn carrying(account: Option<&str>, servers: &[Server]) -> Vec<u8> {
    let mut written = toml::Table::new();

    if let Some(own) = account.and_then(|text| text.parse::<toml::Table>().ok()) {
        for key in CARRIED {
            if let Some(value) = own.get(key) {
                written.insert(key.to_owned(), value.clone());
            }
        }
    }

    if !servers.is_empty() {
        let mut ours = toml::Table::new();

        for server in servers {
            let mut entry = toml::Table::new();
            entry.insert(URL.to_owned(), server.url.as_str().into());

            // Left out altogether where there are none, so a server that wants
            // no header is written the way it was before there were headers.
            if !server.headers.is_empty() {
                let sent: toml::Table = server
                    .headers
                    .iter()
                    .map(|(header, value)| (header.clone(), value.as_str().into()))
                    .collect();

                entry.insert(HEADERS.to_owned(), toml::Value::Table(sent));
            }

            ours.insert(server.name.clone(), toml::Value::Table(entry));
        }

        written.insert(MCP_SERVERS.to_owned(), toml::Value::Table(ours));
    }

    toml::to_string(&written)
        .expect("a TOML table read off TOML writes as TOML")
        .into_bytes()
}

/// The rollout found for a session id, kept so that a poll after the first
/// is not a walk of the whole store: the instance is the one following's.
static FOUND: Mutex<Option<(String, String)>> = Mutex::new(None);

fn remembered(session: &str) -> Option<String> {
    let found = FOUND
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    found
        .as_ref()
        .filter(|(id, _)| id == session)
        .map(|(_, rollout)| rollout.clone())
}

fn remember(session: &str, rollout: &str) {
    let mut found = FOUND
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *found = Some((session.to_owned(), rollout.to_owned()));
}

/// The rollout under `sessions` whose opening line names `worktree`, written
/// since `launched` (milliseconds since the epoch, taken to the second the
/// coarsest filesystem keeps), the newest where there are more.
///
/// Two tests rather than one, because neither is enough on its own: the
/// Worktree alone would find whatever earlier session last worked in it, and
/// the moment alone whichever Conversation started a session at the same time.
///
/// Only the day directories written into since then are looked in: creating a
/// file touches the directory it lands in, so the day the rollout was written
/// in is a directory at least as new as it. Which day codex thinks it is, in
/// which timezone, is codex's business.
fn launched_in(sessions: &str, worktree: &str, launched: u64) -> Option<String> {
    let since = UNIX_EPOCH + Duration::from_secs(launched / 1000);
    let mut newest: Option<(SystemTime, String)> = None;

    for day in days(sessions) {
        if written(&day).is_none_or(|written| written < since) {
            continue;
        }

        for rollout in rollouts(&day) {
            let Some(written) = written(&rollout).filter(|written| *written >= since) else {
                continue;
            };
            if newest.as_ref().is_some_and(|(at, _)| *at >= written) {
                continue;
            }

            let cwd = opening(&rollout).and_then(|line| meta(&line, "cwd"));
            if cwd.as_deref() == Some(worktree) {
                newest = Some((written, rollout));
            }
        }
    }

    newest.map(|(_, rollout)| rollout)
}

/// The rollout under `sessions` whose opening line says it is the session
/// called `session` — its own line rather than its filename, which is codex's
/// spelling of a name rather than what the file says about itself (ADR-0006).
fn declaring(sessions: &str, session: &str) -> Option<String> {
    days(sessions)
        .into_iter()
        .flat_map(|day| rollouts(&day))
        .find(|rollout| declares(rollout).as_deref() == Some(session))
}

/// The id a rollout's opening line gives its session: `session_id`, the older
/// of the two spellings codex 0.155.1 writes holding the same value.
fn declares(rollout: &str) -> Option<String> {
    meta(&opening(rollout)?, "session_id")
}

/// `key` of a `session_meta` line's payload — and none for any other line.
fn meta(line: &str, key: &str) -> Option<String> {
    let line: Value = serde_json::from_str(line).ok()?;

    if line.get("type")?.as_str()? != SESSION_META {
        return None;
    }

    Some(line.get("payload")?.get(key)?.as_str()?.to_owned())
}

/// What codex calls the line it opens a rollout with.
const SESSION_META: &str = "session_meta";

/// How much of a rollout is read to identify it: generous against the
/// eighteen kilobytes codex 0.149.0 opens one with, and small against a log
/// that runs to megabytes. A first line longer than this is one that never
/// becomes identifiable.
const FIRST_LINE: u64 = 256 * 1024;

/// The line a rollout opens with, as far as it has been written.
fn opening(rollout: &str) -> Option<String> {
    let mut first = Vec::new();
    std::fs::File::open(rollout)
        .ok()?
        .take(FIRST_LINE)
        .read_to_end(&mut first)
        .ok()?;
    let end = first
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(first.len());
    Some(String::from_utf8_lossy(&first[..end]).into_owned())
}

/// Every day directory of `sessions`, `YYYY/MM/DD`.
fn days(sessions: &str) -> Vec<String> {
    let mut days = Vec::new();
    for year in directories(sessions) {
        for month in directories(&year) {
            days.extend(directories(&month));
        }
    }
    days
}

/// The directories in `dir`.
fn directories(dir: &str) -> Vec<String> {
    entries(dir)
        .into_iter()
        .filter(|(_, kind)| kind.is_dir())
        .map(|(path, _)| path)
        .collect()
}

/// The rollouts codex is writing in `day`, `rollout-*.jsonl` — and nothing
/// else it keeps beside them: the older ones it compresses, and its index.
fn rollouts(day: &str) -> Vec<String> {
    entries(day)
        .into_iter()
        .filter(|(path, kind)| {
            let name = path.rsplit('/').next().unwrap_or_default();
            kind.is_file() && name.starts_with("rollout-") && name.ends_with(".jsonl")
        })
        .map(|(path, _)| path)
        .collect()
}

fn entries(dir: &str) -> Vec<(String, std::fs::FileType)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            Some((format!("{dir}/{name}"), entry.file_type().ok()?))
        })
        .collect()
}

fn written(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// The kinds of line codex writes that are the session's record of itself
/// rather than the conversation it drew: the whole of the talk over again as
/// the model was sent it, the meta line it opens with, the context each turn
/// ran under, the state of the world it was given, what it scored a command's
/// risk at, what it said to another agent, and the summary it replaced a long
/// conversation with.
///
/// A closed list, and a fall-back past it: a kind codex adds that is on
/// nobody's list is answered as unknown under its own name, and folds away
/// under it all the same (ADR-0006).
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

/// What codex types the lines its TUI drew from, and the event saying it
/// finished drawing one item of a turn.
const EVENT_MSG: &str = "event_msg";
const ITEM_COMPLETED: &str = "item_completed";

/// What a tool item calls the status of a call that went through.
const COMPLETED: &str = "completed";

/// One line of a rollout, read into what it is.
///
/// **The `event_msg` lines are the conversation**: what codex's screen drew,
/// one item at a time. The `response_item` lines beside them are the same turn
/// as the model was sent it, so drawing both would say everything twice — they
/// are notices, with the rest of [`ROLLOUT`].
fn read_line(line: &str, into: &mut Vec<Entry>) {
    let Ok(entry) = serde_json::from_str::<Value>(line) else {
        // Not JSON at all: a line torn by something, shown as it stands.
        into.push(unknown(None, line.to_owned()));
        return;
    };

    match entry["type"].as_str() {
        Some(EVENT_MSG) => drawn(&entry, into),
        Some(kind) if ROLLOUT.contains(&kind) => into.push(notice(kind, &entry)),
        Some(kind) => into.push(unknown(Some(kind), shown(&entry))),
        None => into.push(unknown(None, shown(&entry))),
    }
}

/// One line of what codex drew: an item of the conversation, or codex's own
/// bookkeeping about the turn it was drawing, under the event's own name —
/// every one of these lines is typed `event_msg`, where `token_count` and
/// `task_complete` say what they are.
fn drawn(entry: &Value, into: &mut Vec<Entry>) {
    let payload = &entry["payload"];

    match payload["type"].as_str() {
        Some(ITEM_COMPLETED) => match payload.get("item") {
            Some(item) => completed(item, into),
            None => into.push(notice(ITEM_COMPLETED, entry)),
        },
        Some(event) => into.push(notice(event, entry)),
        None => into.push(notice(EVENT_MSG, entry)),
    }
}

/// One item of the conversation as the TUI drew it.
///
/// An item of a kind nothing here knows stays where it was drawn, as the JSON
/// it is, rather than folding away: an item is part of a turn, and one folded
/// silently would leave a hole in the conversation (ADR-0006).
fn completed(item: &Value, into: &mut Vec<Entry>) {
    let said = |voice, markdown| Entry::Said(Said { voice, markdown });

    match item["type"].as_str() {
        Some("UserMessage") => into.push(said(Voice::Put, spoken(item))),
        Some("AgentMessage") => into.push(said(Voice::Agent, spoken(item))),
        Some("Reasoning") => into.push(said(Voice::Reasoning, thought(item))),
        _ => match tool(item) {
            Some(tool) => {
                let call = item["id"].as_str().unwrap_or_default().to_owned();
                into.push(Entry::ToolCall(ToolCall {
                    name: tool.name,
                    call: call.clone(),
                    about: tool.about,
                    input: tool.input,
                }));
                into.push(Entry::ToolResult(ToolResult {
                    call,
                    failed: tool.failed,
                    text: tool.answer,
                }));
            }
            None => into.push(unknown(None, shown(item))),
        },
    }
}

/// The words of a message item: the `text` each element carries, which codex
/// types `text` for a person's and `Text` for the agent's — and, for an
/// element with no words, what it was.
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
        .collect()
}

/// The agent's thinking: the summary it wrote for the screen, and the raw
/// chain where there is no summary.
fn thought(item: &Value) -> String {
    let summary = words(&item["summary_text"], "\n\n");

    match summary.is_empty() {
        true => words(&item["raw_content"], "\n\n"),
        false => summary,
    }
}

/// A tool item, which codex writes as one item where the pane draws a call
/// and its answer: both are answered, joined by the item's own id.
struct Tool {
    name: String,
    /// The one line about it, uncut: the host cuts it to a row.
    about: String,
    input: String,
    answer: String,
    failed: bool,
}

/// A tool item read — and none for an item that is not a tool.
fn tool(item: &Value) -> Option<Tool> {
    let kind = item["type"].as_str()?;

    // The first of `keys` the item carries: a command's aggregated output and
    // the formatted output beside it say the same thing under two names.
    let first = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| item[*key].as_str())
            .unwrap_or_default()
            .to_owned()
    };

    let tool = match kind {
        "CommandExecution" => Tool {
            name: kind.to_owned(),
            about: ran(item),
            input: pretty(&item["command"]),
            answer: first(&["aggregated_output", "formatted_output", "stdout"]),
            failed: failed(item),
        },
        "FileChange" => Tool {
            name: kind.to_owned(),
            about: changed(item),
            input: pretty(&item["changes"]),
            answer: first(&["stdout", "stderr"]),
            failed: failed(item),
        },
        "WebSearch" => Tool {
            name: kind.to_owned(),
            about: first(&["query"]),
            input: pretty(&item["action"]),
            answer: shown(&item["results"]),
            failed: false,
        },
        "McpToolCall" => Tool {
            name: qualified(item, "server"),
            about: about(&item["arguments"]),
            input: pretty(&item["arguments"]),
            answer: answer(item, "result"),
            failed: failed(item),
        },
        "DynamicToolCall" => Tool {
            name: qualified(item, "namespace"),
            about: about(&item["arguments"]),
            input: pretty(&item["arguments"]),
            answer: answer(item, "content_items"),
            failed: failed(item),
        },
        _ => return None,
    };

    Some(tool)
}

/// The name a tool item gives itself: the two halves it is named in, and the
/// tool alone where there is no first half.
fn qualified(item: &Value, first: &str) -> String {
    let tool = item["tool"].as_str().unwrap_or_default();

    match item[first].as_str() {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}.{tool}"),
        _ => tool.to_owned(),
    }
}

/// What a tool that answers in structure said back: its error where it failed
/// with one, and what it returned under `key` where it did not.
fn answer(item: &Value, key: &str) -> String {
    match item["error"]["message"].as_str() {
        Some(message) => message.to_owned(),
        None => shown(&item[key]),
    }
}

/// Whether a tool item failed: anything the item did not call completed.
fn failed(item: &Value) -> bool {
    item["status"].as_str() != Some(COMPLETED) || item["success"] == Value::Bool(false)
}

/// The one line a command is summarised by: the command as codex parsed it
/// for its own screen, and the argv it ran where it parsed nothing.
fn ran(item: &Value) -> String {
    if let Some(parsed) = item["parsed_cmd"][0]["cmd"].as_str() {
        return parsed.to_owned();
    }

    words(&item["command"], " ")
}

/// The one line a patch is summarised by: the files it changed.
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

/// The keys of a tool's input that say what the call was about, best first.
const ABOUT: &[&str] = &[
    "description",
    "toolSummary",
    "command",
    "CommandLine",
    "file_path",
    "filePath",
    "path",
    "Path",
    "pattern",
    "prompt",
    "query",
    "url",
];

/// How much of that line is worth carrying.
const ABOUT_LIMIT: usize = 120;

/// The one line a call's input is collapsed to, out of whichever of [`ABOUT`]
/// it has.
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

/// A line of codex's own bookkeeping, under `kind`.
fn notice(kind: &str, line: &Value) -> Entry {
    Entry::Notice(Notice {
        kind: kind.to_owned(),
        line: pretty(line),
    })
}

fn unknown(kind: Option<&str>, line: String) -> Entry {
    Entry::Unknown(Unknown {
        kind: kind.map(str::to_owned),
        line,
    })
}

/// JSON laid out to be read.
fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// The same, and nothing at all where there is nothing — which keeps a tool
/// that answered with nothing from answering `null`.
fn shown(value: &Value) -> String {
    match value.is_null() {
        true => String::new(),
        false => pretty(value),
    }
}

export!(Codex);
