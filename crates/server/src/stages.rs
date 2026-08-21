//! The roadmaps Verkstead reads: a Conversation's Worktree, pinned as
//! stage-list Events, and a registered Repo's, read at a commit for the ones
//! nothing is driving.
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
//!
//! ## The other reading
//!
//! [`abandoned`] and what hangs off it read a **Repo** instead, at a commit,
//! with no Worktree anywhere in it. That is what adoption needs: a roadmap the
//! old tools or a human wrote is committed on the default branch and was
//! touched by no branch Verkstead knows, so the reading above sees nothing of
//! it. The entries, the stage and the branch-slug rule are the same ones; only
//! the way the bytes are fetched differs — `ls-tree` and `show` against the
//! Repo's own git directory rather than files off a checkout.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use verkstead_render::{
    AbandonedRepo, AbandonedRoadmap, AdoptedStage, AdoptionView, PinnedEvent, StageEntry,
};

use crate::checklist;
use crate::repos::git;
use crate::{store, worktrees};

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

/// Where a repository records how work of its own goes for review — the file
/// the finish sequence is read out of, and the stacking mechanism with it.
pub(crate) const GIT_WORKFLOW: &str = "docs/agents/git-workflow.md";

/// The section that file keeps its review process under.
const REVIEW_PROCESS: &str = "## Review process";

/// And the block inside it that says how a stage stacks on the stage before it.
const STACKING: &str = "### Stacking roadmap stages";

/// Whether `worktree`'s repository records a way to stack a roadmap stage on
/// its unmerged predecessor.
///
/// The question is only whether the block is *there*. What it says is the
/// repository's own business and the session's to follow: Verkstead carries no
/// stacking mechanism of its own, and one invented here would be a convention
/// this repository never agreed to. Where the block is missing there is nothing
/// to follow, so the stage branches off the default branch and the Timeline says
/// which of the two happened.
///
/// Read under `## Review process` rather than anywhere in the file, because that
/// is where the section belongs and a file that mentioned stacking in passing —
/// a note, a changelog entry — would not be a repository that had recorded one.
pub(crate) fn stacks(worktree: &Path) -> bool {
    let Ok(workflow) = std::fs::read_to_string(worktree.join(GIT_WORKFLOW)) else {
        return false;
    };

    workflow
        .lines()
        .map(str::trim_end)
        .skip_while(|line| *line != REVIEW_PROCESS)
        .skip(1)
        // As far as the section goes: the next `## ` heading is another section,
        // and what it holds is not the review process.
        .take_while(|line| !line.starts_with("## "))
        .any(|line| line == STACKING)
}

/// What a Conversation's roadmap has left to start once its own work has
/// settled.
///
/// Read off the Worktree, like everything else here, and by the same rule the
/// pinned stage list is drawn by: the roadmap this branch has written to, and
/// the boxes as that roadmap wrote them. So what the human is watching and what
/// Verkstead starts next cannot come to disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Next {
    /// This one: the lowest-numbered stage still unchecked.
    Stage(Box<Stage>),

    /// Every stage is done. The roadmap is finished, and its directory stays
    /// where it is as the record of what it was.
    Complete {
        /// What it is called, for saying so.
        roadmap: String,
    },

    /// This branch has written to no roadmap, so this Conversation is not a
    /// stage of anything and there is nothing to carry on.
    NoRoadmap,

    /// There is a stage to start and it cannot be started: the brief it names
    /// is not there to prime a Conversation with.
    ///
    /// A thing to say rather than to guess past. A roadmap entry pointing at a
    /// file nobody wrote is the human's to fix, and starting the stage after it
    /// instead would be Verkstead deciding to skip work.
    Unstartable {
        /// Why, in the words the Timeline says it in.
        why: String,
    },
}

/// One stage of a roadmap, as the Conversation that runs it is started from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stage {
    /// The roadmap's directory name under `docs/roadmaps/` — `mvp`.
    pub(crate) roadmap: String,

    /// The stage's number as the roadmap writes it — `05`. Zero-padding is the
    /// roadmap's own.
    pub(crate) label: String,

    /// What the stage is called.
    pub(crate) title: String,

    /// Where its brief is, relative to the Worktree — for saying which document
    /// the work came from.
    pub(crate) brief_path: String,

    /// And the brief itself, which is what the stage's Conversation starts from.
    pub(crate) brief: String,
}

impl Stage {
    /// What to call the branch this stage is worked on: its brief's name
    /// without the number in front of it — `04-wrap-up.md` becomes `wrap-up`.
    ///
    /// The brief's name rather than the title, because it is already a slug
    /// somebody chose and it is what the roadmap's own annotation will name.
    /// A brief with nothing usable in its name falls back to the number, which
    /// is the one thing every entry has.
    pub(crate) fn branch(&self) -> String {
        let stem = self
            .brief_path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".md");

        let slug = stem
            .strip_prefix(&self.label)
            .unwrap_or(stem)
            .trim_start_matches(['-', '_']);

        match slug.is_empty() {
            true => format!("stage-{}", self.label),
            false => slug.to_owned(),
        }
    }
}

/// What `worktree`'s roadmap has left to start, with `branch` the Conversation
/// that has just finished.
///
/// The Conversation's own stage is skipped, and that is the one piece of
/// reading that is not just *the lowest unchecked box*. A stage ticks itself off
/// in the plan commit of the stage after it — the roadmap keeps its score one
/// step behind — so when a stage's own work settles, its box is still open and
/// annotated with the branch it was worked on. That annotation is the roadmap
/// saying *this one is in flight*, and the branch in it is what says whose.
///
/// Blocking work: a git read and two file reads.
pub(crate) fn next_stage(worktree: &Path, base: &str, branch: &str) -> Next {
    let mut complete = None;

    for name in touched(worktree, base) {
        let directory = worktree.join(ROADMAPS).join(&name);

        let Ok(list) = std::fs::read_to_string(directory.join(INDEX)) else {
            continue;
        };

        let mut entries = list.lines().filter_map(checklist::entry).peekable();

        if entries.peek().is_none() {
            // A directory under `docs/roadmaps/` with an index that plans
            // nothing is not a roadmap, exactly as it is not one to pin.
            continue;
        }

        let Some(entry) = entries.find(|entry| !entry.checked && !ours(entry.after, branch)) else {
            // Every stage of this one is done — or the only one left is this
            // Conversation's, which the plan commit that ticks it has not landed
            // yet and never will, there being no stage after it. Kept rather
            // than answered with, in case another roadmap this branch touched
            // has something to run.
            complete.get_or_insert(name);
            continue;
        };

        let brief = directory.join(entry.link);

        let Ok(markdown) = std::fs::read_to_string(&brief) else {
            return Next::Unstartable {
                why: format!(
                    "stage {} of the {name} roadmap names the brief {}, and there is nothing \
                     there to read",
                    entry.label,
                    brief.display(),
                ),
            };
        };

        return Next::Stage(Box::new(Stage {
            brief_path: format!("{ROADMAPS}/{name}/{}", entry.link),
            roadmap: name,
            label: entry.label.to_owned(),
            title: entry.title.to_owned(),
            brief: markdown,
        }));
    }

    match complete {
        Some(roadmap) => Next::Complete { roadmap },
        None => Next::NoRoadmap,
    }
}

/// Whether what a roadmap wrote after a stage's link says the stage is in
/// flight on `branch`.
///
/// The branch in backticks, which is how `/next-stage` annotates one: `*(in
/// progress: `wrap-up`)*`. Matched on the branch rather than on the words
/// around it, because the words are prose a human may rewrite and the branch
/// name is the fact.
fn ours(after: &str, branch: &str) -> bool {
    !branch.is_empty() && after.contains(&format!("`{branch}`"))
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

/// A roadmap in a registered Repo that nothing is driving, with the stage
/// adopting it would start.
///
/// **Abandoned** is the workbench's word for it, and the whole of what it means
/// is *there is a stage startable right now and nothing is on it* — see
/// [`startable`] for the four clauses. A roadmap that has finished, one whose
/// next stage somebody is already working, and one whose next brief is missing
/// are all not abandoned, and none of them is a state to draw: what the human
/// can do something about is the only thing worth saying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Abandoned {
    /// What the roadmap calls itself in its heading, or empty where it has
    /// none.
    pub(crate) title: String,

    /// The stage adopting it would start — the lowest-numbered unchecked one,
    /// its brief already read.
    pub(crate) stage: Stage,
}

/// What a roadmap has to start at a commit: the stage, or which of the ways it
/// has none.
///
/// Drawing a notice only wants the stage — see [`Startable::stage`], which is
/// what both readings of the abandoned rule are filtered by. The answers are
/// kept apart for the human who pressed Adopt, because each of them is
/// something different for them to go and do: a roadmap that has finished, a
/// brief nobody wrote and a stage somebody else is on are three jobs and not
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Startable {
    /// There is a stage to start, and nothing is on it.
    Stage(Box<Abandoned>),

    /// No roadmap by that name is readable at this commit, or what is there
    /// plans nothing — which is a directory rather than a roadmap.
    NoRoadmap,

    /// Every box is ticked. The roadmap finished, and its directory stays where
    /// it is as the record of what it was.
    Complete,

    /// The next stage's annotation names a branch that is still there, so
    /// somebody or something is on it.
    InFlight,

    /// The next stage names a brief that cannot be read at this commit — or
    /// names none at all.
    NoBrief,

    /// The stage's own slug branch is taken in the Repo, which is a stage
    /// already under way whatever the boxes say.
    BranchTaken,
}

impl Startable {
    /// The stage where there is one, which is the whole of what a list or a
    /// page needs: what is drawn is what can be adopted, and everything else is
    /// nothing to say.
    pub(crate) fn stage(self) -> Option<Abandoned> {
        match self {
            Startable::Stage(abandoned) => Some(*abandoned),
            _ => None,
        }
    }
}

/// The registered Repos holding abandoned roadmaps, one notice each.
///
/// Nothing is stored. The list is read from the repositories every time it is
/// drawn, the way the pinned stage lists are and for the same reason: the
/// repository has already answered for its own roadmaps, and a list Verkstead
/// kept would be a second opinion about one — wrong from the moment somebody
/// ticked a box.
///
/// Off the runtime's threads, and one blocking task for all of them rather than
/// one apiece. There are as many Repos as the human registered by hand and each
/// is a handful of short git reads against a local directory, so what this costs
/// is one borrowed thread.
pub(crate) async fn abandoned(repos: Vec<store::Repo>) -> Vec<AbandonedRepo> {
    let read =
        tokio::task::spawn_blocking(move || repos.iter().filter_map(notice).collect::<Vec<_>>())
            .await;

    read.unwrap_or_else(|error| {
        tracing::error!(error = ?error, "reading the registered Repos' roadmaps failed");
        Vec::new()
    })
}

/// One Repo's notice, or `None` where it has nothing to adopt.
///
/// Read at the default branch's tip, which is the repository as everyone
/// working on it sees it. A repository with no branch that resolves has nothing
/// to read and nothing to say — the same shrug the pinned lists make.
fn notice(repo: &store::Repo) -> Option<AbandonedRepo> {
    let commit = worktrees::resolve(&repo.path, &repo.default_branch)?;

    let roadmaps: Vec<AbandonedRoadmap> = abandoned_at(&repo.path, &commit)
        .into_iter()
        .map(|abandoned| AbandonedRoadmap {
            name: abandoned.stage.roadmap,
            title: abandoned.title,
            stage: abandoned.stage.label,
            stage_title: abandoned.stage.title,
        })
        .collect();

    (!roadmaps.is_empty()).then(|| AbandonedRepo {
        repo_id: repo.id,
        repo: repo.name.clone(),
        roadmaps,
    })
}

/// The abandoned roadmaps `repo` holds at `commit`, in the order their
/// directories are named.
///
/// The whole of the abandoned rule, applied to every roadmap there: a
/// repository keeps the finished ones and may well be mid-flight on another, so
/// most of what is here on any given day comes back as nothing.
fn abandoned_at(repo: &Path, commit: &str) -> Vec<Abandoned> {
    names(repo, commit)
        .into_iter()
        .filter_map(|name| startable(repo, commit, &name).stage())
        .collect()
}

/// Which roadmaps `repo` holds at `commit`, by directory name.
///
/// The tree rather than the filesystem, which is the whole difference between
/// this reading and the Worktree one above: adoption is about a Repo nothing is
/// checked out of, so what is there is git's to say and not a directory's.
///
/// Sorted, so a list drawn twice cannot be drawn in two orders.
fn names(repo: &Path, commit: &str) -> BTreeSet<String> {
    let listed = git(
        repo,
        &[
            "ls-tree",
            "-r",
            "--name-only",
            "--end-of-options",
            commit,
            // `--` rather than `--end-of-options` again: what follows is a
            // pathspec, which is git's own name for a path.
            "--",
            ROADMAPS,
        ],
    );

    listed
        .iter()
        .flat_map(|said| said.lines())
        .filter_map(indexed)
        .map(str::to_owned)
        .collect()
}

/// The roadmap a listed path is the index of — `mvp` of
/// `docs/roadmaps/mvp/ROADMAP.md` — or `None` where it is not one.
///
/// The index directly inside the roadmap's own directory, rather than any file
/// under it: a `ROADMAP.md` further down is a document somebody filed there, and
/// the roadmap is the directory `/next-stage` is pointed at.
fn indexed(path: &str) -> Option<&str> {
    let inside = path.trim().strip_prefix(ROADMAPS)?.strip_prefix('/')?;

    let (name, rest) = inside.split_once('/')?;

    (!name.is_empty() && rest == INDEX).then_some(name)
}

/// What the roadmap `name` at `commit` has to start, or which of the ways it
/// has nothing.
///
/// The four clauses of the abandoned rule, cheapest first: what the index says,
/// then who the annotation names, then whether the brief is there, then what git
/// has for branches.
///
/// Asked at the default branch's tip for the notice, and at a Conversation's
/// base commit for the page that adopts and for the press itself — the same
/// rule each time, so what the human is offered is what pressing would start.
/// The notice and the page keep only the stage; the press is what says which
/// clause refused it, because the press is the one of the three with a human
/// waiting on an answer.
///
/// Which stage is never in question. It is the lowest-numbered unchecked one:
/// the roadmap's order is the roadmap's own and its stages are strictly
/// sequential. And there is no Conversation of this reading's own to skip, so
/// the branch-skipping [`ours`] does for the settling path has no part in it —
/// a roadmap read here belongs to nobody yet.
///
/// Both branch readings are the fail-safe [`worktrees::branch_taken`] rather
/// than [`worktrees::branch_exists`]: what each of them stands in front of is
/// making a branch and letting an agent loose on it, so git failing to answer
/// is answered as *taken*.
pub(crate) fn startable(repo: &Path, commit: &str, name: &str) -> Startable {
    let Some(index) = at(repo, commit, &format!("{ROADMAPS}/{name}/{INDEX}")) else {
        return Startable::NoRoadmap;
    };

    let mut entries = index.lines().filter_map(checklist::entry).peekable();

    // A directory under `docs/roadmaps/` whose index plans nothing is not a
    // roadmap at all, exactly as it is not one to pin.
    if entries.peek().is_none() {
        return Startable::NoRoadmap;
    }

    // Clause 1: a stage left to do. Every box ticked is a roadmap that
    // finished, and its directory stays where it is as the record of what it
    // was.
    let Some(entry) = entries.find(|entry| !entry.checked) else {
        return Startable::Complete;
    };

    // Clause 3: nobody on it. The annotation is prose a human may have
    // rewritten, so the branch inside the backticks is the fact — and one whose
    // branch is gone is a note about an attempt that was abandoned too.
    if annotating(entry.after).is_some_and(|branch| worktrees::branch_taken(repo, branch)) {
        return Startable::InFlight;
    }

    // Clause 2: something to start it from. An entry pointing at a file nobody
    // wrote is the human's to fix, and nothing is offered until they have —
    // starting the stage after it instead would be Verkstead deciding to skip
    // work.
    if entry.link.is_empty() {
        return Startable::NoBrief;
    }

    let brief_path = format!("{ROADMAPS}/{name}/{}", entry.link);

    let Some(brief) = at(repo, commit, &brief_path) else {
        return Startable::NoBrief;
    };

    let stage = Stage {
        roadmap: name.to_owned(),
        label: entry.label.to_owned(),
        title: entry.title.to_owned(),
        brief_path,
        brief,
    };

    // Clause 4: its branch is free. Which is also what keeps a stage already
    // in flight under Verkstead out of the list — its branch is in this git
    // directory from the moment the stage started, long before the plan commit
    // that ticks its box reaches the default branch.
    if worktrees::branch_taken(repo, &stage.branch()) {
        return Startable::BranchTaken;
    }

    Startable::Stage(Box::new(Abandoned {
        title: checklist::heading(&index),
        stage,
    }))
}

/// What an adopting Conversation's page says about the roadmap it was started
/// for: the roadmap named, and the stage adopting would start.
///
/// Read at the base commit the Conversation branches from — the override where
/// the human typed one, and the default branch's tip where they did not — and
/// read again every time the page is. What the notice said is not carried over:
/// a base pointing somewhere the roadmap reads differently, an unmerged
/// predecessor's tip say, is answered by the stage that is next *there*.
///
/// The same rule the notice was drawn by, so what the page names is what the
/// press would start: a stage that has since been ticked, picked up, or had its
/// branch taken leaves the roadmap named with no stage under it. Which of those
/// it was is the press's to say by name.
///
/// Blocking git reads, so they happen off the runtime's threads.
pub(crate) async fn adopting(
    repo: store::Repo,
    base: Option<String>,
    roadmap: String,
) -> AdoptionView {
    // The roadmap is the one thing here that was never the repository's to say,
    // so it is what the page is drawn with whatever the reading comes back as.
    let named = roadmap.clone();

    let read = tokio::task::spawn_blocking(move || {
        let commit = base.unwrap_or(repo.default_branch);

        let found = worktrees::resolve(&repo.path, &commit)
            .and_then(|commit| startable(&repo.path, &commit, &roadmap).stage());

        let Some(abandoned) = found else {
            return AdoptionView {
                roadmap,
                title: String::new(),
                stage: None,
            };
        };

        AdoptionView {
            roadmap,
            title: abandoned.title,
            stage: Some(AdoptedStage {
                label: abandoned.stage.label.clone(),
                title: abandoned.stage.title.clone(),
                brief_path: abandoned.stage.brief_path.clone(),
                branch: abandoned.stage.branch(),
            }),
        }
    })
    .await;

    read.unwrap_or_else(|error| {
        tracing::error!(error = ?error, "reading the roadmap a Conversation is adopting failed");

        AdoptionView {
            roadmap: named,
            title: String::new(),
            stage: None,
        }
    })
}

/// What `path` holds at `commit`, or `None` where nothing is there to read.
///
/// Asked of git rather than of a directory, because there is no directory: the
/// Repo's own checkout is whatever the human left in it, which is neither the
/// default branch's tip nor any of Verkstead's business.
fn at(repo: &Path, commit: &str, path: &str) -> Option<String> {
    git(
        repo,
        &["show", "--end-of-options", &format!("{commit}:{path}")],
    )
}

/// The branch a roadmap's in-progress annotation names, where it names one.
///
/// What is inside the backticks — `*(in progress: `wrap-up`)*` — which is how
/// `/next-stage` writes one. Read off the backticks rather than off the words
/// around them, for the reason [`ours`] is: the words are prose and the branch
/// name is the fact.
fn annotating(after: &str) -> Option<&str> {
    let (_, rest) = after.split_once('`')?;
    let (branch, _) = rest.split_once('`')?;

    (!branch.is_empty()).then_some(branch)
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

        /// And one of its stage briefs beside it, which is what starting a stage
        /// reads.
        fn brief(&self, name: &str, file: &str, markdown: &str) {
            let directory = self.path().join(ROADMAPS).join(name);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(file), markdown).unwrap();
        }

        /// What the repository records about how its own work goes for review.
        fn workflow(&self, markdown: &str) {
            let file = self.path().join(GIT_WORKFLOW);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, markdown).unwrap();
        }

        /// What this Worktree has left to start, as the Conversation on
        /// `branch` finishing asks it.
        fn next(&self, branch: &str) -> Next {
            next_stage(self.path(), &self.base, branch)
        }

        fn commit(&self) {
            run(self.path(), &["add", "-A"]);
            run(self.path(), &["commit", "-m", "docs: the roadmap"]);
        }

        /// The default branch's tip, which is where the abandoned reading looks
        /// and the only commit it ever reads.
        fn tip(&self) -> String {
            run(self.path(), &["rev-parse", "main"]).trim().to_owned()
        }

        /// The abandoned roadmaps this repository holds there.
        fn abandoned(&self) -> Vec<Abandoned> {
            abandoned_at(self.path(), &self.tip())
        }

        /// And what one roadmap of it comes back as, which is the same reading
        /// with its refusals kept: what a notice throws away, the press says
        /// out loud.
        fn startable(&self, name: &str) -> Startable {
            startable(self.path(), &self.tip(), name)
        }

        /// A branch of its own and nothing on it — which is what a stage in
        /// flight leaves in the Repo's git directory, and what a stale
        /// annotation's branch is not.
        fn branch(&self, name: &str) {
            run(self.path(), &["branch", "--end-of-options", name]);
        }

        /// Commit what is written on a branch of its own, and leave the default
        /// branch where it was.
        ///
        /// A stage's plan commit: the tick and the annotation ride on the
        /// stage's own branch, and reach the default branch only when its pull
        /// request merges.
        fn commit_on(&self, branch: &str, message: &str) {
            run(self.path(), &["checkout", "-q", "-b", branch]);
            run(self.path(), &["add", "-A"]);
            run(self.path(), &["commit", "-m", message]);
            run(self.path(), &["checkout", "-q", "main"]);
        }

        /// This repository as a registered Repo, which is what the adoption
        /// reading is handed: there is no Worktree anywhere in that one.
        fn registered(&self) -> store::Repo {
            store::Repo {
                id: 1,
                path: self.path().to_owned(),
                name: "verkstead".to_owned(),
                default_branch: "main".to_owned(),
            }
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

    /// The ordinary continuation: the stage after the one whose work has just
    /// settled, read off the boxes the roadmap keeps.
    #[test]
    fn the_next_stage_is_the_lowest_numbered_unchecked_one() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let Next::Stage(stage) = repo.next("anything-else") else {
            panic!("stage 03 is the one left: {:?}", repo.next("anything-else"));
        };

        assert_eq!(stage.roadmap, "mvp");
        assert_eq!(stage.label, "03");
        assert_eq!(stage.title, "Implementation");
        assert_eq!(stage.brief_path, "docs/roadmaps/mvp/03-implementation.md");
        assert_eq!(stage.brief, "# 03. Implementation\n");
        assert_eq!(
            stage.branch(),
            "implementation",
            "the branch is the brief's own name, without the number in front of it",
        );
    }

    /// The one piece of reading that is not just *the lowest unchecked box*. A
    /// stage is ticked off by the plan commit of the stage after it, so when its
    /// own work settles its box is still open — and what says so is the roadmap's
    /// annotation naming the branch it was worked on.
    ///
    /// A Conversation that started stage 02 again here would be a run going round
    /// in circles for ever, with nobody watching.
    #[test]
    fn the_stage_this_conversation_worked_is_not_the_one_to_start() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `grilling`)*\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n",
        );
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let Next::Stage(stage) = repo.next("grilling") else {
            panic!("the stage after this Conversation's own is the one to start");
        };

        assert_eq!(stage.label, "03");

        // And to anybody else it is the stage it says it is: the annotation is
        // about whose it is, not about whether it is done.
        let Next::Stage(stage) = repo.next("some-other-branch") else {
            panic!("stage 02 is still unchecked");
        };

        assert_eq!(stage.label, "02");
    }

    /// The end of a roadmap, which is where the whole pipeline stops of its own
    /// accord: nothing is started, and there is nothing for the human to do.
    #[test]
    fn a_roadmap_with_every_stage_checked_has_nothing_to_start() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n- [x] 01: Workbench — [brief](01-workbench.md)\n",
        );

        assert_eq!(
            repo.next("anything-else"),
            Next::Complete {
                roadmap: "mvp".to_owned()
            },
        );
    }

    /// Including the last stage of one, whose own box is ticked by a plan commit
    /// that is never going to land: there is no stage after it to write one.
    #[test]
    fn the_last_stage_finishing_is_the_roadmap_complete() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `grilling`)*\n",
        );

        assert_eq!(
            repo.next("grilling"),
            Next::Complete {
                roadmap: "mvp".to_owned()
            },
        );
    }

    /// An ordinary feature: its branch has written to no roadmap, so there is no
    /// roadmap for its wrap-up to carry on.
    #[test]
    fn a_branch_that_wrote_to_no_roadmap_carries_nothing_on() {
        let repo = Repo::with(&[("mvp", MVP)]);

        assert_eq!(repo.next("rate-limiting"), Next::NoRoadmap);
    }

    /// A stage that cannot be started is said rather than skipped: starting the
    /// one after it would be Verkstead deciding to leave work out.
    #[test]
    fn a_stage_whose_brief_is_missing_is_not_startable() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);

        let Next::Unstartable { why } = repo.next("anything-else") else {
            panic!("there is no 03-implementation.md to start stage 03 from");
        };

        assert!(
            why.contains("03") && why.contains("03-implementation.md"),
            "which stage and which brief: {why:?}",
        );
    }

    /// Whether the repository records a way to stack a stage on its predecessor
    /// — which is a fact about the repository, and the only part of stacking
    /// Verkstead decides.
    #[test]
    fn a_repository_records_its_stacking_mechanism_under_its_review_process() {
        let repo = Repo::with(&[]);

        assert!(
            !stacks(repo.path()),
            "a repository with no git-workflow.md has recorded nothing",
        );

        repo.workflow("# Git workflow\n\n## Review process\n\n### Finish sequence\n\nPush it.\n");

        assert!(
            !stacks(repo.path()),
            "and neither has one whose review process says nothing about stacking",
        );

        repo.workflow(
            "# Git workflow\n\n## Review process\n\n\
             ### Finish sequence\n\nPush it.\n\n\
             ### Stacking roadmap stages\n\n`gh stack init <predecessor> <new>`\n",
        );

        assert!(stacks(repo.path()), "and this one has");
    }

    /// Under the review process rather than anywhere in the file: a repository
    /// that mentions stacking in a note has not recorded a mechanism to follow.
    #[test]
    fn a_stacking_block_outside_the_review_process_is_not_the_mechanism() {
        let repo = Repo::with(&[]);
        repo.workflow(
            "# Git workflow\n\n## Review process\n\n### Finish sequence\n\nPush it.\n\n\
             ## Notes\n\n### Stacking roadmap stages\n\nWe gave up on these.\n",
        );

        assert!(!stacks(repo.path()));
    }

    /// A roadmap that already exists, with something left to do and nobody on
    /// it: the whole of what adoption is for, and the shape of the notice.
    #[test]
    fn a_roadmap_with_a_startable_next_stage_is_abandoned() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        let abandoned = repo.abandoned();

        assert_eq!(abandoned.len(), 1);
        assert_eq!(abandoned[0].title, "MVP roadmap");
        assert_eq!(abandoned[0].stage.roadmap, "mvp");
        assert_eq!(
            abandoned[0].stage.label, "03",
            "the lowest-numbered unchecked stage, which is the roadmap's own order",
        );
        assert_eq!(abandoned[0].stage.title, "Implementation");
        assert_eq!(
            abandoned[0].stage.brief_path,
            "docs/roadmaps/mvp/03-implementation.md",
        );
        assert_eq!(abandoned[0].stage.brief, "# 03. Implementation\n");
        assert_eq!(
            abandoned[0].stage.branch(),
            "implementation",
            "adopting it takes the stage's own slug, as the unattended start does",
        );
    }

    /// Read at the commit and not off the checkout: a roadmap somebody is
    /// part-way through writing is not one there is anything to adopt.
    #[test]
    fn a_roadmap_that_is_only_written_is_not_abandoned() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        assert!(repo.abandoned().is_empty(), "nothing is committed");

        repo.commit();

        assert_eq!(repo.abandoned().len(), 1, "and now it is");
    }

    /// Clause 1. A roadmap that finished is not abandoned — it is done, and its
    /// directory stays where it is as the record of what it was.
    #[test]
    fn a_roadmap_with_every_box_ticked_is_not_abandoned() {
        let repo = Repo::with(&[(
            "mvp",
            "# MVP roadmap\n\n- [x] 01: Workbench — [brief](01-workbench.md)\n",
        )]);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.commit();

        assert!(repo.abandoned().is_empty());
    }

    /// And neither is a directory whose index plans nothing, which is not a
    /// roadmap at all.
    #[test]
    fn an_index_with_no_stages_in_it_is_not_a_roadmap_to_adopt() {
        let repo = Repo::with(&[("mvp", "# MVP roadmap\n\nNothing staged yet.\n")]);

        assert!(repo.abandoned().is_empty());
    }

    /// Clause 2. An entry pointing at a file nobody wrote is the human's to
    /// fix. Offering it would be offering to start a Conversation with no Brief.
    #[test]
    fn a_roadmap_whose_next_brief_is_missing_is_not_abandoned() {
        let repo = Repo::with(&[("mvp", MVP)]);

        assert!(
            repo.abandoned().is_empty(),
            "there is no 03-implementation.md at that commit",
        );

        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        assert_eq!(repo.abandoned().len(), 1);
    }

    /// Clause 3. The annotation is prose a human may have rewritten, so the
    /// branch in the backticks is the fact — and the fact is whether it is
    /// there.
    #[test]
    fn an_annotation_naming_a_branch_that_still_exists_stops_adoption() {
        let repo = Repo::with(&[(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `someone-elses`)*\n",
        )]);
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();
        repo.branch("someone-elses");

        assert!(
            repo.abandoned().is_empty(),
            "somebody is on `someone-elses`, so stage 02 is not there to take",
        );
    }

    /// And a note left over from an attempt that was itself abandoned does not
    /// stop it: the branch is the fact, and it is not there.
    #[test]
    fn an_annotation_whose_branch_is_gone_does_not_stop_adoption() {
        let repo = Repo::with(&[(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `long-gone`)*\n",
        )]);
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        let abandoned = repo.abandoned();

        assert_eq!(abandoned.len(), 1);
        assert_eq!(abandoned[0].stage.label, "02");
    }

    /// Clause 4. A branch by the stage's own name is a stage somebody — or some
    /// earlier run — has started already, whatever the roadmap's boxes say. The
    /// same rule the unattended start refuses by, applied before anything is
    /// offered.
    #[test]
    fn a_stage_whose_slug_branch_is_taken_is_not_abandoned() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        assert_eq!(repo.abandoned().len(), 1, "nothing is on it yet");

        repo.branch("implementation");

        assert!(
            repo.abandoned().is_empty(),
            "`implementation` is taken, so stage 03 is under way somewhere",
        );
    }

    /// The same four clauses, each answered by its own name — which is what the
    /// Adopt press hands the human, one job apiece rather than one shrug.
    #[test]
    fn a_roadmap_with_nothing_to_start_says_which_clause_refused_it() {
        let repo = Repo::with(&[
            ("mvp", MVP),
            (
                "finished",
                "# Finished roadmap\n\n- [x] 01: Done — [brief](01-done.md)\n",
            ),
            ("empty", "# Empty roadmap\n\nNothing staged yet.\n"),
            (
                "in-flight",
                "# In-flight roadmap\n\n\
                 - [ ] 01: Packaging — [brief](01-packaging.md) *(in progress: `packaging`)*\n",
            ),
            ("unlinked", "# Unlinked roadmap\n\n- [ ] 01: Nowhere\n"),
        ]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.brief("finished", "01-done.md", "# 01. Done\n");
        repo.brief("in-flight", "01-packaging.md", "# 01. Packaging\n");
        repo.commit();
        repo.branch("packaging");

        let startable = repo.startable("mvp");

        assert_eq!(
            startable
                .clone()
                .stage()
                .expect("stage 03 is there")
                .stage
                .label,
            "03",
        );

        assert_eq!(
            repo.startable("public-release"),
            Startable::NoRoadmap,
            "no roadmap by that name is at this commit",
        );
        assert_eq!(
            repo.startable("empty"),
            Startable::NoRoadmap,
            "and an index that plans nothing is a directory rather than a roadmap",
        );
        assert_eq!(repo.startable("finished"), Startable::Complete);
        assert_eq!(
            repo.startable("in-flight"),
            Startable::InFlight,
            "`packaging` is there, so somebody is on stage 01",
        );
        assert_eq!(
            repo.startable("unlinked"),
            Startable::NoBrief,
            "an entry that links to nothing has nothing to start from",
        );

        // Clause 2 the other way round: an entry that links to a brief nobody
        // wrote, which is the roadmap's own to fix.
        std::fs::remove_file(repo.path().join(ROADMAPS).join("in-flight/01-packaging.md")).unwrap();
        run(repo.path(), &["branch", "-D", "packaging"]);
        repo.commit();

        assert_eq!(repo.startable("in-flight"), Startable::NoBrief);

        // And clause 4, which is the one the roadmap says nothing about at all.
        repo.branch("implementation");

        assert_eq!(repo.startable("mvp"), Startable::BranchTaken);
    }

    /// Which is what keeps a stage currently mid-flight under Verkstead out of
    /// the list. Its plan commit ticks the box and annotates the entry, and both
    /// ride on the stage's own branch until its pull request merges — so the
    /// default-tip read sees a roadmap that still has stage 03 open, and the
    /// branch is the only thing saying otherwise.
    #[test]
    fn a_plan_commit_that_has_not_reached_the_default_branch_is_invisible() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        // The stage starts: a branch at its own slug, and a plan commit on it
        // ticking the box and saying whose it is.
        repo.write(
            "mvp",
            &MVP.replace(
                "- [ ] 03: Implementation — [brief](03-implementation.md)",
                "- [x] 03: Implementation — [brief](03-implementation.md) \
                 *(in progress: `implementation`)*",
            ),
        );
        repo.commit_on("implementation", "chore: plan the implementation stage");

        assert_eq!(
            at(repo.path(), &repo.tip(), "docs/roadmaps/mvp/ROADMAP.md",),
            Some(MVP.to_owned()),
            "the default branch has none of the plan commit",
        );
        assert!(
            repo.abandoned().is_empty(),
            "and the branch is what says the stage is already under way",
        );

        // The branch goes and nothing is running it, which is exactly the state
        // adoption is for.
        run(repo.path(), &["branch", "-D", "implementation"]);

        assert_eq!(repo.abandoned().len(), 1);
    }

    /// One notice per Repo, with its roadmaps inside — and none at all for a
    /// Repo with nothing to adopt.
    #[test]
    fn a_repos_notice_carries_its_roadmaps_and_nothing_else() {
        let repo = Repo::with(&[
            ("mvp", MVP),
            (
                "public-release",
                "# Public release roadmap\n\n- [ ] 01: Packaging — [brief](01-packaging.md)\n",
            ),
            (
                "finished",
                "# Finished roadmap\n\n- [x] 01: Done — [brief](01-done.md)\n",
            ),
        ]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.brief("public-release", "01-packaging.md", "# 01. Packaging\n");
        repo.brief("finished", "01-done.md", "# 01. Done\n");
        repo.commit();

        let notice = notice(&store::Repo {
            id: 7,
            path: repo.path().to_owned(),
            name: "verkstead".to_owned(),
            default_branch: "main".to_owned(),
        })
        .expect("two of its roadmaps have a stage to start");

        assert_eq!(notice.repo_id, 7);
        assert_eq!(notice.repo, "verkstead");
        assert_eq!(
            notice
                .roadmaps
                .iter()
                .map(|roadmap| (
                    roadmap.name.as_str(),
                    roadmap.title.as_str(),
                    roadmap.stage.as_str(),
                    roadmap.stage_title.as_str(),
                ))
                .collect::<Vec<_>>(),
            [
                ("mvp", "MVP roadmap", "03", "Implementation"),
                (
                    "public-release",
                    "Public release roadmap",
                    "01",
                    "Packaging",
                ),
            ],
            "the finished one is not among them",
        );
    }

    #[test]
    fn a_repo_with_nothing_to_adopt_has_no_notice() {
        let repo = Repo::with(&[(
            "mvp",
            "# MVP roadmap\n\n- [x] 01: Workbench — [brief](01-workbench.md)\n",
        )]);

        assert_eq!(
            notice(&store::Repo {
                id: 1,
                path: repo.path().to_owned(),
                name: "verkstead".to_owned(),
                default_branch: "main".to_owned(),
            }),
            None,
        );
    }

    /// A Repo whose default branch resolves to nothing has nothing to read, and
    /// says nothing rather than failing the list.
    #[test]
    fn a_repo_with_no_such_default_branch_says_nothing() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        assert_eq!(
            notice(&store::Repo {
                id: 1,
                path: repo.path().to_owned(),
                name: "verkstead".to_owned(),
                default_branch: "trunk".to_owned(),
            }),
            None,
        );
    }

    #[test]
    fn a_listed_path_names_the_roadmap_it_indexes() {
        assert_eq!(indexed("docs/roadmaps/mvp/ROADMAP.md"), Some("mvp"));
        assert_eq!(indexed("docs/roadmaps/mvp/01-workbench.md"), None);
        assert_eq!(indexed("docs/roadmaps/mvp/old/ROADMAP.md"), None);
        assert_eq!(indexed("docs/roadmaps/ROADMAP.md"), None);
        assert_eq!(indexed("docs/design/verkstead.md"), None);
    }

    #[test]
    fn an_annotation_names_whatever_is_in_its_backticks() {
        assert_eq!(annotating("*(in progress: `wrap-up`)*"), Some("wrap-up"));
        assert_eq!(
            annotating("*(started on `feature/x` last week)*"),
            Some("feature/x")
        );
        assert_eq!(annotating(""), None);
        assert_eq!(annotating("*(in progress)*"), None);
        assert_eq!(annotating("*(in progress: `unclosed)*"), None);
        assert_eq!(annotating("*(in progress: ``)*"), None);
    }

    #[test]
    fn a_path_names_the_roadmap_its_directory_is() {
        assert_eq!(named("docs/roadmaps/mvp/ROADMAP.md"), Some("mvp"));
        assert_eq!(named("docs/roadmaps/mvp/01-workbench.md"), Some("mvp"));
        assert_eq!(named("docs/roadmaps/README.md"), None);
        assert_eq!(named("docs/roadmaps/"), None);
        assert_eq!(named("docs/design/verkstead.md"), None);
    }

    /// What an adopting Conversation's page is drawn from, with no base
    /// override: the roadmap named, and the stage that is next at the default
    /// branch's tip.
    #[tokio::test]
    async fn an_adoption_page_names_the_stage_that_is_next_at_the_default_tip() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        let view = adopting(repo.registered(), None, "mvp".to_owned()).await;

        assert_eq!(view.roadmap, "mvp");
        assert_eq!(view.title, "MVP roadmap");

        let stage = view.stage.expect("that stage is startable");
        assert_eq!(stage.label, "03");
        assert_eq!(stage.title, "Implementation");
        assert_eq!(stage.brief_path, "docs/roadmaps/mvp/03-implementation.md");
        assert_eq!(
            stage.branch, "implementation",
            "the stage's own slug, as the unattended start names one",
        );
    }

    /// The base override is where it reads, so a commit the roadmap says
    /// something else at is answered by the stage that is next *there* rather
    /// than by the one the default branch is up to.
    #[tokio::test]
    async fn an_adoption_page_reads_the_stage_at_whatever_the_base_says() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.brief("mvp", "04-wrap-up.md", "# 04. Wrap-up\n");
        repo.commit();

        // Where the roadmap stood before the implementation stage ticked itself
        // off — which is what an unmerged predecessor's tip is a case of.
        let before = repo.tip();

        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             Turns this askance clone into Verkstead.\n\n\
             ## Stages\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [x] 02: Grilling — [brief](02-grilling.md)\n\
             - [x] 03: Implementation — [brief](03-implementation.md)\n\
             - [ ] 04: Wrap-up — [brief](04-wrap-up.md)\n",
        );
        repo.commit();

        let at_tip = adopting(repo.registered(), None, "mvp".to_owned()).await;
        assert_eq!(
            at_tip.stage.expect("that stage is startable").label,
            "04",
            "with no override, the default branch's tip is what is read",
        );

        let earlier = adopting(repo.registered(), Some(before), "mvp".to_owned()).await;
        assert_eq!(
            earlier.stage.expect("that stage is startable").label,
            "03",
            "read at the base the human named, where 03 is still open",
        );
    }

    /// Everything that can be wrong with a roadmap at a commit comes back the
    /// same way: the roadmap is still what is being adopted, and there is no
    /// stage under it. Which of them it was is the press's to say by name.
    #[tokio::test]
    async fn an_adoption_page_names_no_stage_where_there_is_none_to_start() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        for (base, why) in [
            (
                Some("no-such-thing".to_owned()),
                "the base resolves to nothing",
            ),
            (None, "the roadmap is not there under that name"),
        ] {
            let roadmap = match base {
                Some(_) => "mvp",
                None => "public-release",
            };

            let view = adopting(repo.registered(), base, roadmap.to_owned()).await;

            assert_eq!(view.roadmap, roadmap, "{why}");
            assert_eq!(view.title, "", "{why}");
            assert_eq!(view.stage, None, "{why}");
        }
    }

    /// The same four clauses the notice was drawn by, so what the page names is
    /// what the press would start: a stage whose branch is taken is a stage
    /// somebody is already on.
    #[tokio::test]
    async fn an_adoption_page_names_no_stage_whose_branch_is_taken() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();
        repo.branch("implementation");

        let view = adopting(repo.registered(), None, "mvp".to_owned()).await;

        assert_eq!(view.roadmap, "mvp");
        assert_eq!(view.stage, None);
    }
}
