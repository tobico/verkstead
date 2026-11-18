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
//!   `servers.json` beside them.
//! - **`parse-transcript`** reads `memory/<session>.jsonl` under the account's
//!   root, a line per entry in a shape of the stub's own — see [`entry`] — and
//!   traps on a line `{"trap":true}`, so a suite can see what a component that
//!   stops does to a Transcript.
//! - **`login`** reads what a stub logging in writes — see [`LOGIN`] — and
//!   answers each thing it recognises as the step it is, in its own words.

wit_bindgen::generate!({
    path: "../../../crates/server/wit",
    world: "agent",
});

struct Stub;

impl Guest for Stub {
    fn parse_transcript(account: String, session: String) -> Option<Vec<Entry>> {
        let record = std::fs::read_to_string(format!("{account}/memory/{session}.jsonl")).ok()?;

        Some(
            record
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .map(|line: serde_json::Value| entry(&line))
                .collect(),
        )
    }

    fn write_configuration(found: Vec<Found>, servers: Vec<Server>) -> Configuration {
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
fn entry(line: &serde_json::Value) -> Entry {
    let text = |key: &str| line[key].as_str().unwrap_or_default().to_owned();

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
