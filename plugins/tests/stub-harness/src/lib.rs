//! The component of the stub harness plugin the server's suites install from a
//! directory: a harness that is a script, declared by a manifest the suite
//! writes, with the three exports of `wit/plugin.wit`'s `agent` world behind
//! it. It is never a built-in.
//!
//! What each export answers is what a suite can read back:
//!
//! - **`write-configuration`** hands every configuration file back as it was
//!   found — so a server that let the account's own MCP servers through would
//!   be seen to — and writes the servers the Conversation attached into
//!   `servers.json` beside them. It traps on a file holding `{"trap":true}`,
//!   so a suite can see what a component that stops does to the next call.
//! - **`parse-transcript`** reads `memory/<session>.jsonl` under the account's
//!   root, a line per entry in a shape of the stub's own — see [`entry`] — and
//!   traps on a line `{"trap":true}`, so a suite can see what a component that
//!   stops does to a Transcript — and sleeps on a line `{"sleep":true}`, so
//!   one can see what a component waiting on the host for ever does to it.
//!   Handed no session, it finds the record as a Codex rollout is found: the
//!   newest written since the launch whose opening line
//!   `{"session": <id>, "cwd": <dir>}` names the Worktree — and answers with
//!   that id. Its cursor is how many bytes it has read, and it reads from
//!   there, whole lines only.
//! - **`login`** reads what a stub logging in writes — see [`LOGIN`] — and
//!   answers each thing it recognises as the step it is, in its own words.

wit_bindgen::generate!({
    path: "../../../crates/server/wit",
    world: "agent",
});

use std::io::{Read, Seek};

struct Stub;

impl Guest for Stub {
    fn parse_transcript(account: String, looking: Looking) -> Option<Transcript> {
        let memory = format!("{account}/memory");
        let (session, record) = match looking.session {
            Some(session) => {
                let record = format!("{memory}/{session}.jsonl");
                (Some(session), record)
            }
            None => {
                let (session, record) =
                    found(&memory, looking.worktree.as_deref()?, looking.launched)?;
                (Some(session), record)
            }
        };

        let read: u64 = looking
            .cursor
            .and_then(|cursor| cursor.parse().ok())
            .unwrap_or(0);
        let mut file = std::fs::File::open(record).ok()?;
        file.seek(std::io::SeekFrom::Start(read)).ok()?;
        let mut arrived = Vec::new();
        file.read_to_end(&mut arrived).ok()?;

        // Whole lines only: the rest is a line still being written.
        let whole = arrived
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at + 1);
        let entries = String::from_utf8_lossy(&arrived[..whole])
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .filter(|line: &serde_json::Value| line.get("cwd").is_none())
            .map(|line| entry(&line))
            .collect();

        Some(Transcript {
            session,
            entries,
            cursor: (read + whole as u64).to_string(),
        })
    }

    fn write_configuration(found: Vec<Found>, servers: Vec<Server>) -> Configuration {
        let trap = found.iter().any(|found| {
            found
                .contents
                .as_deref()
                .and_then(|contents| serde_json::from_slice::<serde_json::Value>(contents).ok())
                .is_some_and(|contents| contents["trap"].as_bool() == Some(true))
        });
        if trap {
            panic!("the stub's configuration asked it to stop");
        }

        let mut files: Vec<File> = found
            .into_iter()
            .filter_map(|found| {
                Some(File {
                    path: found.path,
                    contents: found.contents?,
                })
            })
            .collect();

        files.push(File {
            path: "servers.json".to_owned(),
            contents: servers_json(&servers).into_bytes(),
        });

        Configuration {
            files,
            settings: None,
        }
    }

    fn login(written: String) -> LoginStep {
        let mut kept = WRITTEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        kept.push_str(&written);

        // The first thing it recognises in what is not answered yet, and
        // everything up to the end of it taken as answered.
        let Some((at, said, step)) = LOGIN
            .iter()
            .filter_map(|(said, step)| Some((kept.find(said)?, *said, step)))
            .min_by_key(|(at, _, _)| *at)
        else {
            return LoginStep::Waiting;
        };
        let after = at + said.len();

        let step = match step {
            Step::Address => {
                // The address is the rest of its line, once the line is ended.
                let Some(ended) = kept[after..].find('\n') else {
                    return LoginStep::Waiting;
                };
                let url = kept[after..after + ended].trim().to_owned();
                kept.drain(..after + ended);
                return LoginStep::Address(url);
            }
            Step::Code => LoginStep::Code,
            Step::Words(words) => LoginStep::Words((*words).to_owned()),
            Step::Done => LoginStep::Done,
        };
        kept.drain(..after);
        step
    }
}

/// The record under `memory` whose opening line names `worktree`, written
/// since `launched` (milliseconds since the epoch, taken to the second the
/// coarsest filesystem keeps), the newest where there are more — and the
/// session id that line gives.
fn found(memory: &str, worktree: &str, launched: u64) -> Option<(String, String)> {
    let since = std::time::UNIX_EPOCH + std::time::Duration::from_secs(launched / 1000);
    let mut newest: Option<(std::time::SystemTime, String, String)> = None;

    for record in std::fs::read_dir(memory).ok()?.flatten() {
        let Some(written) = record
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .filter(|written| *written >= since)
        else {
            continue;
        };
        if newest.as_ref().is_some_and(|(at, _, _)| *at >= written) {
            continue;
        }

        let path = record.path().to_string_lossy().into_owned();
        let Some(opening) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(text.lines().next()?).ok())
        else {
            continue;
        };
        if opening["cwd"].as_str() != Some(worktree) {
            continue;
        }
        let Some(session) = opening["session"].as_str() else {
            continue;
        };
        newest = Some((written, session.to_owned(), path));
    }

    newest.map(|(_, session, path)| (session, path))
}

/// What a stub logging in has written and `login` has not answered yet. The
/// instance is the one login's, so this is too.
static WRITTEN: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// What `login` recognises in what a stub logging in writes.
enum Step {
    /// The address follows, to the end of the line.
    Address,
    Code,
    Words(&'static str),
    Done,
}

/// What a stub logging in writes, and what each is answered as: the words its
/// own, so that a suite can tell them from anything the host would say.
const LOGIN: &[(&str, Step)] = &[
    ("Go to: ", Step::Address),
    ("Code? ", Step::Code),
    ("Too short.", Step::Words("The stub wants all of the code.")),
    ("Wrong code.", Step::Words("The stub turned that code down.")),
    ("Gave up.", Step::Words("The stub stopped waiting for a code.")),
    ("Signed in.", Step::Done),
];

/// One line of the stub's record, as the kind it says it is.
///
/// - `{"said": "agent" | "reasoning" | "put", "markdown": …}`
/// - `{"call": <name>, "id": …, "about": …, "input": …}`
/// - `{"result": <id>, "failed": …, "text": …}`
/// - `{"notice": <kind>, "line": …}`
/// - `{"unknown": <kind> | null, "line": …}`
/// - `{"trap": true}`, which stops the component where it stands.
/// - `{"sleep": true}`, which waits an hour on the host's clock, running no
///   code of its own all the while.
///
/// And a line naming a `cwd` is none of them: it is a record opening by
/// saying which session it is and where — see [`found`] — and stands for no
/// entry. A record found that way is `<session>.jsonl`, as every other is, so
/// that it is read by its id once it has given one.
fn entry(line: &serde_json::Value) -> Entry {
    let text = |key: &str| line[key].as_str().unwrap_or_default().to_owned();

    if line["sleep"].as_bool() == Some(true) {
        std::thread::sleep(std::time::Duration::from_secs(60 * 60));
    }

    if line["trap"].as_bool() == Some(true) {
        panic!("the stub's record asked it to stop");
    }

    if let Some(voice) = line["said"].as_str() {
        return Entry::Said(Said {
            voice: match voice {
                "reasoning" => Voice::Reasoning,
                "put" => Voice::Put,
                _ => Voice::Agent,
            },
            markdown: text("markdown"),
        });
    }
    if line.get("call").is_some() {
        return Entry::ToolCall(ToolCall {
            name: text("call"),
            call: text("id"),
            about: text("about"),
            input: text("input"),
        });
    }
    if line.get("result").is_some() {
        return Entry::ToolResult(ToolResult {
            call: text("result"),
            failed: line["failed"].as_bool().unwrap_or(false),
            text: text("text"),
        });
    }
    if line.get("notice").is_some() {
        return Entry::Notice(Notice {
            kind: text("notice"),
            line: text("line"),
        });
    }

    Entry::Unknown(Unknown {
        kind: line["unknown"].as_str().map(str::to_owned),
        line: text("line"),
    })
}

/// The attached servers as one JSON object, each by its name.
fn servers_json(servers: &[Server]) -> String {
    let entries: Vec<String> = servers
        .iter()
        .map(|server| {
            let headers: Vec<String> = server
                .headers
                .iter()
                .map(|(name, value)| format!("{}:{}", quoted(name), quoted(value)))
                .collect();
            format!(
                "{}:{{\"url\":{},\"headers\":{{{}}}}}",
                quoted(&server.name),
                quoted(&server.url),
                headers.join(",")
            )
        })
        .collect();

    format!("{{{}}}", entries.join(","))
}

/// `text` as a JSON string.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

export!(Stub);
