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
//! and its checkbox is what says an entry is done, which is the done-signal the
//! whole task runner turns on. The `NN-<slug>.md` files beside it say what each
//! task *is*, and nothing about whether it is finished: a session that has not
//! written a document yet has not done the task, and a backlog half written
//! would otherwise read as a backlog nearly finished.
//!
//! One thing here writes rather than reads, and it writes on a branch rather
//! than into a record: a branch cut from a base that already carries a `.tasks/`
//! has that list taken away before any session runs — see [`clear`]. An
//! inherited list reads as a plan this branch wrote, which is a planning session
//! ended before it has asked anything.
//!
//! Which is the reading a roadmap makes too — see [`crate::stages`], which
//! reads the same lines off `ROADMAP.md` and takes the same box at its word.
//!
//! Those same two files are what the details pane is built from, one level
//! deeper: the entries say what the backlog is made of, and each `NN-<slug>.md`
//! is the document that entry names — see [`documents`], which reads them whole
//! rather than counting them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use verkstead_render::{BacklogPane, TaskEntry, TaskListEvent};

use crate::checklist;
use crate::repos::run;
use crate::settings::Author;

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

    // The box is the whole of it — see the module docs. Whatever is or is not
    // beside `TODO.md` says nothing about which tasks are done.
    let tasks: Vec<TaskEntry> = list
        .lines()
        .filter_map(checklist::entry)
        .map(|entry| TaskEntry {
            number: entry.label.to_owned(),
            title: entry.title.to_owned(),
            done: entry.checked,
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

/// The backlog opened: every task document of it, rendered, in the order the
/// list has them.
///
/// `None` for everything [`showing`] answers `None` to, and for the same reason
/// — no Worktree, no `.tasks/`, or nothing there this can read as a list. All
/// three are a pane with nothing to draw, which is a 404 at the route.
///
/// The entries come from `TODO.md` and the documents from the files beside it,
/// which is the same pair of readings the card is drawn from. A file is found by
/// the number its name leads with rather than by the link the entry carries: the
/// number is what the whole task runner turns on, and a link is a string out of
/// a file in a repository, which has no business being joined onto a path.
///
/// Blocking work, so it happens off the runtime's threads — this is a directory
/// read and a file read per task.
pub(crate) async fn documents(worktree: Option<PathBuf>) -> Option<BacklogPane> {
    let worktree = worktree?;

    match tokio::task::spawn_blocking(move || opened(&worktree)).await {
        Ok(pane) => pane,
        Err(error) => {
            tracing::error!(error = ?error, "reading a Worktree's task documents failed");
            None
        }
    }
}

/// The documents `worktree`'s backlog holds, or `None` where there is no backlog
/// to open — which is what [`backlog`] says `None` to, said the same way.
fn opened(worktree: &Path) -> Option<BacklogPane> {
    let backlog = worktree.join(BACKLOG);

    let list = std::fs::read_to_string(backlog.join(TODO)).ok()?;

    let files = files(&backlog);

    let tasks: Vec<verkstead_render::TaskSource> = list
        .lines()
        .filter_map(checklist::entry)
        .map(|entry| verkstead_render::TaskSource {
            number: entry.label.to_owned(),
            title: entry.title.to_owned(),
            done: entry.checked,
            // There for a done task as much as for one still to do: a task
            // file stays where it is until the feature is finished with, so a
            // section with nothing in it is the list naming a file nobody
            // wrote. Absent too where the file is there and will not be read,
            // which the pane draws the same way — there is nothing the human
            // can do about either from here.
            markdown: files
                .get(&entry.number)
                .and_then(|file| std::fs::read_to_string(backlog.join(file)).ok()),
        })
        .collect();

    if tasks.is_empty() {
        return None;
    }

    Some(verkstead_render::backlog_pane(
        checklist::heading(&list),
        tasks,
    ))
}

/// The task files in the backlog directory, by the number each of them leads
/// with.
///
/// The names rather than the numbers alone, because both readers open them: the
/// pane renders one per entry, and the runner hands the file it found to the
/// session that works it.
///
/// A directory that will not be read comes back empty, which says nothing about
/// what is done — that is the list's to say. What it does say is that there is
/// no document to open and nothing for a session to work from, which each
/// reader answers in its own way.
pub(crate) fn files(backlog: &Path) -> HashMap<u32, String> {
    let Ok(listed) = std::fs::read_dir(backlog) else {
        return HashMap::new();
    };

    listed
        .flatten()
        .filter_map(|file| {
            let name = file.file_name().to_string_lossy().into_owned();
            Some((numbered(&name)?, name))
        })
        .collect()
}

/// One entry of the list as the runner needs it: the number it answers to, the
/// label the list writes that number as, and whether its box is ticked.
///
/// [`checklist::Entry`] borrows the line it was read off, and the runner reads
/// the list, drops it, and then goes and does something about what it said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Listed {
    pub(crate) number: u32,
    pub(crate) label: String,
    pub(crate) checked: bool,
}

/// The entries of the backlog in `backlog`, in the list's own order, or `None`
/// where there is no list to read.
///
/// The same reading the card is drawn from, so the list the human is watching
/// and the list the runner is working are one list — see [`crate::runner`],
/// which decides what to run next by the boxes this comes back with.
///
/// `None` is a `.tasks/` with no `TODO.md` in it, or one that will not be read.
/// Both are a Conversation with no backlog to work, which is what the card says
/// about them too.
pub(crate) fn entries(backlog: &Path) -> Option<Vec<Listed>> {
    let list = std::fs::read_to_string(backlog.join(TODO)).ok()?;

    Some(
        list.lines()
            .filter_map(checklist::entry)
            .map(|entry| Listed {
                number: entry.number,
                label: entry.label.to_owned(),
                checked: entry.checked,
            })
            .collect(),
    )
}

/// The number a task file leads with — `05` of `05-pinned-task-list.md` — or
/// `None` where the name is not one the breaking-down session wrote.
///
/// `TODO.md` is refused by the same rule that refuses everything else: it leads
/// with no number. Nothing here has to know it by name.
///
/// Shared with [`crate::stages`], whose briefs are named the same way: one rule
/// for what a numbered document in a plan directory is called, because the
/// skills that write the two are forks of each other.
pub(crate) fn numbered(name: &str) -> Option<u32> {
    let (number, slug) = name.strip_suffix(".md")?.split_once('-')?;

    if slug.is_empty() || number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    number.parse().ok()
}

/// What became of clearing the task list a fresh branch inherited from its base.
///
/// The counts and the heading are read off the list before it goes, because
/// after the commit there is nothing left to read them off — and what they are
/// for is the notice, which says what was taken away rather than that something
/// was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Clearing {
    /// There was a list, and the branch now has a commit taking it away: what
    /// it was called, how many entries it had, and how many of those were still
    /// open.
    Cleared {
        /// `TODO.md`'s own heading, which is what the list called the work.
        /// Empty where it had none.
        heading: String,

        /// The entries whose box is not ticked.
        open: usize,

        /// And how many entries there were altogether.
        entries: usize,
    },

    /// The base carried none, so nothing was committed and there is nothing to
    /// say.
    Nothing,

    /// There was one and git would not be rid of it. The reason is in the
    /// server's log: what the press does about it is refuse, the way a worktree
    /// git would not make already refuses.
    Refused,
}

impl Clearing {
    /// What the Timeline is told, or nothing where no list was cleared.
    ///
    /// Every time it happens rather than only where the list was worth
    /// something: a branch that quietly lost a file it was cut with is the
    /// thing this whole clearing must never look like, so the record says what
    /// went and how far through it was.
    ///
    /// A list with no heading is said without one rather than with an empty
    /// bold: `****` on a Timeline is markdown nobody wrote.
    pub(crate) fn notice(&self, base: &str) -> Option<String> {
        let Clearing::Cleared {
            heading,
            open,
            entries,
        } = self
        else {
            return None;
        };

        let called = match heading.is_empty() {
            true => String::new(),
            false => format!(" **{heading}**,"),
        };

        Some(format!(
            "The base branch `{base}` carried a task list,{called} with {open} of {entries} \
             entries still open. It was cleared before this work started.",
        ))
    }
}

/// Take the task list `worktree` inherited from `base` away, as a commit of its
/// own by `author`.
///
/// A branch cut from a base that already carries a `.tasks/` reads as a branch
/// that has planned: what says a planning session is finished is a `TODO.md` at
/// the tip, and the watcher that ends one cannot tell an inherited list from a
/// written one. So the list goes before any session runs, on the fresh branch
/// rather than anywhere the base can see — the base is somebody else's work and
/// nothing here writes into it.
///
/// **Said on the command line rather than written into the repository's
/// config**, the way [`crate::repos`] commits a new repository's README and for
/// its reason: it is the same fact either way and one of them leaves a file
/// behind. No trailer either — the commit is Verkstead's own bookkeeping rather
/// than a session's work.
///
/// **Through [`crate::repos::run`] rather than [`crate::repos::git`]**, for the
/// reason a create's own commit goes through it: `git` is the runner for reads
/// and throws git's stderr away, and a [`Clearing::Refused`] takes the whole
/// press back — so the reason git gave is the only thing anybody could act on
/// afterwards, and it has to reach the log.
///
/// Blocking from end to end, and called where the checkouts are made.
pub(crate) fn clear(worktree: &Path, base: &str, author: &Author) -> Clearing {
    let backlog = worktree.join(BACKLOG);

    if !backlog.is_dir() {
        return Clearing::Nothing;
    }

    // Read before it goes, for the notice — see [`Clearing`]. A `.tasks/` with no
    // `TODO.md` in it still gets one, with the nothing it has: what the human
    // needs to know is that a directory was taken away.
    let list = std::fs::read_to_string(backlog.join(TODO)).unwrap_or_default();
    let entries: Vec<_> = list.lines().filter_map(checklist::entry).collect();

    let cleared = Clearing::Cleared {
        heading: checklist::heading(&list),
        open: entries.iter().filter(|entry| !entry.checked).count(),
        entries: entries.len(),
    };

    // `--` rather than `--end-of-options`: what follows is a pathspec, which is
    // git's own name for a path, and it is this module's constant rather than
    // anything a human typed.
    if let Err(said) = run(worktree, &["rm", "--quiet", "-r", "--", BACKLOG]) {
        tracing::error!(
            said,
            worktree = %worktree.display(),
            "the task list a branch inherited from its base could not be removed",
        );

        return Clearing::Refused;
    }

    let committed = run(
        worktree,
        &[
            "-c",
            &format!("user.name={}", author.name()),
            "-c",
            &format!("user.email={}", author.email()),
            "commit",
            "--quiet",
            "--message",
            &format!("chore: clear the task list inherited from {base}"),
        ],
    );

    if let Err(said) = committed {
        tracing::error!(
            said,
            worktree = %worktree.display(),
            "the removal of an inherited task list could not be committed",
        );

        return Clearing::Refused;
    }

    cleared
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

    /// The box is what says a task is done — see the module docs — so this is
    /// the reading that matters most.
    #[test]
    fn a_task_is_done_when_its_entry_is_ticked() {
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

    /// And not the file: a ticked entry is done with its document still sitting
    /// beside the list, which is every finished task until the feature is over.
    #[test]
    fn a_ticked_entry_is_done_with_its_file_still_there() {
        let dir = worktree("# Feature\n\n- [x] 01: Something\n", &["01-something.md"]);

        assert!(list(&dir).tasks[0].done);
    }

    /// The other way round for the same reason, and the case this is really
    /// for: a backlog part way through being written has entries whose
    /// documents nobody has got to yet, and none of those tasks is done.
    #[test]
    fn an_entry_with_no_file_yet_is_not_done() {
        let dir = worktree("# Feature\n\n- [ ] 01: Something\n", &[]);

        assert!(!list(&dir).tasks[0].done);
    }

    /// The whole of a backlog mid-write: `TODO.md` committed and not one
    /// document written. Every task is still to do, and a list that read them
    /// as finished would be a feature nobody had started showing as complete.
    #[test]
    fn a_backlog_being_written_reads_as_nothing_done_yet() {
        let dir = worktree(
            "# Feature\n\n- [ ] 01: First\n- [ ] 02: Second\n- [ ] 03: Third\n",
            &[],
        );

        assert!(list(&dir).tasks.iter().all(|task| !task.done));
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

    /// The same worktree, with what each task file actually says: the pane is
    /// about the documents rather than about which of them are there.
    fn documented(list: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join(BACKLOG);
        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(tasks.join(TODO), list).unwrap();

        for (file, written) in files {
            std::fs::write(tasks.join(file), written).unwrap();
        }

        dir
    }

    /// Every entry of the list, in its order, whatever became of its file: the
    /// pane is the backlog read at a second depth, so it has exactly the rows
    /// the card has.
    #[test]
    fn the_pane_holds_one_section_per_entry_in_the_lists_own_order() {
        let dir = documented(LIST, &[("03-pinned-task-list.md", "# a task\n")]);
        let pane = opened(dir.path()).expect("there is a backlog to open");

        assert_eq!(pane.feature, "Implementation");
        assert_eq!(
            pane.tasks
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

    /// What each of them holds: the file rendered where it is there, and nothing
    /// where the list names one nobody wrote.
    #[test]
    fn a_task_carries_its_document_wherever_there_is_one_to_read() {
        let dir = documented(
            LIST,
            &[(
                "03-pinned-task-list.md",
                "# The pinned task-list Event\n\nRead `.tasks/` off the Worktree.\n",
            )],
        );
        let pane = opened(dir.path()).unwrap();

        assert_eq!(
            pane.tasks[0].html, None,
            "the list names a file nobody wrote, and there is nothing to render",
        );
        assert_eq!(pane.tasks[1].html, None);

        let html = pane.tasks[2].html.as_deref().expect("that file is there");

        assert!(
            html.contains("<h1>The pinned task-list Event</h1>"),
            "rendered as markdown, like every other document on this wire: {html}",
        );
        assert!(html.contains("<code>.tasks/</code>"), "{html}");
    }

    /// The renderer in the page is loaded for the pane rather than for a task,
    /// so the flag is asked of all of them at once — and a backlog whose
    /// documents drew nothing never asks for mermaid at all.
    #[test]
    fn a_diagram_in_any_task_document_is_what_the_pane_draws_with() {
        let plain = documented(LIST, &[("03-pinned-task-list.md", "Just words.\n")]);

        assert!(!opened(plain.path()).unwrap().diagrams);

        let drawn = documented(
            LIST,
            &[(
                "03-pinned-task-list.md",
                "```mermaid\nflowchart LR\n  in --> out\n```\n",
            )],
        );
        let pane = opened(drawn.path()).unwrap();

        assert!(pane.diagrams);
        assert!(
            pane.tasks[2]
                .html
                .as_deref()
                .unwrap()
                .contains("<pre class=\"mermaid\">"),
            "held for the renderer in the page rather than drawn here",
        );
    }

    /// A done task keeps its document, and the section says it is done rather
    /// than standing empty — the way a roadmap's stages do. That is the whole
    /// of what a finished task looks like in the pane now.
    #[test]
    fn a_done_task_shows_its_document_marked_done() {
        let dir = documented(
            LIST,
            &[
                ("01-direction-state.md", "# 01. Wrap-up proposal\n"),
                ("03-pinned-task-list.md", "# 03. The pinned Event\n"),
            ],
        );
        let pane = opened(dir.path()).unwrap();

        assert!(pane.tasks[0].done, "the list ticks that entry off");
        assert!(
            pane.tasks[0]
                .html
                .as_deref()
                .is_some_and(|html| html.contains("<h1>01. Wrap-up proposal</h1>")),
            "and its document is still there to read",
        );

        assert!(!pane.tasks[2].done);
        assert!(pane.tasks[2].html.is_some());
    }

    /// A file left behind with nothing in it is the same as no file: what it
    /// would draw is a box with a gap in it.
    #[test]
    fn a_task_document_of_nothing_is_nothing_to_draw() {
        let dir = documented(LIST, &[("03-pinned-task-list.md", "\n   \n")]);

        assert_eq!(opened(dir.path()).unwrap().tasks[2].html, None);
    }

    /// The same three ways there is nothing to draw the card for are the three
    /// ways there is nothing to open, and the route answers all of them alike.
    #[test]
    fn a_worktree_with_no_backlog_has_nothing_to_open() {
        let bare = tempfile::tempdir().unwrap();
        assert!(opened(bare.path()).is_none());

        let empty = documented("# Feature\n\nNothing broken down yet.\n", &[]);
        assert!(opened(empty.path()).is_none());
    }

    /// A repository with `list` committed on `main`, standing in for the base a
    /// fresh branch is cut from — and a checkout of it is what a clearing works
    /// in.
    fn committed(list: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "base@verkstead.invalid"]);
        run(path, &["config", "user.name", "Somebody Else"]);

        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();

        if let Some(list) = list {
            let tasks = path.join(BACKLOG);

            std::fs::create_dir_all(&tasks).unwrap();
            std::fs::write(tasks.join(TODO), list).unwrap();
            std::fs::write(tasks.join("01-a-task.md"), "# 01. A task\n").unwrap();
        }

        run(path, &["add", "--all"]);
        run(path, &["commit", "-m", "first"]);

        dir
    }

    fn run(dir: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");

        String::from_utf8(output.stdout).unwrap()
    }

    /// Who every clearing here is committed as.
    fn author() -> Author {
        Author::configured(&crate::settings::GitAuthor::of(
            Some("Ada Lovelace".to_owned()),
            Some("ada@example.com".to_owned()),
        ))
        .expect("both halves are there")
    }

    /// The list goes, and it goes as a commit rather than as a change nobody
    /// took: the branch's tip is what the planning watcher reads, so a removal
    /// left in the working tree would be no removal at all.
    #[test]
    fn clearing_commits_the_removal_of_the_list_the_base_carried() {
        let dir = committed(Some(LIST));
        let path = dir.path();

        assert_eq!(
            clear(path, "main", &author()),
            Clearing::Cleared {
                heading: "Implementation".to_owned(),
                open: 1,
                entries: 3,
            },
        );

        assert!(!path.join(BACKLOG).exists(), "the directory is gone");
        assert_eq!(
            run(path, &["status", "--porcelain"]).trim(),
            "",
            "and nothing is left uncommitted",
        );
        assert_eq!(
            run(path, &["log", "-1", "--format=%s%n%an%n%ae"])
                .lines()
                .collect::<Vec<_>>(),
            [
                "chore: clear the task list inherited from main",
                "Ada Lovelace",
                "ada@example.com",
            ],
            "by the configured author rather than by whoever the checkout's git is",
        );
        assert_eq!(
            run(path, &["log", "--format=%H"]).lines().count(),
            2,
            "one commit on top of the base, and no more",
        );
    }

    /// And the author is said on the command line rather than written into the
    /// repository — a start that left a `user.name` behind would be a start that
    /// changed the checkout it was given.
    #[test]
    fn clearing_writes_no_identity_into_the_repository() {
        let dir = committed(Some(LIST));

        clear(dir.path(), "main", &author());

        let config = std::fs::read_to_string(dir.path().join(".git/config")).unwrap();

        assert!(!config.contains("Ada Lovelace"), "{config}");
        assert!(!config.contains("ada@example.com"), "{config}");
    }

    /// A base carrying no list is nothing to clear and nothing to commit: a
    /// branch cut for new work is the ordinary case, and it must arrive holding
    /// exactly what its base held.
    #[test]
    fn a_base_with_no_list_is_left_alone() {
        let dir = committed(None);
        let before = run(dir.path(), &["rev-parse", "HEAD"]);

        assert_eq!(clear(dir.path(), "main", &author()), Clearing::Nothing);
        assert_eq!(run(dir.path(), &["rev-parse", "HEAD"]), before);
    }

    /// What the Timeline is told: what the list was called, and how far through
    /// it was when it went.
    #[test]
    fn the_notice_says_what_was_cleared_and_how_far_through_it_was() {
        let dir = committed(Some(LIST));

        assert_eq!(
            clear(dir.path(), "origin/main", &author())
                .notice("origin/main")
                .as_deref(),
            Some(
                "The base branch `origin/main` carried a task list, **Implementation**, with 1 \
                 of 3 entries still open. It was cleared before this work started."
            ),
        );
    }

    /// A list with no heading and no entries still gets one, with what it has:
    /// what the human needs to know is that a directory was taken away.
    #[test]
    fn a_list_with_nothing_in_it_is_still_said() {
        let dir = committed(Some("Nothing broken down yet.\n"));

        assert_eq!(
            clear(dir.path(), "main", &author())
                .notice("main")
                .as_deref(),
            Some(
                "The base branch `main` carried a task list, with 0 of 0 entries still open. It \
                 was cleared before this work started."
            ),
        );
    }

    /// And nothing cleared is nothing said. A notice every start drew would be
    /// one nobody read.
    #[test]
    fn a_start_that_cleared_nothing_says_nothing() {
        assert_eq!(Clearing::Nothing.notice("main"), None);
        assert_eq!(Clearing::Refused.notice("main"), None);
    }

    /// And a git that will not be rid of the list is a [`Clearing::Refused`],
    /// with what git said about it in the log — which is the whole of what
    /// anybody has to go on, the press it refuses taking every checkout back.
    ///
    /// A directory that is no repository at all, because what the `rm` is
    /// refused for does not matter to what is done about it.
    #[test]
    fn a_git_that_will_not_be_rid_of_the_list_refuses_the_clearing() {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join(BACKLOG);

        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(tasks.join(TODO), LIST).unwrap();

        assert_eq!(clear(dir.path(), "main", &author()), Clearing::Refused);
        assert!(tasks.is_dir(), "and the list is where it was");
    }
}
