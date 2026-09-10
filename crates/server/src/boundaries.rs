//! How long a Conversation's boundary lasts on the platform whose boundary is
//! an identity: written at its first session, taken away with its Worktree, and
//! swept for at startup.
//!
//! **Because an access-control entry is not around a process.** A grant is a
//! change to a directory on the human's own disk and the identity it names is a
//! local account of Verkstead's own — see [`crate::sandbox::entries`] — so
//! nothing of it goes when the session goes, and nothing of it goes when
//! Verkstead does. What ends one is somebody deciding it has ended, which is
//! what this module is.
//!
//! **One set of entries per Conversation** (ADR-0014, Q14), which is the whole
//! of what makes one Conversation's session refused another's Worktree once that
//! Conversation has stopped — there is one account for the installation, so the
//! entries are all the difference there is. They are written at the first
//! session, shared by every session and Conversation Terminal after it, and let
//! go of here.
//!
//! **Closing takes them with the Worktree.** The close already removes the
//! Worktree, the companions' checkouts and the handoff directory — see
//! [`crate::conversations::close`] — and a boundary around a Conversation that
//! has stopped is nothing but entries on somebody's directories granting a
//! checkout nothing will ever run in again.
//!
//! **And the server sweeps at startup**, beside the sweep that reclaims the
//! orphaned worktrees. What a close does not reach is what no close ran for: a
//! server that died holding a Conversation's entries left every one of them
//! standing, and nothing but this will ever look at them. So every record under
//! the Data Directory whose Conversation has stopped — Done, Closed, or gone
//! from the record altogether — is taken back.

//!
//! **A Done Conversation is swept and a Done Conversation keeps its Worktree**,
//! which is the one place these two sweeps read the same word differently. Done
//! is not the end of the work: a Follow-up steer picks it up in the checkout it
//! was left in, which is why [`crate::worktrees`] keeps the directory. What it
//! does not need kept is its reach — the session that steer starts describes
//! its surface and grants it again.

//!
//! **What can go wrong here is worse than what it fixes**, so this takes the
//! shape the worktree sweep takes: a reading that failed strips nothing, every
//! candidate comes out of reading one directory of Verkstead's own, and every
//! entry removed is one this server can say Verkstead wrote — because it is
//! reading back what the server that wrote it wrote down.

//!
//! **Built on every machine**, for the reason [`crate::sandbox::granting`] is:
//! what a sweep decides is a fact about the records and the store, and only the
//! taking-back of an entry is a call one platform has. There are no records at
//! all on the two platforms whose sandbox is a wrapper, so what this does there
//! is read an empty directory.

use std::path::Path;

use crate::store;

use crate::sandbox::granting::remembering;

/// The boundary of `conversation`, taken off this machine — its entries and the
/// record of them.
///
/// What a close calls, and what the sweep below calls for each boundary it has
/// decided about. Blocks, so it is called off the runtime.
pub fn remove(data_dir: &Path, conversation: i64) {
    crate::sandbox::taken_back(data_dir, conversation);
}

/// The same for a Conversation that is closing, off the runtime's threads.
///
/// Nothing is refused and nothing comes back: a close is not a thing to be
/// stopped by an entry that would not come off — the next sweep finds it — and a
/// Conversation nothing could ever end would be worse than an entry left on a
/// directory for an hour.
pub(crate) async fn closing(state: &crate::AppState, conversation: i64) {
    let data_dir = state.data_dir.clone();

    if let Err(error) = tokio::task::spawn_blocking(move || remove(&data_dir, conversation)).await {
        tracing::error!(
            error = ?error,
            conversation_id = conversation,
            "taking back the boundary of a Conversation that is closing failed",
        );
    }
}

/// Sweep once, as the server comes up.
///
/// Started rather than waited on, [`crate::worktrees::at_startup`]'s shape:
/// what it is deciding about is what the *last* server left, and nothing this
/// one does is racing it — a boundary is begun by a session start, and the
/// record naming it is written before anything is granted for it, so a boundary
/// begun while this is running is one whose Conversation is in the keep-set this
/// read.
pub(crate) fn at_startup(state: &crate::AppState) {
    let pool = state.pool.clone();
    let data_dir = state.data_dir.clone();

    tokio::spawn(async move { swept(&pool, &data_dir).await });
}

/// The sweep itself, off what it needs rather than off the whole of the server:
/// the store to ask which Conversations are still working, and the Data
/// Directory the records are under.
///
/// **The keep-set is read first and acted on after**, and a reading that failed
/// ends the pass: a store error read as an empty keep-set would be every
/// Conversation there is, and what that would take back is the boundary of work
/// somebody is still doing. The unrecoverable mistake is taking away a live
/// Conversation's reach; an entry left standing is swept again at the next
/// start.
pub async fn swept(pool: &sqlx::SqlitePool, data_dir: &Path) {
    // A router with no Data Directory has nowhere to have written a record, and
    // the empty path would resolve to the working directory — which is somebody
    // else's. See [`crate::nowhere`].
    if data_dir.as_os_str().is_empty() {
        return;
    }

    let listing = data_dir.to_owned();

    // Every part of this blocks — a directory read, a tree walked per entry
    // taken back — so it goes off the runtime's threads.

    let found = match tokio::task::spawn_blocking(move || remembering::left_behind(&listing)).await
    {
        Ok(found) => found,
        Err(error) => {
            tracing::error!(error = ?error, "listing the boundaries left behind failed");

            return;
        }
    };

    if found.is_empty() {
        return;
    }

    let kept = match store::unfinished_conversations(pool).await {
        Ok(kept) => kept,
        Err(error) => {
            tracing::error!(
                error = ?error,
                "reading which Conversations are still working failed, so no boundaries are \
                 being swept",
            );

            return;
        }
    };

    let stopped = stopped(&found, &kept);

    if stopped.is_empty() {
        return;
    }

    let data_dir = data_dir.to_owned();

    if let Err(error) = tokio::task::spawn_blocking(move || {
        for conversation in stopped {
            tracing::info!(
                conversation_id = conversation,
                "the Conversation these entries were written for has stopped, so every one of \
                 them is being taken off this machine",
            );

            remove(&data_dir, conversation);
        }
    })
    .await
    {
        tracing::error!(error = ?error, "sweeping the boundaries left behind failed");
    }
}

/// Which of the boundaries `found` are to go: every one whose Conversation is
/// not in `kept`.
///
/// **A record is kept exactly when its Conversation is still working**, which is
/// [`store::unfinished_conversations`] read backwards and is the whole of the
/// rule. A Conversation that has stopped is Done or Closed; one the record has
/// lost altogether is a Cleanup's delete, which takes every row and leaves a
/// Windows machine carrying the entries — and that is as much an orphan as the
/// other two.
fn stopped(found: &[remembering::Remembered], kept: &[i64]) -> Vec<i64> {
    found
        .iter()
        .map(|remembered| remembered.conversation)
        .filter(|conversation| !kept.contains(conversation))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::store::Lifecycle;

    use crate::sandbox::granting::remembering::Remembered;

    /// A store with a Repo in it, ready for Conversations, and which Repo that
    /// is.
    async fn database(data_dir: &Path) -> (sqlx::SqlitePool, i64) {
        let pool = store::open_database(&data_dir.join("verkstead.db"))
            .await
            .expect("a database under the Data Directory");

        let path = data_dir.join("repo");
        std::fs::create_dir_all(&path).unwrap();

        let repo = store::register_repo(&pool, &path, "verkstead", "main")
            .await
            .unwrap()
            .expect("the Repo registers");

        (pool, repo.id)
    }

    /// A Conversation of `repo`'s in `state`, with a boundary of its own
    /// written down — which is what a sweep finds and decides about.
    async fn conversation(
        pool: &sqlx::SqlitePool,
        data_dir: &Path,
        repo: i64,
        state: Lifecycle,
    ) -> i64 {
        let id = store::start_conversation(pool, repo, "rate-limiting")
            .await
            .unwrap()
            .expect("the Conversation starts");

        store::set_state(pool, id, state).await.unwrap();

        remembering::wrote(
            data_dir,
            &Remembered {
                conversation: id,
                account: "vk-0123456789ab".to_owned(),
                sid: "S-1-5-21-1-2-3-1001".to_owned(),
                entries: Vec::new(),
                cut: Vec::new(),
            },
        )
        .expect("a record to be writable under the Data Directory");

        id
    }

    /// Whose records are still there, by Conversation.
    fn remembered(data_dir: &Path) -> Vec<i64> {
        let mut found: Vec<i64> = remembering::left_behind(data_dir)
            .iter()
            .map(|remembered| remembered.conversation)
            .collect();

        found.sort_unstable();
        found
    }

    /// The whole of the rule, in one pass: the work that has stopped loses its
    /// boundary and the work that has not keeps it.
    ///
    /// Done as well as Closed, which is this sweep's own reading — a Done
    /// Conversation keeps the checkout a Follow-up would pick up and does not
    /// keep its reach, because the session that steer starts grants what it
    /// needs again.
    #[tokio::test]
    async fn a_boundary_whose_conversation_has_stopped_goes_and_a_working_one_stays() {
        let held = tempfile::tempdir().unwrap();
        let (pool, repo) = database(held.path()).await;

        let grilling = conversation(&pool, held.path(), repo, Lifecycle::Grilling).await;
        let done = conversation(&pool, held.path(), repo, Lifecycle::Done).await;
        let closed = conversation(&pool, held.path(), repo, Lifecycle::Closed).await;

        swept(&pool, held.path()).await;

        assert_eq!(
            remembered(held.path()),
            vec![grilling],
            "the Conversation still working should be the only one left with a boundary, and \
             {done} and {closed} should have lost theirs",
        );
    }

    /// And a Conversation the record has lost altogether, which is what a
    /// Cleanup's delete leaves behind: rows gone, entries still on the machine.
    #[tokio::test]
    async fn a_boundary_whose_conversation_is_no_longer_in_the_record_goes_too() {
        let held = tempfile::tempdir().unwrap();
        let (pool, _repo) = database(held.path()).await;

        remembering::wrote(
            held.path(),
            &Remembered {
                conversation: 404,
                account: "vk-0123456789ab".to_owned(),
                sid: "S-1-5-21-1-2-3-1001".to_owned(),
                entries: Vec::new(),
                cut: Vec::new(),
            },
        )
        .unwrap();

        swept(&pool, held.path()).await;

        assert_eq!(remembered(held.path()), Vec::<i64>::new());
    }

    /// A reading that failed takes nothing back, which is the rule every sweep
    /// here keeps: an empty keep-set is every live Conversation there is.
    #[tokio::test]
    async fn a_keep_set_that_could_not_be_read_sweeps_nothing() {
        let held = tempfile::tempdir().unwrap();
        let (pool, repo) = database(held.path()).await;

        let grilling = conversation(&pool, held.path(), repo, Lifecycle::Grilling).await;
        let done = conversation(&pool, held.path(), repo, Lifecycle::Done).await;

        // The column the keep-set is read by, taken out from under it: any way
        // of failing would do, and this is the one a test can arrange without
        // the rest of the record hanging off the table it took.
        sqlx::query("ALTER TABLE conversations DROP COLUMN state")
            .execute(&pool)
            .await
            .expect("the column to be droppable");

        swept(&pool, held.path()).await;

        assert_eq!(
            remembered(held.path()),
            vec![grilling, done],
            "a store that could not say which Conversations are working should have left \
             every boundary exactly where it was",
        );
    }

    /// And a close takes one Conversation's, whatever the rest of the record
    /// says.
    #[tokio::test]
    async fn closing_a_conversation_takes_its_entries() {
        let held = tempfile::tempdir().unwrap();
        let (pool, repo) = database(held.path()).await;

        let closing = conversation(&pool, held.path(), repo, Lifecycle::Grilling).await;
        let working = conversation(&pool, held.path(), repo, Lifecycle::Grilling).await;

        remove(held.path(), closing);

        assert_eq!(remembered(held.path()), vec![working]);
    }
}
