//! A `provides:` entry of the `forge` Category, read and refused by field —
//! the grammar `docs/plugins.md` writes under *Forge*.
//!
//! **Everything here is data** (ADR-0030, *Forge*): the hosts a Forge answers
//! for, the noun it calls a pull request, how git inside the Sandbox
//! authenticates against it, the tool a session is lent, what the Forge can do
//! and the scopes a token needs for it. What a Forge *does* — find a pull
//! request, read its checks, publish a Share — is its component's, and the
//! manifest only says which of those things it has.

use serde_json::{Map, Value};

use super::{Refusal, entry, text_of, within, words};

/// The keys a forge's entry holds, its `category` aside.
const KEYS: &[&str] = &[
    "hosts",
    "noun",
    "credential",
    "tool",
    "capabilities",
    "scopes",
];

/// A Forge, as a plugin's manifest declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forge {
    /// The hosts a Repo's remote names to be on this Forge, `github.com`.
    pub hosts: Vec<String>,
    /// What it calls a pull request, which the viewer says wherever a Repo on
    /// it is drawn: *pull request*, *merge request*.
    pub noun: String,
    pub credential: Credential,
    /// The command-line tool a session is lent, where it has one.
    pub tool: Option<Tool>,
    /// What it can do, each once, in the order written.
    pub capabilities: Vec<Capability>,
    /// The scopes a token needs for everything Verkstead asks of it.
    pub scopes: Vec<String>,
}

/// How git inside the Sandbox authenticates against the Forge: the username
/// its credential helper answers with, the token being the password, and the
/// hosts the helper answers for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub username: String,
    /// The forge's own `hosts` where the entry names none.
    pub hosts: Vec<String>,
}

/// The tool a session is lent, and the environment variable it reads its token
/// from: `gh` and `GH_TOKEN`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub name: String,
    pub token: String,
}

/// Something a Forge can do, in ADR-0030's words. One it does not declare is
/// settled rather than failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Running checks against a pull request.
    Checks,
    /// Comments and reviews on a pull request.
    Review,
    /// Publishing a Share.
    Publish,
    /// Stacked pull requests.
    Stacks,
    /// Creating a repository.
    Create,
}

impl Capability {
    pub const ALL: [Capability; 5] = [
        Capability::Checks,
        Capability::Review,
        Capability::Publish,
        Capability::Stacks,
        Capability::Create,
    ];

    /// The word a manifest writes.
    pub fn word(self) -> &'static str {
        match self {
            Capability::Checks => "checks",
            Capability::Review => "review",
            Capability::Publish => "publish",
            Capability::Stacks => "stacks",
            Capability::Create => "create",
        }
    }
}

/// A forge's entry, written at `field`, read.
pub(super) fn forge(body: &Map<String, Value>, field: &str) -> Result<Forge, Refusal> {
    if let Some(key) = body.keys().find(|key| !KEYS.contains(&key.as_str())) {
        return Err(Refusal::of(
            &format!("{field}.{key}"),
            format!("`{field}.{key}` isn't a key a forge has."),
        ));
    }
    let at = |key: &str| format!("{field}.{key}");

    within(body, field, "hosts")?;
    let hosts = words(body, field, "hosts")?;
    if hosts.is_empty() {
        return Err(Refusal::of(
            &at("hosts"),
            format!(
                "`{}` is empty. Name every host the forge answers for, like `github.com`.",
                at("hosts")
            ),
        ));
    }

    let noun = text_of(within(body, field, "noun")?, &at("noun"))?;
    let credential = credential(
        within(body, field, "credential")?,
        &at("credential"),
        &hosts,
    )?;
    let tool = match body.get("tool") {
        Some(Value::Null) | None => None,
        Some(value) => Some(tool(value, &at("tool"))?),
    };

    let mut capabilities: Vec<Capability> = Vec::new();
    for (index, word) in words(body, field, "capabilities")?.iter().enumerate() {
        let at = format!("{field}.capabilities[{index}]");
        let Some(capability) = Capability::ALL
            .into_iter()
            .find(|capability| capability.word() == word)
        else {
            return Err(Refusal::of(
                &at,
                format!(
                    "`{at}` is `{word}`, which isn't something a forge can do. It's one of \
                     checks, review, publish, stacks, or create."
                ),
            ));
        };
        if capabilities.contains(&capability) {
            return Err(Refusal::of(
                &at,
                format!("`{at}` is `{word}`, which the list already names."),
            ));
        }
        capabilities.push(capability);
    }

    Ok(Forge {
        hosts,
        noun,
        credential,
        tool,
        capabilities,
        scopes: words(body, field, "scopes")?,
    })
}

fn credential(value: &Value, field: &str, hosts: &[String]) -> Result<Credential, Refusal> {
    let map = entry(value, field, &["username", "hosts"])?;
    let username = text_of(
        within(map, field, "username")?,
        &format!("{field}.username"),
    )?;
    let named = words(map, field, "hosts")?;
    Ok(Credential {
        username,
        hosts: if named.is_empty() {
            hosts.to_vec()
        } else {
            named
        },
    })
}

fn tool(value: &Value, field: &str) -> Result<Tool, Refusal> {
    let map = entry(value, field, &["name", "token"])?;
    let name = text_of(within(map, field, "name")?, &format!("{field}.name"))?;
    let token_field = format!("{field}.token");
    let token = text_of(within(map, field, "token")?, &token_field)?;
    if !super::is_variable(&token) {
        return Err(Refusal::of(
            &token_field,
            format!("`{token_field}` should be an environment variable's name, like `GH_TOKEN`."),
        ));
    }
    Ok(Tool { name, token })
}
