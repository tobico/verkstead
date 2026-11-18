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
//! - **`login`** is over as soon as it is asked.

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

    fn login(_written: String) -> LoginStep {
        LoginStep::Done
    }
}

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
