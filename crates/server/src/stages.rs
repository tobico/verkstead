//! The roadmaps a Conversation's Worktree holds, read back as pinned
//! stage-list Events.
//!
//! Nothing here is stored, for the reason nothing about a backlog is — see
//! [`crate::tasks`]. `docs/roadmaps/` is the repository's: written by the
//! roadmap direction's session, and rewritten by every stage that ticks itself
//! off as it finishes. So the Event is a reading of the Worktree as it stands
//! and cannot disagree with the branch it is read off.
//!
//! Where this parts company with a backlog is what says a stage is done. A task
//! is done when its file has gone, because a session deletes one as it lands;
//! a stage's brief stays where it is for ever, being the record of what the
//! stage was for. So the checkbox in `ROADMAP.md` is the whole of the answer
//! here, and it is what `/next-stage` reads too.
//!
//! And *which* roadmap is the other difference. A Worktree has one `.tasks/`
//! and may hold any number of roadmaps — a repository keeps the finished ones,
//! which is what they are for. The one that is this Conversation's is the one
//! its branch has written to, asked of git against the base commit the branch
//! came off: the session that wrote a roadmap wrote it here, and a stage that
//! ticks itself off ticks it here. Nothing is stored for it, so it cannot come
//! to disagree with the branch it is read off.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use verkstead_render::{PinnedEvent, StageEntry};

use crate::checklist;
use crate::repos::git;

/// Where a repository's roadmaps live inside its Worktree, as `/to-roadmap`
/// writes them and `/next-stage` reads them.
pub(crate) const ROADMAPS: &str = "docs/roadmaps";

/// The index of one roadmap, inside its own directory under that.
pub(crate) const INDEX: &str = "ROADMAP.md";

/// The stage list pinned to a Conversation's Timeline: the roadmap its branch
/// has written to, where there is one.
///
/// Empty where a Conversation has no Worktree, where its branch has touched no
/// roadmap, or where what it touched is not a roadmap this can read. All three
/// are the same thing to draw: nothing is pinned, and there is nothing for the
/// human to do about any of them.
///
/// Blocking work, so it happens off the runtime's threads — this is a git read
/// and a file read per Conversation the human opens.
pub(crate) async fn pinned(worktree: Option<PathBuf>, base: Option<String>) -> Vec<PinnedEvent> {
    let (Some(worktree), Some(base)) = (worktree, base) else {
        return Vec::new();
    };

    match tokio::task::spawn_blocking(move || roadmaps(&worktree, &base)).await {
        Ok(pinned) => pinned,
        Err(error) => {
            tracing::error!(error = ?error, "reading a Worktree's roadmaps failed");
            Vec::new()
        }
    }
}

/// The roadmaps this branch has written to, in the order their directories are
/// named.
///
/// Ordinarily one, which is what a Conversation is: a roadmap Conversation
/// writes one and a stage of it ticks one. More than one is nothing to refuse —
/// a branch that touched two roadmaps has two worth showing — and sorted rather
/// than taken as the filesystem hands them over, so a page that drew them twice
/// cannot draw them in two orders.
fn roadmaps(worktree: &Path, base: &str) -> Vec<PinnedEvent> {
    touched(worktree, base)
        .iter()
        .filter_map(|name| roadmap(&worktree.join(ROADMAPS).join(name)))
        .collect()
}

/// Which roadmaps this branch has written to since `base`, by directory name.
///
/// Two questions, because git answers them separately and a roadmap is often
/// both: what has changed against the base commit — committed or not, since the
/// comparison is with the working tree — and what is there that git is not
/// tracking yet. A roadmap the session has written but not committed is in the
/// second until the commit lands, and in the first afterwards.
///
/// A repository that will not answer says none, which is the right way round:
/// what this decides is whether to draw something, and a git that was briefly
/// busy is no reason to draw a roadmap nobody asked for.
pub(crate) fn touched(worktree: &Path, base: &str) -> BTreeSet<String> {
    // `--` rather than `--end-of-options`: what follows is a pathspec, which is
    // git's own name for a path, and the base is a commit Verkstead resolved
    // itself rather than anything a human typed here.
    let changed = git(worktree, &["diff", "--name-only", base, "--", ROADMAPS]);

    let untracked = git(
        worktree,
        &["ls-files", "--others", "--exclude-standard", "--", ROADMAPS],
    );

    [changed, untracked]
        .into_iter()
        .flatten()
        .flat_map(|said| {
            said.lines()
                .filter_map(named)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The roadmap a changed path belongs to — `mvp` of
/// `docs/roadmaps/mvp/ROADMAP.md` — or `None` where it belongs to none.
///
/// A file directly under `docs/roadmaps/` is in no roadmap, and neither is a
/// path outside it that git mentioned for some reason of its own.
fn named(path: &str) -> Option<&str> {
    let inside = path.trim().strip_prefix(ROADMAPS)?.strip_prefix('/')?;

    let (name, rest) = inside.split_once('/')?;

    (!name.is_empty() && !rest.is_empty()).then_some(name)
}

/// The roadmap in `directory`, or `None` where there is none to show.
///
/// A `ROADMAP.md` with no stages in it comes back as `None` rather than as an
/// empty list, exactly as an empty backlog does: what would be pinned is a
/// heading over nothing.
fn roadmap(directory: &Path) -> Option<PinnedEvent> {
    let index = directory.join(INDEX);

    let list = match std::fs::read_to_string(&index) {
        Ok(list) => list,
        // The ordinary case for a directory under `docs/roadmaps/` that is not
        // a roadmap at all. Nothing to say about it.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            // Worth saying: a `ROADMAP.md` that is there and will not be read
            // is a different thing from one that was never written, even though
            // the Timeline draws them the same way.
            tracing::warn!(
                error = ?error,
                index = %index.display(),
                "a Worktree's ROADMAP.md could not be read",
            );
            return None;
        }
    };

    let stages: Vec<StageEntry> = list
        .lines()
        .filter_map(checklist::entry)
        .map(|entry| StageEntry {
            number: entry.label.to_owned(),
            title: entry.title.to_owned(),
            // The box, and nothing else — see the module docs.
            done: entry.checked,
        })
        .collect();

    if stages.is_empty() {
        return None;
    }

    Some(verkstead_render::stage_list_event(
        name(directory),
        checklist::heading(&list),
        stages,
    ))
}

/// What the roadmap is called: its directory's name under `docs/roadmaps/`.
///
/// The directory rather than the heading, because the directory is the roadmap's
/// identity — it is what `/next-stage` is pointed at and what the briefs sit
/// beside. The heading rides along separately, being prose the roadmap wrote
/// about itself.
fn name(directory: &Path) -> String {
    directory
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    use super::*;

    /// A roadmap index exactly as `/to-roadmap` writes one.
    const MVP: &str = "\
# MVP roadmap

Turns this askance clone into Verkstead.

## Stages

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md)
";

    /// A worktree with a base commit behind it, so that what the branch has
    /// written to can be asked of git the way the server asks it.
    ///
    /// `before` is what the base commit carries and `after` is what the branch
    /// has done since — written but not committed, which is the state a session
    /// part-way through its work leaves. Committing it is a test's own step,
    /// because whether that changes the answer is one of the things worth
    /// checking.
    struct Repo {
        dir: tempfile::TempDir,
        base: String,
    }

    impl Repo {
        fn with(before: &[(&str, &str)]) -> Repo {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path();

            run(path, &["init", "--initial-branch", "main"]);
            run(path, &["config", "user.email", "test@verkstead.invalid"]);
            run(path, &["config", "user.name", "Verkstead Test"]);

            // Something to branch from that is not a roadmap, so that a base
            // commit exists whether or not the repository had one.
            std::fs::write(path.join("README.md"), "# a repository\n").unwrap();

            for (name, index) in before {
                write(path, name, index);
            }

            run(path, &["add", "-A"]);
            run(path, &["commit", "-m", "chore: what was here already"]);

            let base = run(path, &["rev-parse", "HEAD"]).trim().to_owned();

            Repo { dir, base }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        /// Write a roadmap into the worktree, uncommitted.
        fn write(&self, name: &str, index: &str) {
            write(self.path(), name, index);
        }

        fn commit(&self) {
            run(self.path(), &["add", "-A"]);
            run(self.path(), &["commit", "-m", "docs: the roadmap"]);
        }

        /// The stage lists this worktree comes back with, which every test here
        /// wants.
        fn lists(&self) -> Vec<verkstead_render::StageListEvent> {
            roadmaps(self.path(), &self.base)
                .into_iter()
                .map(|pinned| match pinned {
                    PinnedEvent::StageList(list) => list,
                    pinned => panic!("a roadmap is a stage list, not {pinned:?}"),
                })
                .collect()
        }
    }

    fn write(worktree: &Path, name: &str, index: &str) {
        let directory = worktree.join(ROADMAPS).join(name);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(INDEX), index).unwrap();
    }

    fn run(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");

        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn the_entries_are_the_roadmaps_own_order_numbers_and_titles() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);

        let lists = repo.lists();

        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "mvp");
        assert_eq!(lists[0].title, "MVP roadmap");
        assert_eq!(
            lists[0]
                .stages
                .iter()
                .map(|stage| (stage.number.as_str(), stage.title.as_str()))
                .collect::<Vec<_>>(),
            [
                ("01", "Workbench"),
                ("02", "Grilling"),
                ("03", "Implementation"),
            ]
        );
    }

    /// The one place a roadmap differs from a backlog in the reading: the box
    /// is the answer, because a stage's brief stays where it is for ever.
    #[test]
    fn a_stage_is_done_when_its_box_is_ticked() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);

        assert_eq!(
            repo.lists()[0]
                .stages
                .iter()
                .map(|stage| stage.done)
                .collect::<Vec<_>>(),
            [true, true, false]
        );
    }

    /// A roadmap is this Conversation's whether the session has committed it
    /// yet or not: what the branch has written to is the question, and the
    /// commit is a step of the writing rather than the whole of it.
    #[test]
    fn a_roadmap_is_the_branchs_written_or_committed() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);

        assert_eq!(repo.lists().len(), 1, "written and not yet committed");

        repo.commit();

        assert_eq!(repo.lists().len(), 1, "and committed");
    }

    /// The whole of Q2: a repository keeps its finished roadmaps, and a
    /// Conversation that never touched one is not about it.
    #[test]
    fn a_roadmap_this_branch_never_touched_is_not_pinned() {
        let repo = Repo::with(&[("public-release", "# Public release\n\n- [x] 01: Done\n")]);

        assert!(repo.lists().is_empty(), "nothing of this branch's");

        repo.write("mvp", MVP);

        assert_eq!(
            repo.lists()
                .iter()
                .map(|list| list.name.clone())
                .collect::<Vec<_>>(),
            ["mvp"],
            "and only the one it wrote",
        );
    }

    /// Including one it did not write but did change: ticking a stage off is
    /// how a roadmap moves, and the Conversation that ticked it is about it.
    #[test]
    fn a_roadmap_this_branch_only_ticked_is_pinned() {
        let repo = Repo::with(&[("mvp", MVP)]);

        assert!(repo.lists().is_empty());

        repo.write("mvp", &MVP.replace("- [ ] 03", "- [x] 03"));

        assert_eq!(repo.lists().len(), 1);
        assert!(repo.lists()[0].stages[2].done);
    }

    #[test]
    fn a_worktree_with_no_roadmaps_pins_no_stage_list() {
        let repo = Repo::with(&[]);

        assert!(repo.lists().is_empty());
    }

    /// A directory of briefs and no index is not a roadmap: the entries are
    /// read from `ROADMAP.md`, and there is nothing to draw without it.
    #[test]
    fn a_directory_without_an_index_is_not_a_roadmap() {
        let repo = Repo::with(&[]);
        let directory = repo.path().join(ROADMAPS).join("mvp");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("01-workbench.md"), "# a stage\n").unwrap();

        assert!(repo.lists().is_empty());
    }

    #[test]
    fn an_index_with_no_stages_in_it_is_nothing_to_pin() {
        let repo = Repo::with(&[]);
        repo.write("mvp", "# MVP roadmap\n\nNothing staged yet.\n");

        assert!(repo.lists().is_empty());
    }

    #[test]
    fn only_numbered_checkboxes_are_stages() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [ ] not a stage at all\n\
             - [ ] 01: A stage\n\
             - a plain bullet\n",
        );

        let lists = repo.lists();

        assert_eq!(lists[0].stages.len(), 1);
        assert_eq!(lists[0].stages[0].title, "A stage");
    }

    #[test]
    fn an_index_with_no_heading_is_still_a_roadmap() {
        let repo = Repo::with(&[]);
        repo.write("mvp", "- [ ] 01: A stage\n");

        let lists = repo.lists();

        assert_eq!(lists[0].name, "mvp");
        assert_eq!(lists[0].title, "");
        assert_eq!(lists[0].stages.len(), 1);
    }

    /// The formats are the repository's rather than Verkstead's, so the proof
    /// is a roadmap nobody wrote for this test: Verkstead's own, written by
    /// `/to-roadmap` on a workstation and kept up to date by hand ever since.
    ///
    /// A roadmap written by either has to be readable by the other, which is
    /// the whole reason the staging fork writes what it writes — see
    /// [`crate::skills`].
    #[test]
    fn the_repositorys_own_roadmaps_read_back() {
        let roadmaps = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(ROADMAPS);

        for name in ["mvp", "public-release"] {
            let list = match roadmap(&roadmaps.join(name)) {
                Some(PinnedEvent::StageList(list)) => list,
                pinned => panic!("{name} should read back as a stage list, not {pinned:?}"),
            };

            assert_eq!(list.name, name);
            assert!(
                list.title.to_lowercase().contains("roadmap"),
                "{name} names itself in its heading: {:?}",
                list.title,
            );
            assert!(
                list.stages.len() > 1,
                "{name} has stages in it: {:?}",
                list.stages,
            );

            for stage in &list.stages {
                assert!(
                    stage.number.chars().all(|c| c.is_ascii_digit()),
                    "a stage is numbered as the roadmap writes it: {stage:?}",
                );
                assert!(
                    !stage.title.is_empty() && !stage.title.contains("[brief]"),
                    "and titled without the link to its brief: {stage:?}",
                );
            }
        }
    }

    #[test]
    fn a_path_names_the_roadmap_its_directory_is() {
        assert_eq!(named("docs/roadmaps/mvp/ROADMAP.md"), Some("mvp"));
        assert_eq!(named("docs/roadmaps/mvp/01-workbench.md"), Some("mvp"));
        assert_eq!(named("docs/roadmaps/README.md"), None);
        assert_eq!(named("docs/roadmaps/"), None);
        assert_eq!(named("docs/design/verkstead.md"), None);
    }
}
