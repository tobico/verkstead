//! The backlog a Conversation's Worktree holds, read back as the task-list
//! Event.
//!
//! Nothing here is stored. `.tasks/` is the repository's — written by the
//! breaking-down session and rewritten by every session that finishes a task —
//! so the Event is a reading of the Worktree as it stands rather than a row
//! somebody remembered to keep up to date. That is what makes it worth pinning:
//! a record of the backlog *as it was* would be one more thing to be wrong,
//! where this cannot disagree with the branch it is read off.
//!
//! What *is* stored is where the backlog landed — one row with nothing on it,
//! written when the branch first carried one, so that the Timeline has a place
//! to draw the card as well as a block to pin it in. The reading here is what
//! is drawn in both, so the two are one card in two places rather than two
//! answers to the same question. See `store::record_backlog`.
//!
//! Two files say what the list is, and each says a different half of it.
//! `TODO.md` holds the entries — their order, their numbers and their titles —
//! and the `NN-<slug>.md` files say which of them are still outstanding, because
//! finishing a task is what deletes one. So a checkbox is how an entry is
//! written rather than what says it is done: what says that is the file being
//! gone, which is the done-signal the whole task runner turns on.
//!
//! Which is the one place a backlog and a roadmap differ in the reading — see
//! [`crate::stages`], which reads the same lines off `ROADMAP.md` and does take
//! the box at its word.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use verkstead_render::{TaskEntry, TaskListEvent};

use crate::checklist;

/// Where a Conversation's backlog lives inside its Worktree.
pub(crate) const BACKLOG: &str = ".tasks";

/// The list itself, inside that directory.
pub(crate) const TODO: &str = "TODO.md";

/// The backlog a Conversation's Timeline draws, where its Worktree holds one.
///
/// `None` where a Conversation has no Worktree, where the Worktree has no
/// `.tasks/`, or where what is there is not a list this can read. All three are
/// the same thing to draw: no card, in either of the two places one goes.
///
/// One reading behind both of them — the pinned block and the row on the record
/// where the backlog landed — because the card is the same card, and a page that
/// read the directory twice could draw two backlogs that disagreed.
///
/// Blocking work, so it happens off the runtime's threads — this is a directory
/// read and a file read per Conversation the human opens.
pub(crate) async fn showing(worktree: Option<PathBuf>) -> Option<TaskListEvent> {
    let worktree = worktree?;

    match tokio::task::spawn_blocking(move || backlog(&worktree)).await {
        Ok(list) => list,
        Err(error) => {
            tracing::error!(error = ?error, "reading a Worktree's backlog failed");
            None
        }
    }
}

/// The backlog `worktree` holds, or `None` where there is none to show.
///
/// A `TODO.md` with no entries in it comes back as `None` rather than as an
/// empty list: what would be pinned is a heading over nothing, and a
/// Conversation whose backlog cannot be read should read the same as one that
/// has no backlog — there is nothing for the human to do about either.
fn backlog(worktree: &Path) -> Option<TaskListEvent> {
    let backlog = worktree.join(BACKLOG);

    let list = match std::fs::read_to_string(backlog.join(TODO)) {
        Ok(list) => list,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            // Worth saying: a `.tasks/` that is there and will not be read is a
            // different thing from one that was never written, even though the
            // Timeline draws them the same way.
            tracing::warn!(
                error = ?error,
                backlog = %backlog.display(),
                "a Worktree's TODO.md could not be read",
            );
            return None;
        }
    };

    let outstanding = outstanding(&backlog);

    // The box says nothing here: what says a task is done is its file being
    // gone — see the module docs.
    let tasks: Vec<TaskEntry> = list
        .lines()
        .filter_map(checklist::entry)
        .map(|entry| TaskEntry {
            number: entry.label.to_owned(),
            title: entry.title.to_owned(),
            done: !outstanding.contains(&entry.number),
        })
        .collect();

    if tasks.is_empty() {
        return None;
    }

    Some(verkstead_render::task_list(
        checklist::heading(&list),
        tasks,
    ))
}

/// The numbers of the task files still in the backlog directory.
///
/// A directory that will not be read comes back empty, which reads as every
/// task being done. That is the right way round: `TODO.md` was read a moment
/// ago, so a `.tasks/` with nothing listable in it is one whose task files have
/// all gone.
fn outstanding(backlog: &Path) -> HashSet<u32> {
    let Ok(listed) = std::fs::read_dir(backlog) else {
        return HashSet::new();
    };

    listed
        .flatten()
        .filter_map(|file| numbered(&file.file_name().to_string_lossy()))
        .collect()
}

/// The number a task file leads with — `05` of `05-pinned-task-list.md` — or
/// `None` where the name is not one the breaking-down session wrote.
///
/// `TODO.md` is refused by the same rule that refuses everything else: it leads
/// with no number. Nothing here has to know it by name.
///
/// Shared with [`crate::runner`], which decides what to run next by the same
/// reading: which task files are left is one fact about a Worktree, and a
/// Timeline that drew one set of tasks while the runner worked through another
/// would be two answers to one question.
pub(crate) fn numbered(name: &str) -> Option<u32> {
    let (number, slug) = name.strip_suffix(".md")?.split_once('-')?;

    if slug.is_empty() || number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    number.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A backlog exactly as the breaking-down session writes one.
    const LIST: &str = "\
# Implementation

Takes a Conversation from finished grilling to implemented work.

## Tasks

- [x] 01: Wrap-up proposal and the Direction state — [details](01-direction-state.md)
- [x] 02: Handoff document and inline execution — [details](02-inline-execution.md)
- [ ] 03: The pinned task-list Event — [details](03-pinned-task-list.md)
";

    /// A worktree with that list in it and `files` still to do.
    fn worktree(list: &str, files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join(BACKLOG);
        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(tasks.join(TODO), list).unwrap();

        for file in files {
            std::fs::write(tasks.join(file), "# a task\n").unwrap();
        }

        dir
    }

    /// The task list a worktree comes back with, which every test here wants.
    fn list(dir: &tempfile::TempDir) -> TaskListEvent {
        backlog(dir.path()).expect("there is a backlog to read")
    }

    #[test]
    fn the_entries_are_the_lists_own_order_numbers_and_titles() {
        let dir = worktree(LIST, &["03-pinned-task-list.md"]);
        let list = list(&dir);

        assert_eq!(list.feature, "Implementation");
        assert_eq!(
            list.tasks
                .iter()
                .map(|task| (task.number.as_str(), task.title.as_str()))
                .collect::<Vec<_>>(),
            [
                ("01", "Wrap-up proposal and the Direction state"),
                ("02", "Handoff document and inline execution"),
                ("03", "The pinned task-list Event"),
            ]
        );
    }

    /// The file going is what says a task is done — see the module docs — so
    /// this is the reading that matters most.
    #[test]
    fn a_task_is_done_when_its_file_has_gone() {
        let dir = worktree(LIST, &["03-pinned-task-list.md"]);

        assert_eq!(
            list(&dir)
                .tasks
                .iter()
                .map(|task| task.done)
                .collect::<Vec<_>>(),
            [true, true, false]
        );
    }

    /// And not the checkbox: a session that deleted its task file and never
    /// ticked the box has finished the task.
    #[test]
    fn a_file_that_has_gone_is_done_whatever_the_checkbox_says() {
        let dir = worktree("# Feature\n\n- [ ] 01: Something\n", &[]);

        assert!(list(&dir).tasks[0].done);
    }

    /// The other way round for the same reason: the box was ticked, the file
    /// was not deleted, and the task is still there to be done.
    #[test]
    fn a_ticked_entry_whose_file_is_still_there_is_not_done() {
        let dir = worktree("# Feature\n\n- [x] 01: Something\n", &["01-something.md"]);

        assert!(!list(&dir).tasks[0].done);
    }

    #[test]
    fn a_worktree_with_no_backlog_has_no_task_list() {
        let dir = tempfile::tempdir().unwrap();

        assert!(backlog(dir.path()).is_none());
    }

    /// A directory of task files and no list is not a list. `TODO.md` is what
    /// the entries are read from, and there is nothing to draw without it.
    #[test]
    fn task_files_without_a_list_are_not_one() {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join(BACKLOG);
        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(tasks.join("01-something.md"), "# a task\n").unwrap();

        assert!(backlog(dir.path()).is_none());
    }

    #[test]
    fn a_list_with_no_entries_in_it_is_nothing_to_pin() {
        let dir = worktree("# Feature\n\nNothing broken down yet.\n", &[]);

        assert!(backlog(dir.path()).is_none());
    }

    #[test]
    fn only_numbered_checkboxes_are_entries() {
        let dir = worktree(
            "# Feature\n\n\
             - [ ] not a task at all\n\
             - [ ] 01: A task\n\
             - a plain bullet\n",
            &[],
        );

        let list = list(&dir);

        assert_eq!(list.tasks.len(), 1);
        assert_eq!(list.tasks[0].title, "A task");
    }

    #[test]
    fn a_list_with_no_heading_is_still_a_list() {
        let dir = worktree("- [ ] 01: A task\n", &["01-a-task.md"]);
        let list = list(&dir);

        assert_eq!(list.feature, "");
        assert_eq!(list.tasks.len(), 1);
    }

    #[test]
    fn a_file_that_is_not_a_numbered_task_is_not_one() {
        assert_eq!(numbered("05-pinned-task-list.md"), Some(5));
        assert_eq!(numbered("TODO.md"), None);
        assert_eq!(numbered("05.md"), None);
        assert_eq!(numbered("notes-05.md"), None);
        assert_eq!(numbered("05-pinned-task-list.txt"), None);
    }
}
