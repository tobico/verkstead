//! The small YAML envelopes the API answers with, kept beside the Set types so
//! the CLI reads back exactly what the server writes.

use serde::{Deserialize, Serialize};

use crate::validate::Violation;

// The refusal shape is the whole server's, viewer included — see `Response` for
// why the emitter is gated.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What `POST …/api/v1/sets` returns once a Set is stored: the identity the
/// server stamped on it, and whether there is anything to wait on. The CLI waits
/// on `id` where there is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCreated {
    pub id: i64,

    /// When the server accepted the Set, RFC 3339.
    pub created_at: String,

    /// Whether the server stored the Set rather than holding a wait open on it
    /// — a Deferred Ask, or an ask from a backend whose sessions cannot hold a
    /// shell command open for hours (ADR-0011).
    ///
    /// The server's word rather than the CLI's, because which channel a Set was
    /// asked on is a fact about the backend: the CLI asks the same way
    /// everywhere, and the server is what knows the agent type of the session
    /// that asked. A CLI old enough to ignore this opens a wait, which is the
    /// shipped-together case and not one to design around.
    #[serde(default)]
    pub stored: bool,
}

impl SetCreated {
    /// The stored Set as the CLI prints it: there is no Response coming to that
    /// agent, so what it is owed on stdout is that the Set was stored and which
    /// one it is.
    ///
    /// The identity and no more. Whether a wait was opened is how the CLI was
    /// answered rather than anything about the Set, and the agent reading this
    /// has the id it came for either way.
    pub fn to_yaml(&self) -> Result<String, serde_saphyr::SerializeError> {
        serde_saphyr::to_string(&Identity {
            id: self.id,
            created_at: &self.created_at,
        })
    }
}

/// The half of [`SetCreated`] that is about the Set, which is what is printed.
#[derive(Serialize)]
struct Identity<'a> {
    id: i64,
    created_at: &'a str,
}

/// What `POST …/api/v1/sets/{id}/response` returns once a Response is stored.
/// The human's device gets this; the agent gets the Response itself, off the
/// wait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAccepted {
    /// The Set that has just been answered.
    pub set_id: i64,

    /// When the server accepted the Response, RFC 3339.
    pub submitted_at: String,
}

/// The most a Memo may hold, in bytes of its markdown (ADR-0023).
///
/// Here rather than on either end, because both ends hold it: the CLI refuses
/// a longer one before sending anything, as a courtesy, and the server refuses
/// one sent by hand, as the rule.
pub const MAX_MEMO_BYTES: usize = 8 * 1024;

/// What `POST …/api/v1/memos` takes: the title the session names the Memo by,
/// and the markdown it wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoLeaving {
    /// Empty where a body sent by hand left it out, so that it is refused for
    /// having no title rather than for its shape.
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub markdown: String,
}

impl MemoLeaving {
    /// The Memo as it is stored: the title trimmed before anything is checked,
    /// and the markdown as it was written.
    pub fn new(title: &str, markdown: &str) -> Self {
        MemoLeaving {
            title: title.trim().to_owned(),
            markdown: markdown.to_owned(),
        }
    }

    /// Why the Memo is refused before it goes anywhere, or `None` where it may
    /// be left: an empty title, a newline in the title, nothing in the
    /// markdown, or markdown over [`MAX_MEMO_BYTES`]. A title has no length
    /// cap: the card cuts it to one line.
    pub fn refused(&self) -> Option<String> {
        if self.title.trim().is_empty() {
            return Some(
                "the memo has no title: name it in a handful of words for what it is about, \
                 such as \"The limiter's test flakes\""
                    .to_owned(),
            );
        }

        if self.title.contains(['\n', '\r']) {
            return Some(
                "the memo's title has a newline in it: it is one line on the Timeline, so put \
                 the rest in the memo"
                    .to_owned(),
            );
        }

        if self.markdown.trim().is_empty() {
            return Some(
                "the memo is empty: write what the sessions after this one should know".to_owned(),
            );
        }

        (self.markdown.len() > MAX_MEMO_BYTES).then(|| {
            format!(
                "the memo is {} bytes, over the maximum of {} (8 KiB): keep it to what the next \
                 session will walk into, and leave a second memo for a second thing",
                self.markdown.len(),
                MAX_MEMO_BYTES
            )
        })
    }
}

/// What `POST …/api/v1/memos` returns once a Memo is on the Timeline: the
/// Event it is, which is the number it goes by. The CLI prints this and nothing
/// else on stdout, so an agent can read it back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoLeft {
    pub id: i64,
}

/// The most a Preview's command may hold, in bytes (ADR-0024).
///
/// Here for [`MAX_MEMO_BYTES`]'s reason: the CLI refuses a longer one before
/// sending anything, as a courtesy, and the server refuses one sent by hand,
/// as the rule.
pub const MAX_PREVIEW_COMMAND_BYTES: usize = 4 * 1024;

/// The most a Preview's description may hold, in bytes (ADR-0024): a
/// paragraph or two, which is what its card holds.
pub const MAX_PREVIEW_DESCRIPTION_BYTES: usize = 2 * 1024;

/// What `POST …/api/v1/previews` takes: the title the Preview goes by, the
/// one line typed into a terminal to try the work, and the markdown its card
/// shows above that line, where there is any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewSuggestion {
    pub title: String,
    pub command: String,

    /// What the Preview is for, in more words than its title holds. Left off
    /// the wire where there is none, so a server from before descriptions
    /// takes a suggestion without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PreviewSuggestion {
    /// The suggestion as it is stored: the title trimmed, the command trimmed
    /// of the trailing newline a heredoc always ends in, and the description
    /// trimmed of its leading and trailing blank lines — and none at all where
    /// that leaves it blank — before anything is checked.
    pub fn new(title: &str, command: &str, description: Option<&str>) -> Self {
        PreviewSuggestion {
            title: title.trim().to_owned(),
            command: command.trim_end_matches(['\n', '\r']).to_owned(),
            description: description.and_then(trimmed_of_blank_lines),
        }
    }

    /// The suggestion `verkstead preview` reads off stdin, the way a commit
    /// message is read: the first line is the command, and what follows the
    /// first blank line is the description.
    ///
    /// A second line that is not blank leaves the whole of what was written
    /// standing as the command, newline and all, so that [`Self::refused`]
    /// refuses it as a command that runs onto a second line — a command wrapped
    /// by accident never becomes a description.
    pub fn read(title: &str, written: &str) -> Self {
        let written = written.trim_end_matches(['\n', '\r']);

        let Some((command, rest)) = written.split_once('\n') else {
            return PreviewSuggestion::new(title, written, None);
        };

        let (second, description) = rest.split_once('\n').unwrap_or((rest, ""));

        if second.trim().is_empty() {
            PreviewSuggestion::new(title, command, Some(description))
        } else {
            PreviewSuggestion::new(title, written, None)
        }
    }

    /// Why the suggestion is refused before it goes anywhere, or `None` where
    /// it may be left: an empty title, an empty command, a newline anywhere in
    /// the command, a command over [`MAX_PREVIEW_COMMAND_BYTES`], or a
    /// description over [`MAX_PREVIEW_DESCRIPTION_BYTES`].
    pub fn refused(&self) -> Option<String> {
        if self.title.trim().is_empty() {
            return Some(
                "the preview has no title: name it for what it tries, such as \"Build and test\""
                    .to_owned(),
            );
        }

        if self.command.trim().is_empty() {
            return Some(
                "the preview's command is empty: write the one line that tries the work".to_owned(),
            );
        }

        if self.command.contains(['\n', '\r']) {
            return Some(
                "the preview's command runs onto a second line: it is typed into a shell as one \
                 line, so join the steps with the shell's own separator, and leave a blank line \
                 after it before any description"
                    .to_owned(),
            );
        }

        if self.command.len() > MAX_PREVIEW_COMMAND_BYTES {
            return Some(format!(
                "the preview's command is {} bytes, over the maximum of {} (4 KiB): put the \
                 steps in a script and run that",
                self.command.len(),
                MAX_PREVIEW_COMMAND_BYTES
            ));
        }

        let description = self.description.as_deref().unwrap_or_default();

        (description.len() > MAX_PREVIEW_DESCRIPTION_BYTES).then(|| {
            format!(
                "the preview's description is {} bytes, over the maximum of {} (2 KiB): keep it \
                 to a paragraph or two saying what the preview is for",
                description.len(),
                MAX_PREVIEW_DESCRIPTION_BYTES
            )
        })
    }
}

/// `written` without its leading and trailing blank lines, or `None` where
/// nothing else is left. A line's own indentation stays, being markdown.
fn trimmed_of_blank_lines(written: &str) -> Option<String> {
    let lines: Vec<&str> = written
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let first = lines.iter().position(|line| !line.trim().is_empty())?;
    let last = lines.iter().rposition(|line| !line.trim().is_empty())?;

    Some(lines[first..=last].join("\n"))
}

/// What `POST …/api/v1/previews` returns once a Preview stands: its title,
/// which is what it goes by. The CLI prints this and nothing else on stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewLeft {
    pub title: String,
}

/// What the API returns when it refuses a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ApiError {
    /// One line saying what was refused.
    pub error: String,

    /// The grammar violations behind the refusal, each naming its question.
    /// Empty when the request failed for some other reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,
}

impl ApiError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            violations: Vec::new(),
        }
    }

    pub fn with_violations(error: impl Into<String>, violations: Vec<Violation>) -> Self {
        Self {
            error: error.into(),
            violations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PreviewSuggestion;

    /// What `verkstead preview` reads off `written`: the command and the
    /// description.
    fn read(written: &str) -> (String, Option<String>) {
        let read = PreviewSuggestion::read("Build and test", written);
        (read.command, read.description)
    }

    #[test]
    fn one_line_is_a_command_alone() {
        assert_eq!(read("cargo test\n"), ("cargo test".to_owned(), None));
        assert_eq!(read("cargo test"), ("cargo test".to_owned(), None));
        assert_eq!(read("cargo test\r\n"), ("cargo test".to_owned(), None));
    }

    /// What follows the blank line, trimmed of the blank lines around it, and
    /// none at all where only blank lines follow.
    #[test]
    fn a_blank_line_starts_the_description() {
        assert_eq!(
            read("cargo test\n\n\nRuns it.\n\n    indented\n\n\n"),
            (
                "cargo test".to_owned(),
                Some("Runs it.\n\n    indented".to_owned())
            ),
        );
        assert_eq!(
            read("cargo test\r\n\r\nRuns it.\r\n"),
            ("cargo test".to_owned(), Some("Runs it.".to_owned())),
        );
        assert_eq!(read("cargo test\n  \n \n"), ("cargo test".to_owned(), None));
    }

    /// A second line that is not blank stays in the command, to be refused.
    #[test]
    fn a_second_line_is_kept_in_the_command_and_refused() {
        let read = PreviewSuggestion::read("Build and test", "cargo build\ncargo test\n");

        assert_eq!(read.command, "cargo build\ncargo test");
        assert_eq!(read.description, None);
        assert!(
            read.refused()
                .is_some_and(|why| why.contains("runs onto a second line"))
        );
    }
}
