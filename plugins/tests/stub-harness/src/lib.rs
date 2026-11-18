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
//! - **`parse-transcript`** finds nothing yet.
//! - **`login`** is over as soon as it is asked.

wit_bindgen::generate!({
    path: "../../../crates/server/wit",
    world: "agent",
});

struct Stub;

impl Guest for Stub {
    fn parse_transcript(_account: String, _session: String) -> Option<Vec<Entry>> {
        None
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
