//! The index the Antigravity CLI keeps of its conversations, read from outside
//! the program that is writing it.
//!
//! agy takes no conversation id at launch, so nothing Verkstead knows before a
//! session starts names its conversation — the Codex and OpenCode case. What
//! names it afterwards is `conversation_summaries.db` beside the conversations:
//! a row per conversation, with the directories it was working in and when it
//! was last written. The session's is the row naming this Worktree and written
//! at or after launch, the newest of them — Codex's rule against a third store.
//!
//! **Only the id is read here.** The record of what was said is a file of lines
//! under `brain/`, named for that id, and following it is the ordinary file
//! follower's work — see [`crate::transcript`]. The database answers which
//! conversation; it is not the Transcript.
//!
//! **Read as an outsider**, for [`crate::records`]' reason: the database belongs
//! to a program that is writing it, so it is opened read-only and never
//! written, and a poll that cannot read is a poll that looks again. A database
//! of a shape this build cannot read finds no conversation, which leaves the
//! session Capture-only — said in the log by the caller.

use std::path::Path;
use std::time::{Duration, SystemTime};

use sqlx::ConnectOptions;
use sqlx::Connection;
use sqlx::sqlite::SqliteConnectOptions;

/// The conversation agy is having in `worktree`, launched at `launched`, as the
/// index at `database` says — or `None` where it says none yet.
///
/// The newest row naming the Worktree and written since the launch, and only a
/// conversation agy started for itself: a sub-agent's row names the same
/// directory, is newer than the conversation that spawned it, and carries its
/// parent's id, so it is left out.
///
/// `None` is also every way of not being able to ask — no database yet, the
/// writer holding it, a shape that has moved — because each is the next poll's
/// to ask again.
pub(crate) async fn conversation(
    database: &Path,
    worktree: &Path,
    launched: SystemTime,
) -> Option<String> {
    if !tokio::fs::metadata(database)
        .await
        .is_ok_and(|found| found.is_file())
    {
        return None;
    }

    let mut reading = SqliteConnectOptions::new()
        .filename(database)
        .read_only(true)
        .create_if_missing(false)
        .busy_timeout(WAIT_FOR_THE_WRITER)
        .connect()
        .await
        .ok()?;

    // agy's own spelling of its own index, which is why it is written out
    // rather than built. The time is compared as seconds because agy writes it
    // as text with nanoseconds and an offset, which SQLite reads either way.
    let rows: Result<Vec<(String, String)>, sqlx::Error> = sqlx::query_as(
        "SELECT conversation_id, workspace_uris FROM conversation_summaries
          WHERE parent_conversation_id = ''
            AND unixepoch(last_modified_time) >= ?
          ORDER BY unixepoch(last_modified_time, 'subsec') DESC",
    )
    .bind(to_the_second(launched))
    .fetch_all(&mut reading)
    .await;

    let _ = reading.close().await;

    rows.ok()?
        .into_iter()
        .find(|(_, workspaces)| names(workspaces, worktree))
        .map(|(id, _)| id)
}

/// Whether a row's `workspace_uris` — a JSON list of `file://` addresses —
/// names `worktree`.
///
/// Read back as paths rather than the Worktree written out as an address,
/// because how agy spells a path as a URI is agy's business: what this needs is
/// the path each one stands for.
fn names(workspaces: &str, worktree: &Path) -> bool {
    let Ok(workspaces) = serde_json::from_str::<Vec<String>>(workspaces) else {
        return false;
    };

    let worktree = worktree.to_string_lossy();

    workspaces
        .iter()
        .filter_map(|uri| uri.strip_prefix(FILE))
        .filter_map(crate::transcript::decoded)
        .any(|path| stands_for(&path, &worktree))
}

/// Whether `path`, read out of a workspace's address, is the path `worktree`.
///
/// **A Windows path is spelt in an address the way a Unix one is**: a slash in
/// front of the drive, forward slashes after it — `file:///C:/Users/…` — and
/// the drive's colon sometimes escaped and its letter sometimes lowered, as in
/// `file:///c%3A/Users/…`. So a path that is a drive once that slash is off is
/// compared as Windows compares one, slashes either way and case aside, and
/// any other path exactly as it was written.
fn stands_for(path: &str, worktree: &str) -> bool {
    match on_a_drive(path) {
        Some(path) => windows(path) == windows(worktree),
        None => Path::new(path) == Path::new(worktree),
    }
}

/// `path` without the slash an address puts in front of a drive, where it is a
/// path on one.
fn on_a_drive(path: &str) -> Option<&str> {
    let path = path.strip_prefix('/').unwrap_or(path);

    match path.as_bytes() {
        [letter, b':'] | [letter, b':', b'/' | b'\\', ..] if letter.is_ascii_alphabetic() => {
            Some(path)
        }
        _ => None,
    }
}

/// A Windows path as it compares: one separator, one case, no separator on
/// the end.
fn windows(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

/// What a workspace's address opens with, before the path. Somebody else's
/// spelling, kept in one place.
const FILE: &str = "file://";

/// How long a poll waits on the writer before it gives up and asks again: under
/// the cadence it is polled on, as [`crate::records`]' is.
const WAIT_FOR_THE_WRITER: Duration = Duration::from_millis(250);

/// A moment as whole seconds since the epoch, floored, for the reason the
/// rollout finder floors its own: a row written in the second the session was
/// launched must not look older than the session.
fn to_the_second(moment: SystemTime) -> i64 {
    moment
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row of agy 1.4.12's index, as `last_modified_time` and the list of
    /// workspaces it wrote, in the layout it wrote them in.
    async fn index(rows: &[(&str, &str, &str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conversation_summaries.db");

        let mut writing = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .connect()
            .await
            .unwrap();

        // The columns this reads, out of the table agy 1.4.12 makes.
        sqlx::query(
            "CREATE TABLE `conversation_summaries` (`conversation_id` text,
               `last_modified_time` datetime NOT NULL, `workspace_uris` text NOT NULL,
               `parent_conversation_id` text NOT NULL DEFAULT '',
               PRIMARY KEY (`conversation_id`))",
        )
        .execute(&mut writing)
        .await
        .unwrap();

        for (id, modified, workspaces, parent) in rows {
            sqlx::query("INSERT INTO conversation_summaries VALUES (?, ?, ?, ?)")
                .bind(id)
                .bind(modified)
                .bind(workspaces)
                .bind(parent)
                .execute(&mut writing)
                .await
                .unwrap();
        }

        writing.close().await.unwrap();
        dir
    }

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    /// 2026-11-01 21:13:45 UTC, a launch the rows below are measured against.
    const LAUNCHED: u64 = 1_793_567_625;

    /// The session's conversation is the newest row naming its Worktree written
    /// since it was launched: not an earlier conversation in the same Worktree,
    /// not one in another directory, and not a sub-agent's.
    #[tokio::test]
    async fn the_newest_conversation_naming_the_worktree_since_the_launch_is_the_sessions() {
        let index = index(&[
            (
                "earlier",
                "2026-11-01 21:10:00.1+00:00",
                r#"["file:///srv/worktrees/verkstead-a"]"#,
                "",
            ),
            (
                "ours",
                "2026-11-01 21:14:00.308241717+00:00",
                r#"["file:///srv/worktrees/verkstead-a"]"#,
                "",
            ),
            (
                "elsewhere",
                "2026-11-01 21:15:00.5+00:00",
                r#"["file:///srv/worktrees/verkstead-b"]"#,
                "",
            ),
            (
                "its-subagent",
                "2026-11-01 21:16:00.5+00:00",
                r#"["file:///srv/worktrees/verkstead-a"]"#,
                "ours",
            ),
        ])
        .await;

        let database = index.path().join("conversation_summaries.db");

        assert_eq!(
            conversation(
                &database,
                Path::new("/srv/worktrees/verkstead-a"),
                at(LAUNCHED)
            )
            .await,
            Some("ours".to_owned()),
        );

        assert_eq!(
            conversation(
                &database,
                Path::new("/srv/worktrees/verkstead-c"),
                at(LAUNCHED)
            )
            .await,
            None,
        );
    }

    /// A workspace whose path has a character an address escapes is read back
    /// as the path it stands for.
    #[test]
    fn a_workspace_is_read_back_as_the_path_it_stands_for() {
        assert!(names(
            r#"["file:///srv/work%20trees/verkstead-a"]"#,
            Path::new("/srv/work trees/verkstead-a"),
        ));
        assert!(!names("not a list", Path::new("/srv")));
    }

    /// A Windows Worktree is found however the address spells its drive: a
    /// slash in front of it, its colon escaped or not, its letter either case —
    /// and a Unix path that merely looks like one is not mistaken for it.
    #[test]
    fn a_windows_workspace_is_read_back_as_the_path_it_stands_for() {
        let worktree = r"C:\Users\user\AppData\Roaming\Verkstead\worktrees\verkstead-a";

        for uri in [
            "file:///C:/Users/user/AppData/Roaming/Verkstead/worktrees/verkstead-a",
            "file:///c%3A/Users/user/AppData/Roaming/Verkstead/worktrees/verkstead-a",
            "file:///C:/Users/user/AppData/Roaming/Verkstead/worktrees/verkstead-a/",
            "file://C:%5CUsers%5Cuser%5CAppData%5CRoaming%5CVerkstead%5Cworktrees%5Cverkstead-a",
        ] {
            assert!(
                names(&serde_json::json!([uri]).to_string(), Path::new(worktree)),
                "{uri} names {worktree}"
            );
        }

        assert!(!names(
            r#"["file:///C:/Users/user/AppData/Roaming/Verkstead/worktrees/verkstead-b"]"#,
            Path::new(worktree),
        ));
        assert!(!names(
            r#"["file:///srv/C:/verkstead-a"]"#,
            Path::new(r"C:\verkstead-a"),
        ));
    }

    /// No index yet is no conversation yet, rather than an error.
    #[tokio::test]
    async fn no_index_is_no_conversation_yet() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            conversation(
                &dir.path().join("conversation_summaries.db"),
                Path::new("/srv"),
                at(LAUNCHED),
            )
            .await,
            None,
        );
    }
}
