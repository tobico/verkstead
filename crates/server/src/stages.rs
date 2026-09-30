//! The roadmaps Verkstead reads: a Conversation's Worktree, drawn as stage-list
//! Events, and a registered Repo's, read at a commit — for the roadmaps nothing
//! is driving, and for what a roadmap being driven has left to start.
//!
//! What a roadmap *says* is not stored, for the reason nothing about a backlog
//! is — see [`crate::tasks`], which is also where the one row that *is* stored
//! is explained. `docs/roadmaps/` is the repository's: written by the roadmap
//! direction's session, and rewritten by every stage that ticks itself off as it
//! finishes. So the Event is a reading of the Worktree as it stands and cannot
//! disagree with the branch it is read off.
//!
//! **Where** each stage of it is — *done*, *in progress*, *halted*, *waiting on*
//! a named stage or *to do* — is [`states`], which is what the card's rows and the
//! pane's headings say. The same rule as [`done`] below and one word further: what
//! a scheduler asks is whether a stage is done, so that it knows what may start,
//! and what a human reading the roadmap asks is where each of its stages has got
//! to. So the states that are not done are told apart there and are all one *not
//! done* here.
//!
//! What says a stage is done is **Verkstead's own record where it has a row for
//! that stage, and the checkbox in `ROADMAP.md` where it has none** — see
//! [`done`], which is the whole of that rule, and
//! [ADR-0021](../../../../docs/adr/0021-parallel-stages.md). A stage settled is a
//! stage done however its box reads on the branch being read, because with
//! branches side by side each Worktree holds a `ROADMAP.md` of its own and the
//! boxes stop being one fact; a stage with no row is one worked by hand or by the
//! old tools, and its box is all there is to go on. Nothing looks for a file
//! going away, either way: a stage's brief stays where it is for ever, being the
//! record of what the stage was for. See [`crate::tasks`], which is the same
//! answer a backlog gives and for the same reason — a backlog has no record
//! beside it.
//!
//! *Which* roadmap is the one thing this does differently, and the one thing
//! here that is **stored**. A Worktree has one `.tasks/` and may hold any number
//! of roadmaps — a repository keeps the finished ones, which is what they are
//! for — so which of them a Conversation is a stage of has to come from
//! somewhere. It comes from the record: written once, when the stage starts or
//! when the branch that wrote a roadmap is seen to have written it, and read
//! back by name from then on. See `store::stage_roadmap`, and ADR-0017 for why
//! this one fact is stored when nothing else about a roadmap is: it is
//! Verkstead's own decision rather than the repository's, taken once, on the
//! same grounds `stage_branches` already gives for which branch a stage stacks
//! on. What the roadmap *says* — its boxes, its briefs — is still read off the
//! Worktree and cannot come to disagree with the branch.
//!
//! It used to be asked of git at every wrap-up, and that is the bug this
//! replaces: a branch that had touched two roadmaps was walked in name order,
//! so a stage whose own roadmap ran out carried on into somebody else's effort.
//! Adopting another roadmap is the human's act, from *Continue a roadmap*, and
//! nothing here does it for them.
//!
//! Those same files are what the details pane is built from, one level deeper:
//! the index says what the roadmap is made of, and each `NN-<slug>.md` beside it
//! is the brief that stage is worked from — see [`documents`], which reads them
//! whole rather than counting their boxes.
//!
//! ## The other reading
//!
//! [`next_stage`], [`abandoned`] and what hangs off them read a **Repo** instead,
//! at a commit, with no Worktree anywhere in it. That is what adoption needs: a
//! roadmap the old tools or a human wrote is committed on the default branch and
//! was touched by no branch Verkstead knows, so the reading above sees nothing of
//! it. And it is what the **carry-on** needs, for the other half of the same
//! reason: with stages worked side by side the Worktree of the one that has just
//! settled holds a `ROADMAP.md` cut before the newest declarations were written, so
//! what it has left is read at the top of its roadmap's chain — see [`Declaring`].
//! The entries, the stage and the branch-naming rule are the same ones; only the
//! way the bytes are fetched differs — `ls-tree` and `show` against the Repo's own
//! git directory rather than files off a checkout.
//!
//! And the record is the same record: what is done there is [`done`] too, one
//! Repo's rows read once and handed to a reading that never asks the database.
//! Which is what stops the two disagreeing — a stage settled on a branch nobody
//! has merged is unticked at the default branch's tip, and what the boxes alone
//! say there is *start stage 01 again*.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_render::{
    AbandonedRepo, AbandonedRoadmap, AdoptedStage, AdoptionView, RoadmapPane, StageEntry,
    StageListEvent, StageSource, StageState,
};

use crate::checklist;
use crate::declarations;
use crate::repos::git;
use crate::{store, worktrees};

/// Where a repository's roadmaps live inside its Worktree, as `/to-roadmap`
/// writes them and `/next-stage` reads them.
pub(crate) const ROADMAPS: &str = "docs/roadmaps";

/// The index of one roadmap, inside its own directory under that.
pub(crate) const INDEX: &str = "ROADMAP.md";

/// What every stage's branch is named under — see [`Stage::branch`], which is
/// the only thing that spells it.
///
/// The roadmaps' own directory name, one level up from where they live, so that
/// a stage branch reads as the roadmap's work at a glance.
pub(crate) const STAGES: &str = "roadmaps";

/// How many stages of **one roadmap** Verkstead runs at once where nobody has
/// said otherwise.
///
/// Three, which is the number
/// [ADR-0021](../../../../docs/adr/0021-parallel-stages.md) settled: enough that
/// a roadmap shaped like a fan gets on with its fan, few enough that the human is
/// never handed more breakdown Sets at once than they can read.
///
/// **A place is taken by every stage of the roadmap the record has in flight**,
/// whatever that stage is doing — see [`Ready::Stages`], which is where they are
/// counted. A stage blocked on a Question Set and a stage waiting to join the
/// chain are both holding one, which is the human's choice rather than an
/// oversight: what the limit is for is how much of one roadmap is open at once,
/// and work somebody has been asked a question about is open.
///
/// The default rather than the rule: what it is on this machine is
/// `at_once.roadmap_stages` in `config.yaml`, read afresh at every start — see
/// [`crate::settings::AtOnce`], which falls back to this, and the carry-on, which
/// reads the file and passes the number into [`next_stage`]. Passed in rather
/// than read here so that where a start is permitted stays the one place, and how
/// many are permitted is the caller's to say.
pub(crate) const AT_ONCE: usize = 3;

/// How many Conversations Verkstead runs at once **across the whole server**,
/// whatever roadmap or Process they belong to, where nobody has said otherwise.
///
/// Four, which is the other number
/// [ADR-0021](../../../../docs/adr/0021-parallel-stages.md) settled, and it sits
/// beside [`AT_ONCE`]'s three so that one roadmap cannot take the whole server by
/// default. The two limit different things and both are in force: the roadmap's
/// is about how much of one effort is open at once, and this one is there for the
/// machine — a stage may be a heavy build and a test run, and the hardware is
/// shared by everything the server runs.
///
/// **A place is taken by a Conversation with a session running or a driver
/// registered**, of any kind: a grilling, a Review, a Tinker, another roadmap's
/// stage. Both registers rather than the sessions one alone, because a stage
/// waiting to join the chain has no session at all and a Conversation between two
/// task sessions has none for a moment either — see
/// [`crate::drivers::Drivers::taking`], which is where the two are counted
/// together. A Conversation that is Done, Draft or stopped holds none, a stop
/// being raised as the driver lets go.
///
/// Which leaves the two limits deliberately asymmetric, and it is worth knowing
/// which way: a **halted** stage keeps its place under its own roadmap's limit,
/// the record having it in flight, and takes none here, nothing being run or
/// driven. A server of halted stages goes on starting other roadmaps' stages
/// while each halted stage's own roadmap stands still.
///
/// **It holds back only what Verkstead starts by itself** — a stage started by a
/// settle. A press goes ahead over the limit and is counted from then on, which
/// is why nothing on the adoption's side of this module has heard of the number:
/// see [`startable`], which is what a press is offered and reads [`AT_ONCE`]
/// alone.
///
/// The default rather than the rule, the way [`AT_ONCE`] is: what it is on this
/// machine is `at_once.conversations` in `config.yaml`, read afresh at every
/// start and at every look — see [`crate::settings::AtOnce::conversations`],
/// which falls back to this.
///
/// Passed in rather than read here, for the reason [`AT_ONCE`] is: where a start
/// is permitted stays the one place, and how many are permitted is the caller's
/// to say. What counts the places is the carry-on — see
/// [`crate::continuing::carry_on`] — and the look that spends the ones that come
/// free, [`crate::places`].
pub(crate) const CONVERSATIONS_AT_ONCE: usize = 4;

/// The stage lists a Conversation's Timeline draws: the roadmaps its branch has
/// written to, where there are any.
///
/// Empty where a Conversation has no Worktree, where its branch has touched no
/// roadmap, or where what it touched is not a roadmap this can read. All three
/// are the same thing to draw: no card, in either of the two places one goes.
///
/// One reading behind both of them, for the reason [`crate::tasks::showing`] is
/// one reading: the pinned block and the row on the record where the roadmap
/// landed draw the same cards.
///
/// `record` is what Verkstead's own record says about this Repo's stages, which is
/// where each stage's state comes from — see [`states`]. Read by the handler above
/// this, which holds the pool, and handed in as a value, so that this stays a
/// reading: the card and the pane are one Conversation's two views of one
/// roadmap, and a card that asked the database for itself could disagree with the
/// pane beside it.
///
/// `held` is the other half of that, and the register rather than the record:
/// which Conversations are stages this server is holding before their finish — see
/// [`crate::joins`]. Read by the same handler at the same moment, for the reason
/// the sidebar's own label is read there: the hold is a task of this process, and
/// the label and the card's word are the one register read the one way.
///
/// Blocking work, so it happens off the runtime's threads — this is a git read
/// and a file read per Conversation the human opens.
pub(crate) async fn showing(
    worktree: Option<PathBuf>,
    base: Option<String>,
    record: store::StageStandings,
    held: HashSet<i64>,
) -> Vec<StageListEvent> {
    let (Some(worktree), Some(base)) = (worktree, base) else {
        return Vec::new();
    };

    match tokio::task::spawn_blocking(move || roadmaps(&worktree, &base, &record, &held)).await {
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
fn roadmaps(
    worktree: &Path,
    base: &str,
    record: &store::StageStandings,
    held: &HashSet<i64>,
) -> Vec<StageListEvent> {
    touched(worktree, base)
        .iter()
        .filter_map(|name| roadmap(&worktree.join(ROADMAPS).join(name), record, held))
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

/// Which roadmaps this branch *created* since `base`, by directory name.
///
/// [`touched`] asked the narrower way, and the difference is the whole of what
/// it is for: writing a roadmap is choosing it, and amending one somebody else
/// planned is not. A stage session that retires a deferral in another effort's
/// brief has touched that roadmap and created none.
///
/// Created means new against the base commit, judged by the index: the
/// `ROADMAP.md` is untracked, or git says it was added. A roadmap whose
/// directory was there already is not created however much of it this branch
/// rewrote — the index is what says a roadmap exists, so the index appearing is
/// what says one was written here.
///
/// A repository that will not answer says none, which is the right way round
/// for [`touched`]'s reason and one more: what this decides is whether a
/// Conversation gets a roadmap recorded against it, and a git that was briefly
/// busy is no reason to record the wrong one.
pub(crate) fn created(worktree: &Path, base: &str) -> BTreeSet<String> {
    let added = git(
        worktree,
        &[
            "diff",
            "--name-only",
            "--diff-filter=A",
            base,
            "--",
            ROADMAPS,
        ],
    );

    let untracked = git(
        worktree,
        &["ls-files", "--others", "--exclude-standard", "--", ROADMAPS],
    );

    [added, untracked]
        .into_iter()
        .flatten()
        .flat_map(|said| {
            said.lines()
                .filter_map(indexed)
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
/// this repository never agreed to.
///
/// **What this does not decide is where the branch starts.** That is git's
/// question rather than the file's — see [`crate::continuing::Stands`] — and a
/// stage whose predecessor is unmerged stands on it whether or not there is a
/// block to read. Where the block is missing what follows is a pull request
/// carrying the stage before it until that one merges, and the Timeline says so.
///
/// Read under `## Review process` rather than anywhere in the file, because that
/// is where the section belongs and a file that mentioned stacking in passing —
/// a note, a changelog entry — would not be a repository that had recorded one.
pub(crate) fn stacks(worktree: &Path) -> bool {
    std::fs::read_to_string(worktree.join(GIT_WORKFLOW)).is_ok_and(|workflow| stacking(&workflow))
}

/// The same question asked of a Repo at a commit, with no Worktree anywhere in
/// it — [`abandoned`]'s way of reading, for [`abandoned`]'s reason.
///
/// What adoption needs. A stage adopted off an unmerged predecessor stacks on
/// it exactly as an unattended one does, and the file that says whether this
/// repository stacks at all is a file in the repository — read here at the
/// commit the stage branches from, which is the commit whose contents the stage
/// will be working against.
pub(crate) fn stacks_at(repo: &Path, commit: &str) -> bool {
    at(repo, commit, GIT_WORKFLOW).is_some_and(|workflow| stacking(&workflow))
}

/// Whether a `git-workflow.md`'s text records a stacking mechanism.
///
/// The bytes are the caller's to fetch — off a checkout or out of a git
/// directory — and what counts as recording one is the same either way.
fn stacking(workflow: &str) -> bool {
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
/// *Which* roadmap is the record's — named by the caller and settled when the
/// stage started. What it has ready comes from its declarations, the record and
/// the boxes together: the entries and the briefs are read at a commit, off the
/// branch holding the newest of that roadmap's declarations — see [`Declaring`] —
/// and which of those entries may start now is [`ready`]'s question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Next {
    /// What the roadmap starts now, and what it says about the ready stages it
    /// does not start.
    ///
    /// **Every ready stage there was a place for**, lowest number first — see
    /// [`AT_ONCE`], which is how many places a roadmap has until Settings says
    /// otherwise, and [`ready`], which
    /// is which of its stages want one. A list rather than one stage because each
    /// start is its own act from here on: one that halts halts itself, and the
    /// rest are started anyway.
    ///
    /// `starting` is empty where every ready stage is held, which is a roadmap
    /// already running as many as it runs at once. So this answer is not *the
    /// roadmap has something to start* — it is *this is what the roadmap has to
    /// say about what may start*, and the roadmap with nothing ready at all is
    /// [`Next::InFlight`].
    Stages {
        /// The stages to start, lowest number first.
        starting: Vec<Stage>,

        /// One sentence per ready stage that does not start, and what each of
        /// them is waiting on — see [`Held`].
        ///
        /// Said on the settled Conversation's Timeline as they stand, so that a
        /// roadmap which has gone quiet says why it went quiet — a stage waiting
        /// for a place is indistinguishable, from the sidebar, from a scheduler
        /// that forgot about it.
        held: Vec<Held>,
    },

    /// Every stage of it is done. The roadmap finished, and its directory stays
    /// where it is as the record of what it was.
    Complete {
        /// What it is called, for saying so.
        roadmap: String,
    },

    /// Nothing may start and the roadmap has not finished: what is left of it is
    /// in flight, or stands on something that is.
    ///
    /// The third answer, and the one that used to be folded into
    /// [`Next::Complete`] — a roadmap whose only unfinished stages are stages
    /// somebody is on has nothing to start and has not finished, and saying it
    /// was complete is how a roadmap worked side by side would announce itself
    /// finished half way through.
    InFlight {
        /// What it is called, for saying so.
        roadmap: String,
    },

    /// **Nothing of this roadmap may start**, and that is a thing to say: it is
    /// not on the branch its declarations are read off at all, its index plans
    /// nothing, or it declares badly.
    ///
    /// A thing to say rather than to guess past. A roadmap that has been renamed
    /// or emptied on that branch is the human's to fix, and falling back to some
    /// older reading of it would be Verkstead guessing at which roadmap this is.
    ///
    /// **A stage of it whose own brief is missing is not this.** That halts the
    /// one stage and no other: it is one of [`Next::Stages`]'s held sentences,
    /// and the rest of the ready stages start anyway.
    Unstartable {
        /// Why, in the words the Timeline says it in.
        why: String,
    },
}

/// One ready stage that did not start, in the words the Timeline says it in and
/// under what it is waiting on.
///
/// Three of them, and they are told apart because a reader's next move is
/// different for each: a stage waiting on its own roadmap waits on that
/// roadmap's own work, a stage waiting on the server waits on whatever else the
/// machine is running, and a stage that halted for its own brief waits on the
/// human writing one. See [`CONVERSATIONS_AT_ONCE`] for the second limit and
/// [`next_stage`], where all three are decided.
///
/// **And on which of them a look says out loud.** A settle says every one of
/// them, being the one reading that happens once. A look runs every
/// [`crate::Pace::places`] for as long as the server is up, so it says only the
/// wait that is about the thing it has just moved: a place on the server
/// changing hands — see [`crate::continuing::Brought`], where that is decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Held {
    /// Waiting for a place on **its own roadmap**: the stages of it already in
    /// flight have taken them, and one of them settling is what frees one.
    Roadmap(String),

    /// Waiting for a place **on the server**: the Conversations Verkstead is
    /// running have taken them all, whatever roadmap or Process they belong to,
    /// and nothing about this roadmap will free one.
    Server(String),

    /// Not waiting at all — **halted for itself**: its line names no brief, or
    /// the brief it names is not there to prime it from. The human's to fix, and
    /// it holds up no other stage.
    Halted(String),
}

impl Held {
    /// What it says, which is what goes on a Timeline.
    pub(crate) fn said(&self) -> &str {
        match self {
            Self::Roadmap(said) | Self::Server(said) | Self::Halted(said) => said,
        }
    }
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
    /// What to call the branch this stage is worked on: [`STAGES`], then the
    /// roadmap it belongs to, then its brief's name as the brief is named —
    /// `docs/roadmaps/mvp/04-wrap-up.md` becomes `roadmaps/mvp/04-wrap-up`.
    ///
    /// Under the roadmap's own name rather than the bare slug, because the bare
    /// slug is a name the repository may already be using for something that has
    /// nothing to do with any roadmap. A stage whose branch name is taken is one
    /// Verkstead will not start — and that refusal happens at the end of an
    /// unattended run, where a roadmap that stops advancing is a roadmap nobody
    /// is told about until they come and look. Qualified this way the only thing
    /// it can collide with is another attempt at the same stage of the same
    /// roadmap, which is exactly the collision the refusal is for.
    ///
    /// And under [`STAGES`] rather than straight under the roadmap — which is
    /// what [`former_branch`](Self::former_branch) still is — because git keeps
    /// a branch as a file under `refs/heads/`. `refs/heads/mvp` being a file is
    /// `refs/heads/mvp/` never being a directory, so a roadmap whose own
    /// Conversation branch is named for the roadmap, which is what a roadmap
    /// Conversation is usually called, blocks every stage that roadmap plans —
    /// and closing the Conversation keeps the branch, so it blocks them for
    /// good. A fixed component in front puts the whole scheme out from under
    /// every name a roadmap can have. See [`in_the_way`], which is what says so
    /// where even that is standing on something.
    ///
    /// The number kept in front of the slug for the same reason it is in the
    /// brief's name: it is what puts the stages of one roadmap in their order,
    /// wherever they are listed — and `git branch` lists them together, the
    /// roadmap being a path component of each.
    ///
    /// The brief's name rather than the title, because it is already a slug
    /// somebody chose and it is what the roadmap's own annotation will name.
    /// A brief with nothing usable in its name falls back to the number alone,
    /// which is the one thing every entry has.
    pub(crate) fn branch(&self) -> String {
        format!("{STAGES}/{}/{}", self.roadmap, self.named())
    }

    /// What this same stage was called before [`STAGES`] went in front of it.
    ///
    /// Asked wherever a stage is looked at for having been started already, and
    /// asked for good rather than for a while. A stage started under the former
    /// scheme is on a branch of that shape for as long as the branch is there,
    /// and the commit ticking its box rides on that branch until its pull
    /// request merges — so the branch is the only thing saying the stage is
    /// under way, and a reading that did not ask would offer the stage a second
    /// time. One ref lookup, and it never lies.
    pub(crate) fn former_branch(&self) -> String {
        format!("{}/{}", self.roadmap, self.named())
    }

    /// The part both shapes share: the stage's number, and its brief's slug
    /// where the brief's name has one to give.
    fn named(&self) -> String {
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
            true => self.label.clone(),
            false => format!("{}-{slug}", self.label),
        }
    }
}

/// The branch of `repo` standing where a component of `branch`'s own path would
/// go, where there is one.
///
/// Git keeps a branch as a file under `refs/heads/`, so a branch whose name is a
/// prefix of another's path is a file where that other one needs a directory:
/// `roadmaps` and `roadmaps/mvp/01-packaging` cannot both exist, and git refuses
/// to make the second while the first is there. What it refuses with goes to the
/// server log, and what the human is left with is a stage that did not start.
///
/// So every proper prefix is asked about before the branch is cut — `roadmaps`
/// and `roadmaps/mvp`, for `roadmaps/mvp/01-packaging` — and the one that is
/// there is named in the refusal. Rare, now that [`STAGES`] is in front of every
/// stage: it takes somebody having a branch called `roadmaps`, or one named for
/// a roadmap underneath it. But the moment it happens the roadmap stalls, and a
/// stall nobody is told the reason for is one nobody fixes.
///
/// [`worktrees::branch_at`] rather than [`worktrees::branch_exists`], because a
/// read git would not make is answered as *there is something there*: what this
/// stands in front of is making a branch and letting an agent loose on it.
///
/// And rather than [`worktrees::branch_taken`], which is the wrong question by
/// one word. That one is *is this name free*, and a name with `roadmaps/mvp/01`
/// under it is not free — which is right where it is asked and wrong here, a
/// roadmap with a stage branch already cut being what every working roadmap
/// looks like. What is in the way is a branch *at* the prefix, and nothing else.
pub(crate) fn in_the_way(repo: &Path, branch: &str) -> Option<String> {
    branch
        .match_indices('/')
        .map(|(end, _)| &branch[..end])
        .find(|prefix| worktrees::branch_at(repo, prefix))
        .map(str::to_owned)
}

/// **Where** a roadmap's declarations are read: the branch holding the newest of
/// them, and the commit that branch is at.
///
/// Two halves of one fact, passed together rather than as two strings in a row
/// for the reason `store::RoadmapStage`'s two are: the commit is what is read and
/// the branch is what the reading is named after, and a caller that put them the
/// other way round would read a roadmap at a branch name.
///
/// Which branch that is, is the carry-on's to decide — see
/// `crate::continuing::declaring`, which is the whole of that rule: the top of the
/// roadmap's chain, its own Conversation's branch while nothing has joined, and
/// the branch that has just settled where git holds neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Declaring<'a> {
    /// The branch the reading came off, for saying so. Nothing is looked up by
    /// it: what is read is the commit beside it, this being the name that commit
    /// was resolved from.
    pub(crate) branch: &'a str,

    /// And the commit itself, which is what every byte here is read at.
    pub(crate) commit: &'a str,
}

/// What `roadmap` has left to start, with `branch` the Conversation that has
/// just finished.
///
/// One roadmap, named by the caller. *Which* one is not a question this asks: it
/// was settled when the stage started and is read back out of the record — see
/// `store::stage_roadmap` and the module doc. So a roadmap this branch amended in
/// passing is never read here, and neither is a roadmap somebody else is carrying
/// on: what the branch touched decides nothing at all.
///
/// **Read at a commit out of the Repo's own git directory**, the way [`startable`]
/// reads one for the adoption, rather than out of the settling stage's checkout.
/// With stages worked side by side each Worktree holds a `ROADMAP.md` of its own,
/// and the settling stage's was very likely cut before a dependency was edited or
/// a stage added — so **declarations are read afresh at every start**, off the
/// branch holding the newest of them. Which branch that is, is [`Declaring`]'s,
/// and the reading is the index, the declarations on its lines and the stage's
/// brief together: all three at the one commit, so that what starts and what it is
/// primed with cannot come from two readings of the roadmap.
///
/// What it has left is not the lowest unchecked box: which of its stages may start
/// now is [`ready`]'s question, and `record` is the half of it the repository does
/// not hold — what Verkstead's own record says about each stage of this roadmap in
/// this Repo. The Conversation's own stage settling is what brought this reading
/// about, so the record is what skips it, and the annotation the roadmap keeps
/// beside the line is the fallback where the record has no label for it.
///
/// **Every ready stage there is a place for**, lowest number first. `at_once` is
/// how many stages of one roadmap run at once, which is the setting the carry-on
/// reads and [`AT_ONCE`] where nobody has set it — and every stage
/// the record has in flight is already holding one of those places, so what starts
/// here is what is left over. Where there are more ready stages than places the
/// lowest-numbered ones start and the rest are held: the roadmap's order is still
/// the roadmap's own, and a stage that waited is told so rather than dropped.
///
/// **And a place on the server, which is the second limit**, in front of the
/// first. `server_places` is how many of them are left over once every
/// Conversation with a session running or a driver registered has taken its own —
/// the carry-on counts them and hands the remainder in, the way it hands in the
/// roadmap's number, so that how many are permitted stays the caller's to say. See
/// [`CONVERSATIONS_AT_ONCE`], which is how many there are where Settings has not
/// said, and [`crate::drivers::Drivers::taking`], which is the counting.
///
/// The two are spent together and a stage that starts spends one of each, so a
/// ready stage that fits under its roadmap's limit and not under the server's is
/// held with **a sentence of its own**: a roadmap waiting on the machine and a
/// roadmap waiting on itself are two different things to be waiting for, and the
/// human's next move is different for each.
///
/// **One brief read per stage that starts**, and a stage whose brief is not there
/// halts itself alone — held with its own sentence while the rest of the ready
/// stages go on starting. A brief nobody wrote is the human's to fix, and stopping
/// a whole roadmap's fan for one of them would be a stage's fault spreading to its
/// siblings. **And it leaves its place behind it**: a place is spent by a stage
/// that starts and by nothing else, so the ready stage after it takes the one that
/// stage would have had. Otherwise a roadmap allowed three would run two and tell
/// the third its places were taken.
///
/// A value rather than a lookup, so this stays a reading: the rows are read once,
/// by the caller, and nothing in here asks the database.
///
/// A roadmap that is not there to read is [`Next::Unstartable`], which is the
/// treatment a stage naming a brief nobody wrote gets and for the same reason:
/// the record says this Conversation is a stage of it, so the directory being
/// renamed away or emptied where the declarations are read is the human's to look
/// at rather than Verkstead's to guess past. Falling back to whatever else the
/// branch touched is exactly the guess this stopped making — and so is falling
/// back to a reading of the same roadmap somewhere older.
///
/// Blocking work: one git read, and one more per stage that starts.
pub(crate) fn next_stage(
    repo: &Path,
    read: Declaring<'_>,
    roadmap: &str,
    branch: &str,
    record: &store::StageStandings,
    at_once: usize,
    server_places: usize,
) -> Next {
    let index = format!("{ROADMAPS}/{roadmap}/{INDEX}");

    let Some(list) = at(repo, read.commit, &index) else {
        return Next::Unstartable {
            why: format!(
                "this Conversation is a stage of the {roadmap} roadmap, and there is no {index} \
                 on `{}` — the branch its declarations are read off — to read what it has left",
                read.branch,
            ),
        };
    };

    if list.lines().filter_map(checklist::entry).next().is_none() {
        // A directory under `docs/roadmaps/` with an index that plans nothing is
        // not a roadmap, exactly as it is not one to pin — and the record saying
        // this Conversation is a stage of it is what makes that worth saying out
        // loud rather than passing over.
        return Next::Unstartable {
            why: format!(
                "this Conversation is a stage of the {roadmap} roadmap, and its {INDEX} on \
                 `{}` has no stages in it",
                read.branch,
            ),
        };
    }

    // Which of its stages may start is [`ready`]'s answer, and how many of them
    // are already under way comes back with it. Another roadmap in this Repo
    // having work left is not a reason to start any of it.
    let (ready, in_flight) = match ready(roadmap, &list, record, branch) {
        Ready::Stages {
            lowest,
            rest,
            in_flight,
        } => (
            std::iter::once(lowest).chain(rest).collect::<Vec<_>>(),
            in_flight,
        ),
        Ready::Complete => {
            return Next::Complete {
                roadmap: roadmap.to_owned(),
            };
        }
        Ready::InFlight => {
            return Next::InFlight {
                roadmap: roadmap.to_owned(),
            };
        }
        // Refused rather than repaired, and never run in order instead — see
        // [`declarations::judge`], which is where the sentence is written. Saying
        // it where the press can see it is the other half of the refusal.
        Ready::Misdeclared(why) => return Next::Unstartable { why },
    };

    // What is left over once the stages somebody is on have taken theirs. Saturating
    // because the record can hold more in flight than the limit allows — the limit
    // is lowered in Settings, or a stage was started by hand — and a roadmap over
    // its limit starts nothing rather than going backwards.
    let places = at_once.saturating_sub(in_flight);

    let mut starting = Vec::new();
    let mut held = Vec::new();

    for entry in ready {
        // Beyond the places there are, so it waits — and is told so. It starts the
        // moment one of the stages ahead of it settles, which is a settle that runs
        // this reading again.
        //
        // Counted off what has **started** rather than off how far down the ready
        // stages this has walked: a place is spent by a stage that starts and by
        // nothing else, so a stage held below for a brief nobody wrote leaves its
        // place to whatever is ready after it. Counting positions would have this
        // roadmap run fewer stages than it is allowed and tell one of them its
        // places were taken while one stood free.
        if starting.len() >= places {
            // How many are in flight is deliberately not said: three of them may
            // have started a moment ago, in this very reading, and the record the
            // count came off was read before any of that.
            held.push(Held::Roadmap(format!(
                "Stage {} of the `{roadmap}` roadmap — *{}* — is ready and waiting for a place: \
                 this roadmap runs {} at a time, and its places are taken. It starts when one of \
                 them settles.",
                entry.label,
                entry.title,
                how_many_places(at_once),
            )));

            continue;
        }

        // And the second limit, which this one fits under its roadmap's and not
        // under: the places the server has are taken by Conversations of every
        // kind, and this stage would be one more of them.
        //
        // A sentence of its own rather than the one above, because what it is
        // waiting for is a different thing: nothing about this roadmap will free
        // the place, and the human reading the Timeline can see for themselves
        // what the server is busy with. Counted off `starting` for the reason the
        // roadmap's places are — a place is spent by a stage that starts and by
        // nothing else.
        if starting.len() >= server_places {
            // How many are taken is deliberately not said, for the reason the
            // stages in flight are not: the registers were read before any of
            // this, and a Conversation may have finished since. What says the
            // number as of the moment it is asked is the settings pane.
            held.push(Held::Server(format!(
                "Stage {} of the `{roadmap}` roadmap — *{}* — is ready and waiting for a place \
                 on the server: the Conversations Verkstead is running have taken them all, \
                 whatever roadmap or Process they belong to. It starts when one of them comes \
                 free.",
                entry.label, entry.title,
            )));

            continue;
        }

        // An entry naming no brief at all, which is a line to say something about
        // rather than a path to go and read: `<commit>:docs/roadmaps/<name>/` is
        // the roadmap's own directory, and git would hand back a listing of it.
        if entry.link.is_empty() {
            held.push(Held::Halted(format!(
                "Stage {} of the `{roadmap}` roadmap was ready and could not be started: its \
                 line names no brief to start it from.",
                entry.label,
            )));

            continue;
        }

        let brief_path = format!("{ROADMAPS}/{roadmap}/{}", entry.link);

        let Some(markdown) = at(repo, read.commit, &brief_path) else {
            held.push(Held::Halted(format!(
                "Stage {} of the `{roadmap}` roadmap was ready and could not be started: it \
                 names the brief `{brief_path}`, and there is nothing there to read on `{}`.",
                entry.label, read.branch,
            )));

            continue;
        };

        starting.push(Stage {
            brief_path,
            roadmap: roadmap.to_owned(),
            label: entry.label.to_owned(),
            title: entry.title.to_owned(),
            brief: markdown,
        });
    }

    Next::Stages { starting, held }
}

/// How many places a roadmap has, said in a sentence.
///
/// The number is a setting now — see [`crate::settings::AtOnce`] — so *at most 1
/// stages* is a thing a Timeline can be asked to say, and a roadmap somebody set
/// to one is the roadmap most likely to be waiting for a place at all.
fn how_many_places(at_once: usize) -> String {
    match at_once {
        1 => "one stage".to_owned(),
        many => format!("at most {many} stages"),
    }
}

/// What one roadmap may start now: the stages that are ready, or which of the
/// ways it has none.
///
/// Three answers beside the stages, and the third of them is the one that used to
/// be missing — see [`Ready::InFlight`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ready<'a> {
    /// The stages that may start now, lowest number first, beside the count of
    /// places the roadmap has already given away.
    ///
    /// Split into the lowest and the rest because the answer is never *no
    /// stages* — that is one of the three below — and because the adoption's
    /// reading still takes the lowest and leaves the rest where they are.
    Stages {
        /// The lowest-numbered of them.
        lowest: checklist::Entry<'a>,

        /// And the rest behind it, in the roadmap's own order.
        rest: Vec<checklist::Entry<'a>>,

        /// How many stages of this roadmap the record has **in flight**, each of
        /// them holding one of its places — see [`AT_ONCE`], which is how many
        /// there are where Settings has not said.
        ///
        /// Counted here rather than by the caller because this is where the
        /// record and the boxes have already been read against each other: a
        /// stage settled is not in flight whatever its row says, and a stage
        /// whose box is ticked on the branch being read may still be somebody's.
        in_flight: usize,
    },

    /// Every stage of it is done: the roadmap finished.
    Complete,

    /// Nothing may start and the roadmap has not finished. What is left of it is
    /// in flight, or stands on something that is.
    ///
    /// **A third answer rather than the roadmap complete**, which is what it used
    /// to be folded into. A roadmap whose only unfinished stages are stages
    /// somebody is on has nothing to start *now* and will have something to start
    /// the moment one of them settles, and the two readings say so in their own
    /// words.
    ///
    /// Nothing ready and something left over is always this: the stages that are
    /// left either are in flight or stand on one that is not done, and walking
    /// down what is not done through a graph with no cycle in it — a cycle being
    /// refused — reaches a stage whose every dependency is done. That stage would
    /// be ready were somebody not on it.
    InFlight,

    /// The roadmap declares badly, so nothing of it may start anywhere — with the
    /// fault in the words [`declarations::judge`] refuses it in.
    ///
    /// Refused rather than repaired, and never run in order instead: falling back
    /// to running in order runs a roadmap in a way nobody wrote down.
    Misdeclared(String),
}

/// Which stages of `roadmap` may **start now**, lowest number first.
///
/// The one reading both starts go through — the carry-on that runs when a stage
/// settles, and the adoption's, which answers the roadmap notice, the compose page
/// and the press. Three things go into it and all three were already there: what
/// each stage's line declares it stands on, Verkstead's own record of what each
/// stage of this roadmap has got to, and the boxes the roadmap keeps.
///
/// A stage is **ready** when every stage it stands on has **settled** and it is
/// itself neither done nor in flight. Settled is [`done`]'s question, which is the
/// rule that joins the record to the boxes — and settled rather than merged: a
/// stage whose pull request is open and whose wrap-up reached Done is a stage the
/// ones above it stand on, which is the whole of how a roadmap runs.
///
/// **An undeclared roadmap is read as each stage standing on the one before it**,
/// as the roadmap lists them. That is what *in order* means to the scheduler, and
/// it is what keeps this one reading rather than two: every roadmap written before
/// any of this runs exactly as it did, and a stage in flight now holds up what
/// stands on it rather than being stepped over.
///
/// **Nothing ready does not mean the roadmap is complete** — see
/// [`Ready::InFlight`], which is that third answer.
///
/// `branch` is the Conversation whose settling brought the reading about, and it
/// is empty where none did: the adoption reads a roadmap that belongs to nobody
/// yet. A stage the roadmap's annotation says is on that branch counts as
/// settled, and that is the fallback in front of all of it — the Conversation
/// whose settling brought the reading about has a row of its own saying so, but
/// every stage started between ADR-0017 landing and the label being written down
/// has a row holding a roadmap and no label, and for one of those the annotation
/// naming its branch is the only thing that keeps it from being offered its own
/// stage back.
///
/// A reading: the record comes in as a value and nothing here asks the database or
/// git. Which is what lets one function answer for a Worktree as it stands and for
/// a Repo at a commit alike — the bytes are the caller's to fetch.
pub(crate) fn ready<'a>(
    roadmap: &str,
    list: &'a str,
    record: &store::StageStandings,
    branch: &str,
) -> Ready<'a> {
    let entries: Vec<checklist::Entry<'a>> = list.lines().filter_map(checklist::entry).collect();

    // What each stage stands on, by where in the list that stage is. The labels
    // are the roadmap's and the indices are ours: a declaration names stages of
    // its own roadmap, which the judging has already checked, so every label here
    // is one of these entries.
    let stands_on: Vec<Vec<usize>> = match declarations::judge(roadmap, list) {
        declarations::Judgement::Refused(why) => return Ready::Misdeclared(why),

        // Each stage standing on the one before it, which is what *in order* means
        // — and the first stage of one standing on nothing at all.
        declarations::Judgement::Undeclared => (0..entries.len())
            .map(|at| match at.checked_sub(1) {
                Some(before) => vec![before],
                None => Vec::new(),
            })
            .collect(),

        declarations::Judgement::Declared(declared) => entries
            .iter()
            .map(|entry| {
                declared
                    .iter()
                    .find(|one| one.label == entry.label)
                    .map(|one| {
                        one.stands_on
                            .iter()
                            .filter_map(|label| entries.iter().position(|one| one.label == *label))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect(),
    };

    // And what the record and the boxes say about each of them, once each: whether
    // it is done, and whether somebody is on it. The annotation comes first, so a
    // stage the record holds no label for is the settling Conversation's own rather
    // than a stage nothing stands on.
    let settled: Vec<bool> = entries
        .iter()
        .map(|entry| ours(entry.after, branch) || done(entry, record.of(roadmap, entry.label)))
        .collect();

    let on_it: Vec<bool> = entries
        .iter()
        .zip(&settled)
        .map(|(entry, settled)| {
            !settled && record.of(roadmap, entry.label) == Some(store::StageStanding::InFlight)
        })
        .collect();

    let mut ready: Vec<checklist::Entry<'a>> = entries
        .iter()
        .enumerate()
        .filter(|(at, _)| !settled[*at] && !on_it[*at])
        .filter(|(at, _)| stands_on[*at].iter().all(|stood| settled[*stood]))
        .map(|(_, entry)| *entry)
        .collect();

    // The roadmap's order is still the roadmap's own, and lowest first is what
    // decides which of them start where there are more than places for. A stable
    // sort, so a roadmap that numbers two lines alike keeps them as it wrote them.
    ready.sort_by_key(|entry| entry.number);

    // And what the roadmap has already given away, which is the other half of
    // what the scheduler needs: a place per stage somebody is on, whatever that
    // stage is doing. See [`AT_ONCE`], where the rest of that rule is.
    let in_flight = on_it.iter().filter(|on_it| **on_it).count();

    let mut ready = ready.into_iter();

    match ready.next() {
        Some(lowest) => Ready::Stages {
            lowest,
            rest: ready.collect(),
            in_flight,
        },
        None => match settled.iter().all(|settled| *settled) {
            true => Ready::Complete,
            false => Ready::InFlight,
        },
    }
}

/// Whether one stage of a roadmap is **done**: the rule that joins Verkstead's
/// record to the boxes the repository keeps.
///
/// **The record decides where it says something about the stage, and the box
/// decides where it does not** — see
/// [ADR-0021](../../../../docs/adr/0021-parallel-stages.md), which is where that
/// was settled, and `store::stage_standings`, which is where the record is read.
///
/// One rule in one function, spent by [`ready`] on every stage of a roadmap: what
/// is done is what the stages above it stand on, and what is not done is what may
/// yet start. Both readings of a roadmap go through that one reading — the
/// carry-on's, off a Worktree as it stands, and the adoption's, off a Repo at a
/// commit — because a stage that is done to one of them and open to the other is a
/// notice offering work somebody has finished, or a roadmap that stops advancing.
///
/// - **Settled** is done, whatever the box on the branch being read says. This is
///   the whole point of the record: with stages worked side by side each Worktree
///   holds a `ROADMAP.md` of its own, so the boxes stop being one fact — the stage
///   that settled ticked its own box in its own finish commit, on its own branch,
///   and the branch being read here may well never have seen it.
/// - **In flight** is not done. Newly load-bearing, and load-bearing in the other
///   direction from the box: a stage ticks its own box at its finish, before its
///   pull request has even opened, so a ticked box already means *its tasks are
///   done* rather than *it settled*.
/// - **Abandoned**, and **no row at all**, leave the box to speak. Nothing starts
///   an abandoned stage again on the strength of the record: what it left behind
///   is a branch, and a branch by the stage's name is what refuses it wherever a
///   stage is started from. A stage with no row is one worked by hand or by the
///   old tools, and its box is all there is to go on.
fn done(entry: &checklist::Entry<'_>, standing: Option<store::StageStanding>) -> bool {
    match standing {
        Some(store::StageStanding::Settled) => true,
        Some(store::StageStanding::InFlight) => false,
        Some(store::StageStanding::Abandoned) | None => entry.checked,
    }
}

/// **Where** every stage of a roadmap is, in the order `list` has them — which is
/// what the card's rows, the pane's headings and the pane's contents lines say.
///
/// [`done`] widened from a box into a word, and the same rule under it: the record
/// decides wherever it says anything about the stage, and the box decides wherever
/// it does not. What the two readings are *for* is what parts them — the
/// schedulers ask whether a stage is done, so that they know what may start, and
/// a human reading the roadmap asks where each of its stages has got to. So the
/// states that are not done are told apart here and are all one *not done* there.
///
/// - **Settled** is [`StageState::Done`], whatever the box on the branch being
///   drawn says, for [`done`]'s reason: each branch carries a `ROADMAP.md` of its
///   own and the boxes stop being one fact, while the record is one.
/// - **In flight** is [`StageState::InProgress`], unless the Conversation has
///   stopped — which is [`StageState::Halted`], along with **abandoned**. One word
///   for the two, because what a reader does about either is go and look at that
///   Conversation; and where it parts company with [`done`], which leaves an
///   abandoned stage's box to speak. Nothing is *started* on the strength of this
///   reading, so nothing here has to be careful the way that one does: what the
///   box says about a stage somebody walked away from is worth less to a reader
///   than the walking away.
/// - **In flight and held** before its finish is
///   [`StageState::WaitingToJoin`]: every task of it is done and the join waits on
///   the chain below it settling — see [`crate::joins`]. It beats *in progress*,
///   the hold being the more particular thing to say about a stage the record has
///   in flight, and loses to *halted*, a stopped Conversation being held by
///   nothing.
/// - **No row at all** leaves the box to speak. A ticked box is
///   [`StageState::Done`], and an unticked one is [`StageState::WaitingOn`] the
///   stages its own line stands on that have not settled, or
///   [`StageState::ToDo`] where there are none of those.
///
/// **The whole list rather than a stage at a time**, which is the one thing this
/// does that [`done`] does not have to: what a stage is waiting on is a fact about
/// its neighbours, so the declarations and each stage's settling are read once
/// over the file and the answers come back in its own order.
///
/// **Only a declaring roadmap waits on anything.** A roadmap that declares on no
/// line is scheduled as each stage standing on the one before it — see [`ready`],
/// where that reading of silence lives — but the silence is the scheduler's
/// reading rather than something the roadmap says, and *waiting on 03* about a
/// line that declares nothing would be this putting a declaration in the human's
/// mouth. So an undeclared roadmap's unstarted stages read [`StageState::ToDo`],
/// exactly as they always have.
///
/// A roadmap that declares **badly** says what its lines say all the same, the way
/// the pane draws them: nothing of it will start until the human fixes it, and the
/// lines are what they have to go and look at. Which is why the file is asked
/// whether it declares rather than what it declared — see [`declarations::judge`],
/// which answers the second question for a good roadmap only.
///
/// `held` is the joins register as it stands at the moment the page is drawn —
/// which Conversations are stages this server is holding — and it is asked by the
/// Conversation the record names for the label, that being the one thing joining a
/// register keyed by Conversation to a roadmap that keeps labels. Read here rather
/// than stored, for the reason the sidebar's own label is: the hold is a task of
/// this process, so a server that has just come back is holding nothing and such a
/// stage reads *in progress* again until the resume finds it held a second time.
/// The label and this word being the one register read the one way is the point of
/// it — a stage cannot read one thing on its own row and another on the roadmap's
/// card.
fn states(
    roadmap: &str,
    list: &str,
    record: &store::StageStandings,
    held: &HashSet<i64>,
) -> Vec<StageState> {
    let entries: Vec<checklist::Entry<'_>> = list.lines().filter_map(checklist::entry).collect();

    // Whether this roadmap declares at all, which is a fact about the whole file
    // rather than about a line — see [`opened`], which asks it the same way for
    // the same reason.
    let declaring = !matches!(
        declarations::judge(roadmap, list),
        declarations::Judgement::Undeclared
    );

    // And whether each of them has settled, once each: the same question [`ready`]
    // asks of every stage before it decides what may start, and the answer *that*
    // reading stands what is above a stage on.
    let settled: Vec<bool> = entries
        .iter()
        .map(|entry| done(entry, record.of(roadmap, entry.label)))
        .collect();

    entries
        .iter()
        .map(|entry| match record.of(roadmap, entry.label) {
            Some(store::StageStanding::Settled) => StageState::Done,
            Some(store::StageStanding::Abandoned) => StageState::Halted,
            Some(store::StageStanding::InFlight) => {
                if record.stopped(roadmap, entry.label) {
                    StageState::Halted
                } else if holding(roadmap, entry.label, record, held) {
                    StageState::WaitingToJoin
                } else {
                    StageState::InProgress
                }
            }
            None if entry.checked => StageState::Done,
            None => match waiting_on(entry, &entries, &settled, declaring) {
                stages if stages.is_empty() => StageState::ToDo,
                stages => StageState::WaitingOn { stages },
            },
        })
        .collect()
}

/// What one unstarted stage is **waiting on**: the stages its own line declares
/// it stands on that have not settled, by the labels the roadmap writes them
/// under.
///
/// Empty where there are none of those, which is the three ways a stage is not
/// waiting on a stage: the roadmap declares on no line, the line is a root, or
/// everything it stands on has settled already. The last of those is a stage
/// waiting for a place or halted for itself rather than for a neighbour, and
/// saying nothing about it here is what leaves room for those.
///
/// The labels are the *entries'* rather than the declaration's, so that what is
/// drawn is what the human reads the lines by — the two are the same string for a
/// roadmap that declares well, the judging having checked every label against the
/// roadmap's own, and a label naming no stage of it is dropped the way [`ready`]
/// drops one.
fn waiting_on(
    entry: &checklist::Entry<'_>,
    entries: &[checklist::Entry<'_>],
    settled: &[bool],
    declaring: bool,
) -> Vec<String> {
    if !declaring {
        return Vec::new();
    }

    // Read off the same tail the in-flight annotation lives in, the two sharing it
    // in either order — see [`opened`], which reads it for what the pane says a
    // stage stands on.
    let Some(declarations::StandsOn::Stages(stages)) = declarations::read(entry.after).stands_on
    else {
        return Vec::new();
    };

    stages
        .iter()
        .filter_map(|named| entries.iter().position(|one| one.label == *named))
        .filter(|stood| !settled[*stood])
        .map(|stood| entries[stood].label.to_owned())
        .collect()
}

/// Whether this server is **holding** stage `label`'s Conversation before its
/// finish: every task of it is done and the join waits on the chain below it
/// settling.
///
/// The record says which Conversation the stage is and the register says which
/// Conversations are held, so the two are asked in that order. A stage the record
/// holds no row for is held by nothing — there is no Conversation to hold — which
/// is every stage worked by hand or by the old tools.
///
/// The same register the sidebar's *Waiting to join* label and the status button's
/// words are read off, read at the same moment a page is drawn — see
/// [`crate::joins`], where what that buys and what it costs are written down.
fn holding(
    roadmap: &str,
    label: &str,
    record: &store::StageStandings,
    held: &HashSet<i64>,
) -> bool {
    record
        .conversation(roadmap, label)
        .is_some_and(|conversation_id| held.contains(&conversation_id))
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
fn roadmap(
    directory: &Path,
    record: &store::StageStandings,
    held: &HashSet<i64>,
) -> Option<StageListEvent> {
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

    // Which roadmap this is, which is what the record answers about a stage by:
    // a Repo may hold any number of roadmaps and a label means nothing on its own.
    let name = name(directory);

    // The record where it says anything about a stage, the box where it does not,
    // and what the line stands on where nothing has started it — see [`states`],
    // which is that rule, read over the whole file because what a stage waits on
    // is a fact about its neighbours.
    let stages: Vec<StageEntry> = list
        .lines()
        .filter_map(checklist::entry)
        .zip(states(&name, &list, record, held))
        .map(|(entry, state)| StageEntry {
            number: entry.label.to_owned(),
            title: entry.title.to_owned(),
            state,
        })
        .collect();

    if stages.is_empty() {
        return None;
    }

    Some(verkstead_render::stage_list(
        name,
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

/// The roadmap opened: every stage brief of it, rendered, in the order the index
/// has them.
///
/// `None` for everything [`showing`] draws no card for, and for the same reasons
/// — no Worktree, no base commit, or nothing under that name this branch has
/// written to and can read as a roadmap. All of them are a pane with nothing to
/// draw, which is a 404 at the route.
///
/// `name` comes off the card that was pressed, which is a directory name out of
/// this Conversation's own reading. It is checked against [`touched`] all the
/// same, before anything is joined onto a path: the roadmaps this branch wrote
/// to are the roadmaps this Conversation is about, so the check is the answer to
/// *which roadmap is this* and to *whose name is this* at once.
///
/// A brief is found by the number its file name leads with rather than by the
/// link the entry carries, for [`crate::tasks::documents`]'s reason: the number
/// is what the roadmap turns on, and a link is a string out of a file in a
/// repository.
///
/// `record` and `held` are the same record and the same register the card was
/// drawn against, and reach this the same way — see [`showing`], which is where
/// why is written down.
///
/// Blocking work, so it happens off the runtime's threads — this is a git read,
/// a directory read and a file read per stage.
pub(crate) async fn documents(
    worktree: Option<PathBuf>,
    base: Option<String>,
    name: String,
    record: store::StageStandings,
    held: HashSet<i64>,
) -> Option<RoadmapPane> {
    let (Some(worktree), Some(base)) = (worktree, base) else {
        return None;
    };

    match tokio::task::spawn_blocking(move || opened(&worktree, &base, &name, &record, &held)).await
    {
        Ok(pane) => pane,
        Err(error) => {
            tracing::error!(error = ?error, "reading a Worktree's stage briefs failed");
            None
        }
    }
}

/// The briefs of `worktree`'s `name` roadmap, or `None` where there is no
/// roadmap of that name to open — which is what [`roadmaps`] would leave out,
/// said the same way.
fn opened(
    worktree: &Path,
    base: &str,
    name: &str,
    record: &store::StageStandings,
    held: &HashSet<i64>,
) -> Option<RoadmapPane> {
    if !touched(worktree, base).contains(name) {
        return None;
    }

    let directory = worktree.join(ROADMAPS).join(name);

    let list = std::fs::read_to_string(directory.join(INDEX)).ok()?;

    let files = briefs(&directory);

    // Whether this roadmap declares at all, which is a fact about the whole file
    // rather than about a line — see [`declarations::judge`], which is asked here
    // for the same reason it refuses over the file at once. A roadmap written
    // before there was anything to declare has prose after its link, and a word
    // following an `on` in it is not a platform somebody typed wrong: such a
    // roadmap is drawn exactly as it always was, which is the promise every part
    // of this makes. A roadmap that declares badly still shows what its lines
    // say, that being what the human has to go and fix.
    let declaring = !matches!(
        declarations::judge(name, &list),
        declarations::Judgement::Undeclared
    );

    let stages: Vec<StageSource> = list
        .lines()
        .filter_map(checklist::entry)
        .zip(states(name, &list, record, held))
        .map(|(entry, state)| {
            // What its line declares, read off the same tail the in-flight
            // annotation lives in: the two share it in either order and neither
            // reading trips on the other.
            let declared = declaring.then(|| declarations::read(entry.after));

            StageSource {
                number: entry.label.to_owned(),
                title: entry.title.to_owned(),
                // The same state the card's row says, off the same reading — see
                // [`states`]. A stage's brief stays where it is for ever, so a
                // stage that is over has a document like any other and the
                // section says where it is on its heading.
                state,
                // The root comes over as the empty list, which is what the pane
                // draws it as: *stands on nothing*.
                stands_on: declared
                    .as_ref()
                    .and_then(|declared| declared.stands_on.as_ref())
                    .map(|stands_on| match stands_on {
                        declarations::StandsOn::Nothing => Vec::new(),
                        declarations::StandsOn::Stages(stages) => {
                            stages.iter().map(|named| (*named).to_owned()).collect()
                        }
                    }),
                platform: declared
                    .as_ref()
                    .and_then(|declared| declared.platform)
                    .map(str::to_owned),
                // Absent only where the roadmap names a brief nobody wrote, or
                // one that will not be read. Both are the same nothing to draw,
                // and the pane says so in words.
                markdown: files
                    .get(&entry.number)
                    .and_then(|file| std::fs::read_to_string(directory.join(file)).ok()),
            }
        })
        .collect();

    if stages.is_empty() {
        return None;
    }

    Some(verkstead_render::roadmap_pane(
        name.to_owned(),
        checklist::heading(&list),
        stages,
    ))
}

/// The stage briefs in a roadmap's directory, by the number each of them leads
/// with.
///
/// `ROADMAP.md` is not one of them, carrying no number — which is the index
/// leaving itself out for free. A directory that will not be read comes back
/// empty, and every stage is drawn as a brief that is not there to read, which
/// is what it would be.
fn briefs(directory: &Path) -> HashMap<u32, String> {
    let Ok(listed) = std::fs::read_dir(directory) else {
        return HashMap::new();
    };

    listed
        .flatten()
        .filter_map(|file| {
            let name = file.file_name().to_string_lossy().into_owned();
            Some((crate::tasks::numbered(&name)?, name))
        })
        .collect()
}

/// Why any of `names` — roadmaps of `worktree`, by directory name — declares
/// badly, in the words [`declarations::judge`] refuses it in, or `None` where
/// every one of them is a roadmap something could run.
///
/// The roadmaps this branch has written to, as [`touched`] hands them over,
/// because that is the same reading the landing this stands beside is made of:
/// a roadmap the branch is answerable for is one the session at this terminal
/// can put right. In directory-name order, so a branch that wrote two of them
/// and got both wrong is refused over the same one every time it signals.
///
/// A directory with no readable `ROADMAP.md` is nothing to judge, exactly as it
/// is nothing to pin — see [`roadmap`]. And an undeclared roadmap is no fault at
/// all: that is every roadmap written before any of this, and it runs strictly in
/// order as it always did.
///
/// Blocking work: one file read per roadmap.
pub(crate) fn misdeclared(worktree: &Path, names: &BTreeSet<String>) -> Option<String> {
    names.iter().find_map(|name| {
        let list = std::fs::read_to_string(worktree.join(ROADMAPS).join(name).join(INDEX)).ok()?;

        match declarations::judge(name, &list) {
            declarations::Judgement::Refused(why) => Some(why),
            declarations::Judgement::Undeclared | declarations::Judgement::Declared(_) => None,
        }
    })
}

/// A roadmap in a registered Repo with a stage that could start now, and the
/// stages adopting it would start.
///
/// **Abandoned** is the workbench's word for it, and the whole of what it means
/// is *there is a stage startable right now and nothing is on that stage* — see
/// [`startable`] for the four clauses. A roadmap that has finished, one whose
/// every ready stage somebody is already working, one whose places are all taken
/// and one whose next brief is missing are all not abandoned, and none of them is
/// a state to draw: what the human can do something about is the only thing worth
/// saying.
///
/// **Not the same thing as a roadmap nothing is driving**, which is what this
/// meant while a roadmap ran its stages one at a time — there the one stage that
/// could start was the whole roadmap, so a stage in flight was a roadmap in
/// flight. A declaring roadmap has stages side by side, and one with a ready
/// stage beside the ones somebody is on is a roadmap being driven *and* one with
/// work the press can pick up. So this is drawn off what may start rather than
/// off whether anybody is anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Abandoned {
    /// What the roadmap calls itself in its heading, or empty where it has
    /// none.
    pub(crate) title: String,

    /// The stage adopting it would start — the lowest-numbered one that may
    /// start now, its brief already read.
    pub(crate) stage: Stage,

    /// And the rest of the stages the press would start beside it, lowest
    /// number first, each with its brief read too.
    ///
    /// Empty for a roadmap with one stage ready, which is every roadmap that
    /// declares nothing — *in order* means each stage standing on the one
    /// before it, and one stage at a time is what that comes to. A declaring
    /// roadmap has as many as it has lines standing on work that has settled,
    /// and this holds as many of those as the roadmap has places left for.
    ///
    /// **Only the ones the press could actually start.** One of the rest whose
    /// brief is not there, whose branch is taken, or that somebody has
    /// annotated is left out rather than named: what is offered is what pressing
    /// does, and one line the roadmap has not finished writing is no reason to
    /// hold up the siblings beside it. The lowest is not treated that way — it
    /// is what the Conversation itself becomes, so what is wrong with it refuses
    /// the press by name.
    pub(crate) beside: Vec<Stage>,

    /// Whether the commit this was read at is **missing the work of a stage below
    /// the one offered**: a stage the record says settled whose box is still
    /// unticked here, which is its finish commit — and its work — sitting on a
    /// branch this commit does not hold.
    ///
    /// What it is for is [`waiting`], which reads one roadmap at several commits
    /// and draws each stage once: a reading that is behind offers work the base
    /// under it cannot build on, so it loses to a reading of the same stage that
    /// is not. Nothing about the stage differs between the two — it is the base
    /// that does.
    ///
    /// `false` wherever the boxes and the record agree, which is every roadmap
    /// read at a commit that holds everything settled so far, and every roadmap
    /// the record knows nothing about.
    pub(crate) behind: bool,
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

    /// The roadmap declares badly, so nothing of it may start — with the fault
    /// in the words [`declarations::judge`] refuses it in.
    ///
    /// A clause of its own rather than folded into [`Startable::NoRoadmap`],
    /// because the roadmap is there and readable and what is wrong with it is a
    /// line the human can go and fix. The notice and the page draw nothing off
    /// it either way — both keep only [`Startable::stage`] — and it is the press
    /// this carries a sentence for: that is the one path with somebody waiting
    /// on the answer, and a roadmap written by hand or by the old tools, which
    /// is what adoption is for, is the likeliest to declare badly.
    ///
    /// The judgement's own sentence, word for word, so that a roadmap refused
    /// here and the same roadmap refused on a Timeline or at `verkstead done`
    /// read as one fault rather than three.
    Misdeclared {
        /// Why, as [`declarations::Judgement::Refused`] put it.
        why: String,
    },

    /// Every stage of it is done. The roadmap finished, and its directory stays
    /// where it is as the record of what it was.
    Complete,

    /// Somebody — or some unattended run — is on what is left of it: the record
    /// says so of every stage that could otherwise have started, or, where it says
    /// nothing about the stage this reading landed on, its annotation names a
    /// branch that is still there.
    ///
    /// **And a roadmap whose places are all taken**, which is the same answer for
    /// the same reason: every one of them is held by a stage somebody is on, so
    /// what a roadmap with lines ready and nowhere to run them is waiting for is
    /// one of those to settle. See [`AT_ONCE`], which is how many places there are.
    ///
    /// Not the roadmap complete and not a stage to offer, which is why it is an
    /// answer of its own: one of them settling is what gives this roadmap something
    /// to start again.
    InFlight,

    /// The next stage names a brief that cannot be read at this commit — or
    /// names none at all.
    NoBrief,

    /// The stage's own branch is taken in the Repo, which is a stage
    /// already under way whatever the boxes say — under the name it has now, or
    /// under the [former one](Stage::former_branch).
    BranchTaken,

    /// A branch of the Repo stands where a component of the stage's branch path
    /// would go, so git will not make that branch at all — see [`in_the_way`],
    /// which is what found it.
    ///
    /// Carries the branch that is in the way, because that is the whole of what
    /// there is to go and do about it: no other clause of this reading can tell
    /// the human which name to rename, and git's own refusal never reaches them.
    BranchInTheWay {
        /// The branch of the Repo standing in the stage's way.
        by: String,
    },
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
///
/// What Verkstead's record says about each of them is read here, in front of that
/// task: one read per Repo rather than one per roadmap inside it, and what the
/// readings take is the value — see [`done`], which is the rule they spend it on.
/// A Repo whose rows will not come back says nothing at all rather than falling
/// back to its boxes, because the boxes alone are what offer a stage that has
/// already settled.
pub(crate) async fn abandoned(
    pool: &SqlitePool,
    repos: Vec<store::Repo>,
    at_once: usize,
) -> Vec<AbandonedRepo> {
    let mut reading = Vec::with_capacity(repos.len());

    for repo in repos {
        let repo_id = repo.id;

        match store::stage_standings(pool, repo_id).await {
            Ok(record) => reading.push((repo, record)),
            Err(error) => tracing::error!(
                error = ?error,
                repo_id,
                "reading what a Repo's roadmap stages have got to failed",
            ),
        }
    }

    let read = tokio::task::spawn_blocking(move || {
        reading
            .iter()
            .filter_map(|(repo, record)| notice(repo, record, at_once))
            .collect::<Vec<_>>()
    })
    .await;

    read.unwrap_or_else(|error| {
        tracing::error!(error = ?error, "reading the registered Repos' roadmaps failed");
        Vec::new()
    })
}

/// One Repo's notice, or `None` where it has nothing to adopt.
///
/// The reading is [`waiting`] below; what this adds is the notice's own rule,
/// which is that a Repo with nothing to adopt has no notice at all rather than
/// an empty one.
fn notice(
    repo: &store::Repo,
    record: &store::StageStandings,
    at_once: usize,
) -> Option<AbandonedRepo> {
    let roadmaps = waiting(repo, record, at_once);

    (!roadmaps.is_empty()).then(|| AbandonedRepo {
        repo_id: repo.id,
        repo: repo.name.clone(),
        roadmaps,
    })
}

/// The roadmaps one Repo is holding with a stage that could start now, whether or
/// not there are any — see [`Abandoned`], which is what *abandoned* comes to now
/// that a roadmap can have stages worked side by side.
///
/// Read at the default branch's tip as origin holds it, which is the repository
/// as everyone working on it sees it — the same ref [`adopting`] draws the page
/// from and [`crate::conversations::adopt`] branches off, so what is offered
/// here and what happens when it is pressed are one reading. A local default
/// branch that has not been pulled for a week would offer a roadmap somebody
/// finished and hide one somebody pushed. A repository with no branch that
/// resolves has nothing to read and nothing to say — the same shrug the pinned
/// lists make.
///
/// **And at every local branch the default branch has not swallowed**, because
/// a roadmap is a document on a branch like any other and the default branch is
/// only one place it can be. A roadmap staged on a branch whose pull request is
/// still open is invisible at the default tip, and so is every stage of it — so
/// the effort that most needs carrying on is the one nothing offers. Each
/// reading names the branch it came off, which is what the press fixes the new
/// Conversation's base to: a roadmap found on a branch is a roadmap adopted
/// from that branch, and reading it anywhere else would offer a stage the base
/// cannot start.
///
/// Merged branches are skipped rather than read, and that is the whole of what
/// keeps this affordable: they hold nothing the default tip does not, so the
/// set is *what is in flight* rather than every branch the repository has ever
/// kept. In a working repository that is a handful out of a hundred.
///
/// The same reading twice over is drawn once — deduplicated on the branch the
/// stage would be worked on, which is the roadmap and the stage together. Two
/// branches offering genuinely different stages of one roadmap are two different
/// pieces of work off two different bases, and both are drawn.
///
/// **Which of two readings of one stage is kept is a question about the base**,
/// because the base is the only thing that differs between them and it is what
/// the press fixes the new Conversation to. A reading that is
/// [`behind`](Abandoned::behind) — one that had to count a settled stage as done
/// while its box was still unticked at that commit — is offering work whose
/// predecessor is not there, so it loses to a reading of the same stage that is
/// not behind. That is the case the record put back in this notice in the first
/// place: a roadmap whose stage 01 settled on an unmerged branch is offered at
/// the default tip *and* off that branch, and only one of the two holds stage
/// 01's commits. Where neither is behind, or both are, the default branch's is
/// kept: it is the base that needs no fixing.
///
/// No fetch, unlike those two. This is read for every registered Repo every
/// time the workbench reads the sidebar, and a network call per Repo per read is
/// not what a notice is worth: origin's copy as it last stood is enough to agree
/// with the page and the press, both of which freshen it themselves before they
/// act on it.
///
/// An empty list rather than the notice's "or nothing at all": the notice under
/// the new-conversation box is drawn only where there is something to say, and
/// what is waiting in a Repo is asked for whether or not there is any — empty
/// included, which is the ordinary answer.
///
/// `record` is what Verkstead knows about the stages of this Repo's roadmaps,
/// read once by whoever calls in: every roadmap here is judged against the same
/// value, which is what makes it one read per Repo rather than one per roadmap —
/// and this stays a reading, nothing in it asking the database.
///
/// Blocking, like everything else here: short git reads against a local
/// directory, and whoever calls it is on a borrowed thread already.
pub(crate) fn waiting(
    repo: &store::Repo,
    record: &store::StageStandings,
    at_once: usize,
) -> Vec<AbandonedRoadmap> {
    let named = worktrees::default_ref(&repo.path, &repo.default_branch);
    let Some(commit) = worktrees::resolve(&repo.path, &named) else {
        return Vec::new();
    };

    // The default branch first, so that where two readings of a stage are as good
    // as each other the one kept is the one needing no base fixed at all. Its base
    // is empty for that reason: there is nothing for the press to override.
    let bases = std::iter::once((String::new(), commit)).chain(unmerged(&repo.path, &named));

    let readings = bases
        .flat_map(|(base, commit)| {
            abandoned_at(&repo.path, &commit, record, at_once)
                .into_iter()
                .map(move |abandoned| (base.clone(), abandoned))
        })
        .collect::<Vec<_>>();

    // Which reading of each stage to draw, keyed on the stage's branch rather
    // than on the roadmap: it carries the roadmap and the stage both, and two
    // bases offering the same stage are offering the one piece of work.
    //
    // Read order decides a tie, the default branch's being first, and a reading
    // that is behind gives way to one that is not — which is the whole of the
    // preference, and which base each came off never enters into it.
    let mut kept: HashMap<String, usize> = HashMap::new();

    for (at, (_, abandoned)) in readings.iter().enumerate() {
        let better = match kept.get(&abandoned.stage.branch()) {
            None => true,
            Some(&held) => readings[held].1.behind && !abandoned.behind,
        };

        if better {
            kept.insert(abandoned.stage.branch(), at);
        }
    }

    // Back in read order, so the rows come out in the order the bases were read
    // however the preference above landed.
    let drawn = kept.into_values().collect::<BTreeSet<_>>();

    readings
        .into_iter()
        .enumerate()
        .filter(|(at, _)| drawn.contains(at))
        .map(|(_, (base, abandoned))| AbandonedRoadmap {
            name: abandoned.stage.roadmap.clone(),
            title: abandoned.title,
            stage: abandoned.stage.label.clone(),
            stage_title: abandoned.stage.title.clone(),
            // And the rest the press would start with it, so the row names what
            // pressing does rather than the lowest of what it does.
            beside: abandoned.beside.iter().map(named_stage).collect(),
            base,
        })
        .collect()
}

/// One stage named for a page: what the roadmap calls it, where its brief is and
/// the branch it would be worked on.
///
/// The notice's rows and the adopting Conversation's pane both want the same four
/// facts about a stage, and both have the whole [`Stage`] in hand when they draw
/// one — so the shape is filled once here rather than spelled at each of them.
fn named_stage(stage: &Stage) -> AdoptedStage {
    AdoptedStage {
        label: stage.label.clone(),
        title: stage.title.clone(),
        brief_path: stage.brief_path.clone(),
        branch: stage.branch(),
    }
}

/// Every local branch of `repo` whose commits `default` does not already hold,
/// with the commit each is at.
///
/// The branches worth reading a roadmap off, and the reason [`waiting`] can
/// afford to read any of them: a merged branch holds nothing the default tip
/// does not, so whatever it would offer is offered there already and read more
/// cheaply. What is left is the work in flight, which in a repository people
/// use is a handful of the branches it keeps.
///
/// The commit comes back with the name rather than being resolved again per
/// branch — git has it in hand while it is listing them, and this runs on the
/// sidebar's path.
///
/// `--no-merged=` written as one argument, because git takes what follows the
/// `=` as a value whatever it looks like: the default branch's name comes from
/// the Repo's own record, and a record is not a place to be relaxed about an
/// argument that could read as an option.
///
/// A repository git will not list says it has no branches, which is the answer
/// that changes nothing: the default branch's reading stands on its own.
fn unmerged(repo: &Path, default: &str) -> Vec<(String, String)> {
    let listed = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)\t%(objectname)",
            &format!("--no-merged={default}"),
            "refs/heads",
        ],
    );

    listed
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(branch, commit)| !branch.is_empty() && !commit.is_empty())
        .map(|(branch, commit)| (branch.to_owned(), commit.to_owned()))
        .collect()
}

/// The abandoned roadmaps `repo` holds at `commit`, in the order their
/// directories are named.
///
/// The whole of the abandoned rule, applied to every roadmap there: a
/// repository keeps the finished ones and may well be mid-flight on another, so
/// most of what is here on any given day comes back as nothing.
fn abandoned_at(
    repo: &Path,
    commit: &str,
    record: &store::StageStandings,
    at_once: usize,
) -> Vec<Abandoned> {
    names(repo, commit)
        .into_iter()
        .filter_map(|name| startable(repo, commit, &name, record, at_once).stage())
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
///
/// Shared with [`created`], which asks the same question of a Worktree: a brief
/// appearing is a stage being written down, and the index appearing is the
/// roadmap itself being written. Only the second says a branch created one.
fn indexed(path: &str) -> Option<&str> {
    let inside = path.trim().strip_prefix(ROADMAPS)?.strip_prefix('/')?;

    let (name, rest) = inside.split_once('/')?;

    (!name.is_empty() && rest == INDEX).then_some(name)
}

/// What the roadmap `name` at `commit` has to start, or which of the ways it
/// has nothing.
///
/// The four clauses of the abandoned rule, cheapest first: what the index and the
/// record say between them, then who is on it, then whether the brief is there,
/// then what git has for branches.
///
/// Asked at the default branch's tip for the notice, and at a Conversation's
/// base commit for the page that adopts and for the press itself — the same
/// rule each time, so what the human is offered is what pressing would start.
/// The notice and the page keep only the stage; the press is what says which
/// clause refused it, because the press is the one of the three with a human
/// waiting on an answer.
///
/// Which stages they are, is [`ready`]'s question: the ones whose every
/// dependency has settled and that are themselves neither done nor in flight. The
/// roadmap's order is still the roadmap's own, and an undeclared roadmap is read
/// as each stage standing on the one before it — so what this offers of one is
/// the lowest unticked box it always offered. There is no Conversation of this
/// reading's own to skip, so the branch the settling path hands [`ready`] is empty
/// here: a roadmap read here belongs to nobody yet.
///
/// **Every one of them the roadmap has a place for**, and not the lowest alone:
/// the press starts them all, so the notice and the page name them all. The
/// lowest is [`Abandoned::stage`], that being the one the Conversation doing the
/// pressing becomes, and the rest are [`Abandoned::beside`]. `at_once` is how many
/// stages of one roadmap run at once — [`crate::settings::AtOnce`], the same
/// setting the carry-on hands [`next_stage`] — and every stage the record has in
/// flight is already holding one of those places, so a roadmap whose places are
/// all taken has nothing to offer however many of its lines are ready.
///
/// `record` is what Verkstead knows about the stages of this Repo's roadmaps. A
/// value rather than a lookup, so this stays a reading — the rows are read once
/// per Repo by whoever calls in, and nothing in here asks the database.
///
/// Every branch reading is the fail-safe [`worktrees::branch_taken`] rather
/// than [`worktrees::branch_exists`]: what each of them stands in front of is
/// making a branch and letting an agent loose on it, so git failing to answer
/// is answered as *taken*.
pub(crate) fn startable(
    repo: &Path,
    commit: &str,
    name: &str,
    record: &store::StageStandings,
    at_once: usize,
) -> Startable {
    let Some(index) = at(repo, commit, &format!("{ROADMAPS}/{name}/{INDEX}")) else {
        return Startable::NoRoadmap;
    };

    // A directory under `docs/roadmaps/` whose index plans nothing is not a
    // roadmap at all, exactly as it is not one to pin.
    if index.lines().filter_map(checklist::entry).next().is_none() {
        return Startable::NoRoadmap;
    }

    // Clause 1: a stage that may start — [`ready`]'s answer, which is the one
    // reading the carry-on goes through too, so that what is offered here and what
    // an unattended run would start are one rule. Every stage done is a roadmap
    // that finished, and its directory stays where it is as the record of what it
    // was; nothing ready with a stage still in flight is neither that nor a stage
    // to offer.
    //
    // All of them, lowest first: the press starts every one there is a place for,
    // so this offers every one there is a place for.
    let (entry, rest, in_flight) = match ready(name, &index, record, "") {
        Ready::Stages {
            lowest,
            rest,
            in_flight,
        } => (lowest, rest, in_flight),
        Ready::Complete => return Startable::Complete,
        Ready::InFlight => return Startable::InFlight,
        // Nothing of a roadmap that declares badly may start, here as anywhere.
        // Naming the fault where the press can see it is a clause of its own, and
        // the press is where it is worth wording.
        Ready::Misdeclared(why) => return Startable::Misdeclared { why },
    };

    // And how many places it has left, the stages somebody is on having taken
    // theirs — see [`AT_ONCE`], where that rule is. A roadmap with none left has
    // nothing to offer whatever is ready: what it is waiting for is one of the
    // stages holding a place to settle, which is what [`Startable::InFlight`]
    // says.
    //
    // Saturating for [`next_stage`]'s reason: the record can hold more in flight
    // than the limit allows — the limit lowered in Settings, or a stage started by
    // hand — and a roadmap over its limit starts nothing rather than going
    // backwards.
    let places = at_once.saturating_sub(in_flight);

    if places == 0 {
        return Startable::InFlight;
    }

    // What was passed over on the way is worth one bit: a stage counted as done by
    // the record while its box is still unticked *here* is a stage whose finish
    // commit has not reached this commit, so neither has its work — see
    // [`Abandoned::behind`], which is what that is for.
    let behind = index
        .lines()
        .filter_map(checklist::entry)
        .take_while(|one| one.label != entry.label)
        .any(|one| !one.checked && done(&one, record.of(name, one.label)));

    // Clauses 2 to 4, on the stage the Conversation doing the pressing would
    // become: what is wrong with that one refuses the press by name, each of them
    // being a different thing for the human to go and do.
    let stage = match offered(repo, commit, name, &entry) {
        Ok(stage) => stage,
        Err(refusal) => return refusal,
    };

    // And the same three clauses on the rest, until there are as many as there are
    // places left over — the stage above has taken one of them. What refuses one of
    // these leaves it out rather than refusing the press: the human is being offered
    // work, and a sibling whose brief nobody has written yet is no reason to
    // withhold the stages that are ready beside it.
    //
    // Taken *after* the clauses rather than before them, for [`next_stage`]'s
    // reason: a place is spent by a stage that is offered and by nothing else, so
    // one left out leaves its place to the ready stage behind it. Taking first
    // would have the press start fewer stages than the roadmap is allowed, and the
    // lazy iterator is what keeps the clauses to one reading per stage offered.
    let beside = rest
        .into_iter()
        .filter_map(|entry| offered(repo, commit, name, &entry).ok())
        .take(places - 1)
        .collect();

    Startable::Stage(Box::new(Abandoned {
        title: checklist::heading(&index),
        stage,
        beside,
        behind,
    }))
}

/// One ready stage of a roadmap as something to start, or which of the three
/// ways it is nothing to offer.
///
/// The clauses after [`ready`]'s, spent once on every stage the adoption would
/// start — the one the pressing Conversation becomes and each one started beside
/// it. Kept together so that a stage offered beside another is offered on exactly
/// the terms the first was: a brief that is there, a branch nobody holds, and
/// nobody's annotation on the line.
///
/// - **Clause 3: nobody on it.** The record saying so outright is [`ready`]'s
///   business, a stage it says is in flight being no stage to start. **What is
///   left here is the annotation**, which is where an abandoned stage goes with a
///   stage the record has no row for at all: the record saying a Conversation was
///   closed part-way through says nothing about whether anybody is on the stage
///   *now*, and somebody carrying that work on by hand says so where the score is
///   kept — the one way an unwanted row is silenced in the repository, the other
///   being the box. Prose a human may have rewritten, so the branch inside the
///   backticks is the fact, and one whose branch is gone is a note about an
///   attempt that was abandoned too.
/// - **Clause 2: something to start it from.** An entry pointing at a file nobody
///   wrote is the human's to fix, and nothing is offered until they have —
///   starting the stage after it instead would be Verkstead deciding to skip work.
/// - **Clause 4: its branch is free.** Which is also what keeps a stage already in
///   flight under Verkstead out of the list — its branch is in this git directory
///   from the moment the stage started, long before the finish commit that ticks
///   its box reaches the default branch. Both names, because a stage started
///   before the scheme changed is on the former one and that branch is the only
///   thing saying so. Permanently rather than for a while: it is one more ref
///   lookup, and it never lies. And nothing standing where the path of that name
///   goes, which is a branch git would refuse to make rather than one somebody is
///   already on.
fn offered(
    repo: &Path,
    commit: &str,
    roadmap: &str,
    entry: &checklist::Entry<'_>,
) -> Result<Stage, Startable> {
    if annotating(entry.after).is_some_and(|branch| worktrees::branch_taken(repo, branch)) {
        return Err(Startable::InFlight);
    }

    if entry.link.is_empty() {
        return Err(Startable::NoBrief);
    }

    let brief_path = format!("{ROADMAPS}/{roadmap}/{}", entry.link);

    let Some(brief) = at(repo, commit, &brief_path) else {
        return Err(Startable::NoBrief);
    };

    let stage = Stage {
        roadmap: roadmap.to_owned(),
        label: entry.label.to_owned(),
        title: entry.title.to_owned(),
        brief_path,
        brief,
    };

    let branch = stage.branch();
    let former = stage.former_branch();

    if worktrees::branch_taken(repo, &branch) || worktrees::branch_taken(repo, &former) {
        return Err(Startable::BranchTaken);
    }

    if let Some(by) = in_the_way(repo, &branch) {
        return Err(Startable::BranchInTheWay { by });
    }

    Ok(stage)
}

/// How long [`adopting`] gives its fetch before git is stopped.
///
/// Long enough for a slow remote on a slow connection, and short enough that a
/// human waiting on a page has not yet decided it is broken. The Conversation
/// pane is read behind it, so this is the longest that page can take to say
/// anything at all.
const FETCHING: std::time::Duration = std::time::Duration::from_secs(10);

/// What an adopting Conversation's page says about the roadmap it was started
/// for: the roadmap named, and the stage adopting would start.
///
/// Read at the base commit the Conversation branches from — the override where
/// the human typed one, and origin's tip of the default branch where they did
/// not, fetched for before it is read — and
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
///
/// And the fetch inside is given [`FETCHING`] and no longer, because this is
/// the one read on the Conversation pane's own path that can wait on a network.
/// Everything else the pane reads is the database or the filesystem; this is a
/// `git fetch`, and a route that drops packets rather than refusing them leaves
/// one sitting there for good. A pane that never resolves is the worst of all
/// the answers — the human cannot even reach the menu to close the
/// Conversation — so past the deadline git is stopped and the page is drawn off
/// what was last fetched, exactly as it is for a fetch git itself refused.
pub(crate) async fn adopting(
    pool: &SqlitePool,
    repo: store::Repo,
    base: Option<String>,
    roadmap: String,
    at_once: usize,
) -> AdoptionView {
    // The roadmap is the one thing here that was never the repository's to say,
    // so it is what the page is drawn with whatever the reading comes back as.
    let named = roadmap.clone();

    // And what Verkstead's record says about this Repo's stages, which is half of
    // what says a stage is done — see [`done`]. Read here rather than inside the
    // reading, so that the reading stays a reading; and rows that will not come
    // back leave the page naming no stage, for the reason the notice says nothing
    // for such a Repo: what the boxes alone would offer is a stage that settled.
    let record = match store::stage_standings(pool, repo.id).await {
        Ok(record) => record,
        Err(error) => {
            tracing::error!(
                error = ?error,
                repo_id = repo.id,
                "reading what a Repo's roadmap stages have got to failed",
            );

            return AdoptionView {
                roadmap: named,
                title: String::new(),
                stage: None,
                beside: Vec::new(),
            };
        }
    };

    let read = tokio::task::spawn_blocking(move || {
        let commit = match base {
            // The override resolves exactly as the human fixed it.
            Some(base) => base,

            // Without one it is the default branch as origin holds it, made
            // current first: a page drawn off a week-old copy of `main` names
            // the stage that was next a week ago. A fetch git refused is a line
            // in the log and no more — what this draws is a reading, and the
            // reading off what was last fetched says more than a blank page
            // does. The press is where a failed fetch refuses, because the press
            // is what would act on it.
            //
            // Which is why the deadline can be here and cannot be there: a
            // fetch stopped for running long is a failed fetch, and this path
            // already knows what to do with one. What it buys is that the pane
            // behind it always answers.
            None => {
                if let worktrees::Fetched::Failed(said) =
                    worktrees::fetch_within(&repo.path, FETCHING)
                {
                    tracing::warn!(
                        said,
                        repo = %repo.path.display(),
                        "fetching a Repo's remotes failed, so its adoption page is drawn off \
                         what was last fetched",
                    );
                }

                worktrees::default_ref(&repo.path, &repo.default_branch)
            }
        };

        let found = worktrees::resolve(&repo.path, &commit)
            .and_then(|commit| startable(&repo.path, &commit, &roadmap, &record, at_once).stage());

        let Some(abandoned) = found else {
            return AdoptionView {
                roadmap,
                title: String::new(),
                stage: None,
                beside: Vec::new(),
            };
        };

        AdoptionView {
            roadmap,
            title: abandoned.title,
            stage: Some(named_stage(&abandoned.stage)),
            // And every stage the press would start beside it, so what the pane
            // names is what pressing does.
            beside: abandoned.beside.iter().map(named_stage).collect(),
        }
    })
    .await;

    read.unwrap_or_else(|error| {
        tracing::error!(error = ?error, "reading the roadmap a Conversation is adopting failed");

        AdoptionView {
            roadmap: named,
            title: String::new(),
            stage: None,
            beside: Vec::new(),
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

    /// The one stage a reading starts, where the test expects exactly one — and
    /// `said`, which is what that test is about, where it started anything else.
    ///
    /// Most of these are about *which* stage a roadmap starts rather than how
    /// many, and every one of those roadmaps has one ready stage at a time: they
    /// run in order, or the record says everything above the stage they are about
    /// stands on something in flight. So the list is unwrapped here, once, and a
    /// roadmap that suddenly starts two fails the test that says it starts one.
    #[track_caller]
    fn only(next: Next, said: &str) -> Stage {
        let Next::Stages { mut starting, held } = next else {
            panic!("{said} — and the reading started nothing at all: {next:?}");
        };

        assert!(
            starting.len() == 1 && held.is_empty(),
            "{said} — and the reading answered {starting:?}, holding {held:?}",
        );

        starting.remove(0)
    }

    /// What a reading holds back, which is a sentence per ready stage that does
    /// not start: one waiting for a place on its roadmap, one waiting for a place
    /// on the server, one whose brief is not there.
    ///
    /// The sentences alone, which is what most of these ask about. See
    /// [`waits_on`] for the tests that are about which of the three each one is.
    #[track_caller]
    fn holding(next: Next) -> Vec<String> {
        waits_on(next)
            .iter()
            .map(|held| held.said().to_owned())
            .collect()
    }

    /// And the same with what each of them is waiting on kept — see [`Held`],
    /// which is what a **look** says one of and keeps the other two off a
    /// Timeline.
    #[track_caller]
    fn waits_on(next: Next) -> Vec<Held> {
        match next {
            Next::Stages { held, .. } => held,
            other => panic!("nothing of this roadmap was read as ready: {other:?}"),
        }
    }

    /// And which stages it starts, by label — for the tests that are about how
    /// many start rather than about one of them.
    #[track_caller]
    fn starting(next: &Next) -> Vec<&str> {
        match next {
            Next::Stages { starting, .. } => {
                starting.iter().map(|stage| stage.label.as_str()).collect()
            }
            other => panic!("nothing of this roadmap was read as ready: {other:?}"),
        }
    }

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

        /// What the roadmap `roadmap` has left to start, as the Conversation on
        /// `branch` finishing asks it.
        ///
        /// The roadmap is named rather than worked out, which is the whole of
        /// the change: the record says which one this Conversation is a stage
        /// of, and nothing about the branch decides it.
        ///
        /// With a record holding nothing, which is every roadmap worked before
        /// Verkstead wrote one — so what these answer with is the boxes and the
        /// annotation alone, exactly as they answered before there was a record.
        fn next(&self, roadmap: &str, branch: &str) -> Next {
            self.next_with(roadmap, branch, &store::StageStandings::default())
        }

        /// And the same reading with a record behind it: what Verkstead knows
        /// about the stages of this Repo's roadmaps, which is what decides whether
        /// a stage is done wherever it says anything at all.
        ///
        /// Committed first, because the reading is off a commit rather than off a
        /// checkout: what a session has written and not committed is not the
        /// roadmap yet, and the branch the declarations are read off may be one
        /// nothing is checked out of at all.
        fn next_with(&self, roadmap: &str, branch: &str, record: &store::StageStandings) -> Next {
            let commit = self.committed();

            self.next_at("main", &commit, roadmap, branch, record)
        }

        /// And the same reading at a named branch of this repository, which is
        /// what the carry-on actually asks: the roadmap at the top of its chain
        /// rather than wherever the branch that settled left it.
        fn next_at(
            &self,
            at: &str,
            commit: &str,
            roadmap: &str,
            branch: &str,
            record: &store::StageStandings,
        ) -> Next {
            self.next_running(at, commit, roadmap, branch, record, AT_ONCE)
        }

        /// And the same reading with a limit of the caller's choosing, for the
        /// tests that are about the limit rather than about which stages are
        /// ready. Every other reading here runs at [`AT_ONCE`], which is what the
        /// carry-on passes where nobody has set the number in Settings.
        ///
        /// On an **empty server**: every place there is stands free, which is the
        /// reading every test but the ones about the second limit wants. See
        /// [`Repo::next_on_a_server`], which is those.
        fn next_running(
            &self,
            at: &str,
            commit: &str,
            roadmap: &str,
            branch: &str,
            record: &store::StageStandings,
            at_once: usize,
        ) -> Next {
            self.next_with_places(
                at,
                commit,
                roadmap,
                branch,
                record,
                at_once,
                CONVERSATIONS_AT_ONCE,
            )
        }

        /// And the same reading with both limits the caller's, which is what the
        /// carry-on hands in: how many stages of this roadmap run at once, and how
        /// many places the whole server has **left over** once everything already
        /// running has taken its own.
        #[allow(clippy::too_many_arguments)]
        fn next_with_places(
            &self,
            at: &str,
            commit: &str,
            roadmap: &str,
            branch: &str,
            record: &store::StageStandings,
            at_once: usize,
            server_places: usize,
        ) -> Next {
            next_stage(
                self.path(),
                Declaring { branch: at, commit },
                roadmap,
                branch,
                record,
                at_once,
                server_places,
            )
        }

        /// What this roadmap would start with `server_places` free across the
        /// whole server, its own limit being whatever is passed.
        ///
        /// The second limit's own harness — see [`CONVERSATIONS_AT_ONCE`]. A
        /// roadmap's places and the server's are spent together, so both numbers
        /// are the caller's here: what these tests are about is which of the two
        /// is the one that bites.
        fn next_on_a_server(
            &self,
            roadmap: &str,
            record: &store::StageStandings,
            at_once: usize,
            server_places: usize,
        ) -> Next {
            let commit = self.committed();

            self.next_with_places("main", &commit, roadmap, "", record, at_once, server_places)
        }

        /// What this roadmap would start with only `at_once` places to give, the
        /// record behind it as it stands.
        fn next_limited(
            &self,
            roadmap: &str,
            branch: &str,
            record: &store::StageStandings,
            at_once: usize,
        ) -> Next {
            let commit = self.committed();

            self.next_running("main", &commit, roadmap, branch, record, at_once)
        }

        /// Everything written so far committed to the branch checked out, and the
        /// commit that made.
        ///
        /// `--allow-empty`, so that a test asking the same reading twice is not a
        /// commit git refuses the second time.
        fn committed(&self) -> String {
            run(self.path(), &["add", "-A"]);
            run(
                self.path(),
                &[
                    "commit",
                    "--allow-empty",
                    "-m",
                    "docs: the roadmap as it stands",
                ],
            );

            run(self.path(), &["rev-parse", "HEAD"]).trim().to_owned()
        }

        /// And the commit a named branch of it is at, which is what a reading off
        /// that branch is read at.
        fn commit_of(&self, branch: &str) -> String {
            run(
                self.path(),
                &["rev-parse", "--verify", "--end-of-options", branch],
            )
            .trim()
            .to_owned()
        }

        /// And which roadmaps this branch created since the base commit, which
        /// is what says a roadmap Conversation chose one.
        fn created(&self) -> BTreeSet<String> {
            created(self.path(), &self.base)
        }

        /// And which it has written to at all, which is the wider question the
        /// pinned stage list is drawn from.
        fn touched(&self) -> BTreeSet<String> {
            touched(self.path(), &self.base)
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

        /// The abandoned roadmaps this repository holds there, with a record
        /// holding nothing — which is every roadmap worked before Verkstead kept
        /// one, so what these answer with is the boxes, the annotation and the
        /// branches alone.
        fn abandoned(&self) -> Vec<Abandoned> {
            self.abandoned_with(&store::StageStandings::default())
        }

        /// And the same reading with a record behind it: what Verkstead knows
        /// about the stages of this Repo's roadmaps, which is what says a stage is
        /// done wherever it says anything at all.
        fn abandoned_with(&self, record: &store::StageStandings) -> Vec<Abandoned> {
            abandoned_at(self.path(), &self.tip(), record, AT_ONCE)
        }

        /// And what one roadmap of it comes back as, which is the same reading
        /// with its refusals kept: what a notice throws away, the press says
        /// out loud.
        fn startable(&self, name: &str) -> Startable {
            self.startable_with(name, &store::StageStandings::default())
        }

        /// With a record behind it, which is the half of the answer the
        /// repository does not hold.
        fn startable_with(&self, name: &str, record: &store::StageStandings) -> Startable {
            self.startable_running(name, record, AT_ONCE)
        }

        /// And with a limit of its own, which is the other half of what the
        /// adoption offers: every ready stage there is a *place* for, and the
        /// places a roadmap has are a setting.
        fn startable_running(
            &self,
            name: &str,
            record: &store::StageStandings,
            at_once: usize,
        ) -> Startable {
            startable(self.path(), &self.tip(), name, record, at_once)
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
        /// A stage's own commits: the annotation its plan commit writes and the
        /// tick its finish commit writes both ride on the stage's own branch, and
        /// reach the default branch only when its pull request merges.
        fn commit_on(&self, branch: &str, message: &str) {
            run(self.path(), &["checkout", "-q", "-b", branch]);
            run(self.path(), &["add", "-A"]);
            run(self.path(), &["commit", "-m", message]);
            run(self.path(), &["checkout", "-q", "main"]);
        }

        /// Everything this Repo is holding with a stage that could start — at the
        /// default branch's tip and on every branch the default has not swallowed
        /// — with a record holding nothing, for [`Repo::abandoned`]'s reason.
        fn waiting(&self) -> Vec<AbandonedRoadmap> {
            waiting(
                &self.registered(),
                &store::StageStandings::default(),
                AT_ONCE,
            )
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
        /// The stage lists this worktree comes back with, with a record holding
        /// nothing — which is a roadmap worked by hand or by the old tools, read
        /// off its boxes exactly as it always was.
        fn lists(&self) -> Vec<StageListEvent> {
            self.lists_with(&store::StageStandings::default())
        }

        /// And with a record behind them, which is where a stage's state comes
        /// from wherever it has a row — this server holding no stage, which is
        /// what a server that has just come back is.
        fn lists_with(&self, record: &store::StageStandings) -> Vec<StageListEvent> {
            self.lists_holding(record, &HashSet::new())
        }

        /// And with the joins register beside it: the Conversations this server is
        /// holding before their finish, which is where *waiting to join* comes from.
        fn lists_holding(
            &self,
            record: &store::StageStandings,
            held: &HashSet<i64>,
        ) -> Vec<StageListEvent> {
            roadmaps(self.path(), &self.base, record, held)
        }

        /// One of them opened, which is the same reading a level deeper: the
        /// briefs the stages name rather than the lines beside them.
        fn opened(&self, name: &str) -> Option<RoadmapPane> {
            self.opened_with(name, &store::StageStandings::default())
        }

        /// And that pane against a record, as the card above is.
        fn opened_with(&self, name: &str, record: &store::StageStandings) -> Option<RoadmapPane> {
            self.opened_holding(name, record, &HashSet::new())
        }

        /// And against the register too, which the pane reads exactly as the card
        /// does.
        fn opened_holding(
            &self,
            name: &str,
            record: &store::StageStandings,
            held: &HashSet<i64>,
        ) -> Option<RoadmapPane> {
            opened(self.path(), &self.base, name, record, held)
        }
    }

    fn write(worktree: &Path, name: &str, index: &str) {
        let directory = worktree.join(ROADMAPS).join(name);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(INDEX), index).unwrap();
    }

    /// One Repo's notice, with a record holding nothing — [`Repo::abandoned`]'s
    /// reason, one level up.
    fn notice_of(repo: &store::Repo) -> Option<AbandonedRepo> {
        notice(repo, &store::StageStandings::default(), AT_ONCE)
    }

    /// A store of its own, with nothing whatever in it, for the two readings that
    /// ask the record for themselves.
    ///
    /// An empty database is a Repo whose stages Verkstead has never recorded
    /// anything about, which is what a roadmap worked by hand or by the old tools
    /// answers to: the boxes, the annotation and the branches are the whole of the
    /// answer, exactly as they were.
    async fn empty_store(dir: &tempfile::TempDir) -> SqlitePool {
        store::open_database(&dir.path().join("verkstead.db"))
            .await
            .expect("a database in a fresh directory should open")
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
                .map(|stage| stage.state.clone())
                .collect::<Vec<_>>(),
            [StageState::Done, StageState::Done, StageState::ToDo],
            "the record holds nothing about this roadmap, so its boxes are the \
             whole of what says where each stage is",
        );
    }

    /// A roadmap of four, one line per state there is: the record decides wherever
    /// it says anything about a stage, and the box decides wherever it says
    /// nothing.
    ///
    /// The boxes here are deliberately the wrong way round from the record —
    /// stage 01 settled and is unticked, stage 04 is ticked and nothing has ever
    /// been started on it — because that is what a branch cut before its
    /// neighbours' finish commits actually holds, and reading it off the boxes is
    /// what the record is here to stop.
    const FOUR: &str = "\
# MVP roadmap

- [ ] 01: Workbench — [brief](01-workbench.md)
- [ ] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md)
- [x] 04: Wrap-up — [brief](04-wrap-up.md)
";

    /// Where the states come from: the record where it has a row, the box where it
    /// has none.
    #[test]
    fn the_record_says_where_each_stage_is_and_the_boxes_say_where_it_has_none() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        assert_eq!(
            states(&repo.lists_with(&record([
                ("mvp", "01", store::StageStanding::Settled),
                ("mvp", "02", store::StageStanding::InFlight),
                ("mvp", "03", store::StageStanding::Abandoned),
            ]))),
            [
                StageState::Done,
                StageState::InProgress,
                StageState::Halted,
                StageState::Done,
            ],
            "01 settled however its box reads here, 02 is somebody's now, 03 was \
             walked away from, and 04 has only its ticked box to speak for it",
        );
    }

    /// And a stage in flight whose Conversation has **stopped** is halted rather
    /// than in progress: the same word an abandoned stage gets, because what a
    /// reader does about either is go and look at that Conversation.
    #[test]
    fn a_stage_whose_conversation_has_stopped_reads_halted_rather_than_in_progress() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        let in_flight = [("mvp", "01", store::StageStanding::InFlight)];

        assert_eq!(
            states(&repo.lists_with(&record(in_flight)))[0],
            StageState::InProgress,
            "nothing has stopped it, so somebody is on it",
        );
        assert_eq!(
            states(&repo.lists_with(&halted(in_flight)))[0],
            StageState::Halted,
            "and a run that has stopped is not in progress, whatever else it is",
        );
    }

    /// Stages of `mvp` the record has **in flight**, as the Conversations named.
    ///
    /// Which Conversation a stage is, is what the joins register is asked by — it
    /// keeps Conversations where a roadmap keeps labels — so the tests about the
    /// hold name one where [`record`] has no need to.
    fn on_it<'a>(rows: impl IntoIterator<Item = (&'a str, i64)>) -> store::StageStandings {
        store::StageStandings::from_rows(rows.into_iter().map(|(label, conversation_id)| {
            (
                "mvp",
                label,
                store::StageStanding::InFlight,
                false,
                conversation_id,
            )
        }))
    }

    /// And the same with those Conversations **stopped**, which is the one thing a
    /// hold loses to.
    fn halted_on_it<'a>(rows: impl IntoIterator<Item = (&'a str, i64)>) -> store::StageStandings {
        store::StageStandings::from_rows(rows.into_iter().map(|(label, conversation_id)| {
            (
                "mvp",
                label,
                store::StageStanding::InFlight,
                true,
                conversation_id,
            )
        }))
    }

    /// A stage this server is **holding** before its finish reads *waiting to join*
    /// where the record alone would have said *in progress*.
    ///
    /// Every task of it is done and what waits is the join, so the record has it in
    /// flight and the hold is the more particular thing to say about it — see
    /// [`holding`], and [`crate::joins`], which is the hold itself.
    ///
    /// And only that stage: the register is asked by the Conversation the record
    /// names for the label, so a second stage in flight under a Conversation nothing
    /// is holding goes on reading *in progress*.
    #[test]
    fn a_stage_held_before_its_finish_reads_waiting_to_join() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        let record = on_it([("01", 7), ("02", 9)]);

        assert_eq!(
            states(&repo.lists_holding(&record, &HashSet::from([9]))),
            [
                StageState::InProgress,
                StageState::WaitingToJoin,
                StageState::ToDo,
                StageState::Done,
            ],
            "Conversation 9 is the one being held, and stage 02 is the stage it is: the \
             stage in flight beside it is nobody's hold",
        );
    }

    /// And a server **holding nothing** reads *in progress* for the same stage, which
    /// is exactly what the sidebar's own label does.
    ///
    /// The register is a task of this process rather than anything stored, so a
    /// server that has just come back is holding nothing at all: such a stage reads
    /// *in progress* again until the resume takes it up and finds it held a second
    /// time. The two agreeing is the point of them being one register.
    #[test]
    fn a_server_holding_nothing_reads_the_same_stage_in_progress() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        let record = on_it([("02", 9)]);

        assert_eq!(
            states(&repo.lists_holding(&record, &HashSet::new()))[1],
            StageState::InProgress,
            "nothing is held, so nothing is waiting to join",
        );
        assert_eq!(
            states(&repo.lists_with(&record))[1],
            StageState::InProgress,
            "which is what a reading handed no register at all says too",
        );
    }

    /// And a held stage whose Conversation has **stopped** reads *halted*: a
    /// Conversation that has stopped is being held by nothing, whatever a register
    /// this process has not swept says.
    #[test]
    fn a_held_stage_whose_conversation_has_stopped_reads_halted() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        assert_eq!(
            states(&repo.lists_holding(&halted_on_it([("02", 9)]), &HashSet::from([9])))[1],
            StageState::Halted,
            "halted wins: what a reader does about it is go and look at that \
             Conversation",
        );
    }

    /// And the pane says *waiting to join* exactly where the card does, off the one
    /// register: a pane saying a stage was in progress while the card that opened it
    /// said the stage was waiting to join would be two readings of one hold.
    #[test]
    fn the_pane_says_a_stage_is_waiting_to_join_exactly_as_the_card_does() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        let record = on_it([("01", 7), ("02", 9)]);
        let held = HashSet::from([9]);

        let pane = repo
            .opened_holding("mvp", &record, &held)
            .expect("there is a roadmap to open");

        assert_eq!(
            pane.stages
                .iter()
                .map(|stage| stage.state.clone())
                .collect::<Vec<_>>(),
            states(&repo.lists_holding(&record, &held)),
        );
        assert_eq!(pane.stages[1].state, StageState::WaitingToJoin);
    }

    /// The pane says the same thing about a stage as the card does, off the same
    /// record: they are one Conversation's two views of one roadmap, and a pane
    /// that disagreed with the card that opened it would be two readings.
    #[test]
    fn the_pane_says_where_each_stage_is_exactly_as_the_card_does() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR);

        let record = record([
            ("mvp", "01", store::StageStanding::Settled),
            ("mvp", "02", store::StageStanding::InFlight),
            ("mvp", "03", store::StageStanding::Abandoned),
        ]);

        let pane = repo
            .opened_with("mvp", &record)
            .expect("there is a roadmap to open");

        assert_eq!(
            pane.stages
                .iter()
                .map(|stage| stage.state.clone())
                .collect::<Vec<_>>(),
            states(&repo.lists_with(&record)),
        );
    }

    /// And the card says the same thing about a stage on the roadmap's own
    /// Timeline as on that stage's, which is what reading the record rather than
    /// the boxes buys.
    ///
    /// Two Worktrees of one Repo, holding `ROADMAP.md` as each of them last saw
    /// it: the roadmap's own branch was cut before any stage finished and has
    /// nothing ticked, and stage 02's branch ticked itself off at its own finish.
    /// The boxes disagree; the record does not, so the cards do not either.
    #[test]
    fn the_card_says_the_same_thing_on_the_roadmaps_timeline_as_on_a_stages() {
        let planner = Repo::with(&[]);
        planner.write("mvp", FOUR);

        let stage = Repo::with(&[]);
        stage.write("mvp", &FOUR.replace("- [ ] 02", "- [x] 02"));

        let record = record([
            ("mvp", "01", store::StageStanding::Settled),
            ("mvp", "02", store::StageStanding::InFlight),
        ]);

        assert_eq!(
            states(&planner.lists_with(&record)),
            states(&stage.lists_with(&record)),
            "the tick stage 02 wrote on its own branch says its tasks are done \
             rather than that it settled, and the record is what knows the \
             difference",
        );
    }

    /// The states of one reading's one roadmap, in the roadmap's own order.
    #[track_caller]
    fn states(lists: &[StageListEvent]) -> Vec<StageState> {
        let [list] = lists else {
            panic!("this reading should have come back with one roadmap: {lists:?}");
        };

        list.stages
            .iter()
            .map(|stage| stage.state.clone())
            .collect()
    }

    /// One *waiting on*, by the labels it names.
    #[track_caller]
    fn behind(stages: &[&str]) -> StageState {
        StageState::WaitingOn {
            stages: stages.iter().map(|named| (*named).to_owned()).collect(),
        }
    }

    /// A stage of a **declaring** roadmap that has not started says which stages it
    /// is standing behind, in place of the *to do* that says nothing.
    ///
    /// [`DECLARED`] with a record holding nothing, which is the roadmap as it reads
    /// the moment it lands: 01 and 02 stand on nothing and are the two that could
    /// start, 03 stands on 02, and 04 stands on 01 and 03 both — so the card says
    /// what each of the other two is behind rather than lining all four up as work
    /// to do.
    #[test]
    fn a_stage_of_a_declaring_roadmap_says_which_stages_it_is_waiting_on() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);

        assert_eq!(
            states(&repo.lists()),
            [
                StageState::ToDo,
                StageState::ToDo,
                behind(&["02"]),
                behind(&["01", "03"]),
            ],
            "a root has nothing to wait on, and the two that stand on stages say \
             which ones",
        );
    }

    /// And the pane says it too, off the same reading: the section's heading and the
    /// contents line beside it are the card's row one level deeper.
    #[test]
    fn the_pane_says_what_a_stage_is_waiting_on_exactly_as_the_card_does() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);

        let pane = repo.opened("mvp").expect("there is a roadmap to open");

        assert_eq!(
            pane.stages
                .iter()
                .map(|stage| stage.state.clone())
                .collect::<Vec<_>>(),
            states(&repo.lists()),
        );
    }

    /// Only the ones that have **not settled**, which is what makes this a state
    /// rather than the declaration said again: what a stage stands on does not move
    /// as the roadmap runs, and what it is still behind does.
    ///
    /// Stage 04 stands on 01 and 03 throughout. With 01 settled it is behind 03
    /// alone; with both settled it is behind nothing and reads *to do* — which is a
    /// stage waiting for a place or halted for itself rather than for a neighbour.
    #[test]
    fn it_names_only_the_stages_that_have_not_settled_yet() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);

        let settled = |labels: &[&str]| {
            record(
                labels
                    .iter()
                    .map(|label| ("mvp", *label, store::StageStanding::Settled))
                    .collect::<Vec<_>>(),
            )
        };

        assert_eq!(
            states(&repo.lists_with(&settled(&["01"])))[3],
            behind(&["03"]),
            "01 has settled, so what 04 is behind is 03 alone",
        );

        assert_eq!(
            states(&repo.lists_with(&settled(&["01", "02", "03"])))[3],
            StageState::ToDo,
            "and with both of them settled 04 is waiting on no stage at all",
        );
    }

    /// A stage the record has something to say about is where the record says it is,
    /// whatever its line stands on: *waiting on* replaces *to do* rather than
    /// standing in front of the states that come off the record.
    #[test]
    fn a_stage_the_record_speaks_for_is_not_waiting_on_anything() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);

        assert_eq!(
            states(&repo.lists_with(&record([
                ("mvp", "03", store::StageStanding::InFlight),
                ("mvp", "04", store::StageStanding::Abandoned),
            ]))),
            [
                StageState::ToDo,
                StageState::ToDo,
                StageState::InProgress,
                StageState::Halted,
            ],
            "03 stands on 02 and somebody is on it anyway, and 04 was walked away \
             from",
        );
    }

    /// And an **undeclared** roadmap's unstarted stages go on reading *to do*.
    ///
    /// Such a roadmap is scheduled as each stage standing on the one before it — see
    /// [`ready`] — but that is the scheduler's reading of silence rather than
    /// anything the roadmap says, and *waiting on 01* about a line that declares
    /// nothing would be Verkstead putting a declaration in the human's mouth.
    #[test]
    fn every_unstarted_stage_of_an_undeclared_roadmap_still_reads_to_do() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);

        assert_eq!(
            states(&repo.lists()),
            [StageState::ToDo, StageState::ToDo, StageState::ToDo],
            "nothing here declares anything, so nothing here is waiting on \
             anything either",
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
        assert_eq!(repo.lists()[0].stages[2].state, StageState::Done);
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

    /// The card says which stages there are; the pane says what each of them is
    /// for. Both are one reading of the roadmap, so the entries line up.
    #[test]
    fn the_pane_holds_one_section_per_stage_in_the_roadmaps_own_order() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.brief("mvp", "01-workbench.md", "# 1. Workbench\n");

        let pane = repo.opened("mvp").expect("there is a roadmap to open");

        assert_eq!(pane.name, "mvp");
        assert_eq!(pane.title, "MVP roadmap");
        assert_eq!(
            pane.stages
                .iter()
                .map(|stage| (
                    stage.number.as_str(),
                    stage.title.as_str(),
                    stage.state.clone()
                ))
                .collect::<Vec<_>>(),
            [
                ("01", "Workbench", StageState::Done),
                ("02", "Grilling", StageState::Done),
                ("03", "Implementation", StageState::ToDo),
            ]
        );
    }

    /// The one place this parts company with a backlog: a stage's brief stays
    /// where it is for ever, so a done stage has a document like any other and
    /// the box beside it is the whole of what says it is finished.
    #[test]
    fn a_done_stage_carries_its_brief_all_the_same() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.brief(
            "mvp",
            "01-workbench.md",
            "# 1. Workbench\n\nThree panes, read from `docs/design/`.\n",
        );

        let pane = repo.opened("mvp").unwrap();
        let html = pane.stages[0].html.as_deref().expect("that file is there");

        assert_eq!(pane.stages[0].state, StageState::Done);
        assert!(
            html.contains("<h1>1. Workbench</h1>"),
            "rendered as markdown, like every other document on this wire: {html}",
        );
        assert!(html.contains("<code>docs/design/</code>"), "{html}");
    }

    /// A brief is found by the number its name leads with rather than by the
    /// link the entry carries — the number is what the roadmap turns on, and a
    /// link is a string out of a file in a repository.
    #[test]
    fn a_brief_is_found_by_its_number_rather_than_by_the_link() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n- [ ] 01: Workbench — [brief](../../../escape.md)\n",
        );
        repo.brief("mvp", "01-workbench.md", "# 1. Workbench\n");

        let pane = repo.opened("mvp").unwrap();

        assert!(
            pane.stages[0]
                .html
                .as_deref()
                .unwrap()
                .contains("<h1>1. Workbench</h1>"),
            "the file beside the index, whatever the link says",
        );
    }

    /// A stage the roadmap names a brief for that nobody wrote. Nothing to draw,
    /// and the pane says so in words rather than leaving a gap.
    #[test]
    fn a_stage_whose_brief_is_not_there_has_nothing_to_draw() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.brief("mvp", "01-workbench.md", "\n   \n");

        let pane = repo.opened("mvp").unwrap();

        assert_eq!(
            pane.stages[0].html, None,
            "a file left behind with nothing in it is the same as no file",
        );
        assert_eq!(pane.stages[2].html, None, "and one that was never written");
    }

    /// What each line declares, drawn only where the roadmap declares at all.
    ///
    /// A roadmap written before there was anything to declare has prose after
    /// the link — `— landed, and replaced by 04` is in this repository — and a
    /// word following an `on` in prose is not a platform somebody typed wrong.
    /// So the file is judged before any line of it is read for a declaration,
    /// and such a roadmap is drawn exactly as it always was.
    #[test]
    fn an_undeclared_roadmaps_prose_is_not_read_as_a_declaration() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md) — waits on the API landing\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n",
        );

        let pane = repo.opened("mvp").unwrap();

        assert!(
            pane.stages
                .iter()
                .all(|stage| stage.stands_on.is_none() && stage.platform.is_none()),
            "not one line of it declares, so none of it is read as one: {:?}",
            pane.stages,
        );

        // And a roadmap that declares badly still shows what its lines say: the
        // refusal is what names the fault, and the pane is where the human goes
        // to see the line that holds it.
        let declaring = Repo::with(&[]);
        declaring.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) — after 01 — on freebsd\n",
        );

        let pane = declaring.opened("mvp").unwrap();

        assert_eq!(pane.stages[0].stands_on, Some(Vec::new()));
        assert_eq!(pane.stages[1].platform.as_deref(), Some("freebsd"));
    }

    /// The renderer in the page is loaded for the pane rather than for a stage,
    /// so the flag is asked of all of them at once.
    #[test]
    fn a_diagram_in_any_stage_brief_is_what_the_pane_draws_with() {
        let plain = Repo::with(&[]);
        plain.write("mvp", MVP);
        plain.brief("mvp", "01-workbench.md", "Just words.\n");

        assert!(!plain.opened("mvp").unwrap().diagrams);

        let drawn = Repo::with(&[]);
        drawn.write("mvp", MVP);
        drawn.brief(
            "mvp",
            "03-implementation.md",
            "```mermaid\nflowchart LR\n  in --> out\n```\n",
        );
        let pane = drawn.opened("mvp").unwrap();

        assert!(pane.diagrams);
        assert!(
            pane.stages[2]
                .html
                .as_deref()
                .unwrap()
                .contains("<pre class=\"mermaid\">"),
            "held for the renderer in the page rather than drawn here",
        );
    }

    /// The ways there is nothing to open, which are the ways there is nothing to
    /// draw a card for — and the name being checked against them is what keeps a
    /// path out of a URL from being joined onto anything.
    #[test]
    fn a_name_this_branch_has_not_written_to_is_nothing_to_open() {
        let repo = Repo::with(&[("mvp", MVP)]);

        assert!(
            repo.opened("mvp").is_none(),
            "a roadmap this branch never touched is not this Conversation's",
        );

        repo.write("public-release", MVP);

        assert!(repo.opened("public-release").is_some());
        assert!(repo.opened("mvp").is_none());
        assert!(repo.opened("..").is_none());
        assert!(repo.opened("../../etc").is_none());

        // And a roadmap this branch did write to whose index plans nothing,
        // which is a heading over nothing in either place it would be drawn.
        repo.write("empty", "# Nothing planned yet\n");

        assert!(repo.opened("empty").is_none());
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
            let list = roadmap(
                &roadmaps.join(name),
                &store::StageStandings::default(),
                &HashSet::new(),
            )
            .unwrap_or_else(|| panic!("{name} should read back as a stage list"));

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

        let stage = only(
            repo.next("mvp", "anything-else"),
            "stage 03 is the one left",
        );

        assert_eq!(stage.roadmap, "mvp");
        assert_eq!(stage.label, "03");
        assert_eq!(stage.title, "Implementation");
        assert_eq!(stage.brief_path, "docs/roadmaps/mvp/03-implementation.md");
        assert_eq!(stage.brief, "# 03. Implementation\n");
        assert_eq!(
            stage.branch(),
            "roadmaps/mvp/03-implementation",
            "the branch is the brief's own name, under the roadmap it belongs to and the \
             fixed component no roadmap's own branch can be called",
        );
        assert_eq!(
            stage.former_branch(),
            "mvp/03-implementation",
            "and the shape it had before that component went in front of it",
        );
    }

    /// And the fallback, which is the number alone: a brief whose name is
    /// nothing but its number has no slug to put after it, and the number is the
    /// one thing every entry of every roadmap has.
    #[test]
    fn a_brief_with_no_slug_is_worked_on_its_number_alone() {
        let stage = Stage {
            roadmap: "mvp".to_owned(),
            label: "04".to_owned(),
            title: "Wrap-up".to_owned(),
            brief_path: "docs/roadmaps/mvp/04.md".to_owned(),
            brief: "# 04.\n".to_owned(),
        };

        assert_eq!(stage.branch(), "roadmaps/mvp/04");
        assert_eq!(stage.former_branch(), "mvp/04");
    }

    /// Only the proper prefixes are asked about, which for a stage branch is
    /// two names and no more — and the branch itself is not one of them. Whether
    /// *it* is there is clause 4's question, and it is a different answer: a
    /// stage already under way rather than a name to go and move.
    #[test]
    fn a_branch_is_never_in_its_own_way() {
        let repo = Repo::with(&[]);

        assert_eq!(
            in_the_way(repo.path(), "roadmaps/mvp/03-implementation"),
            None,
            "a repository with no branch on that path has nothing in the way",
        );

        repo.branch("roadmaps/mvp/03-implementation");

        assert_eq!(
            in_the_way(repo.path(), "roadmaps/mvp/03-implementation"),
            None,
            "and the stage's own branch is not something standing in front of it",
        );
    }

    /// The fallback, which is what a roadmap the record knows nothing about is
    /// read by: a stage's box is still open where it was worked before Verkstead
    /// wrote a record, and what says so is the roadmap's annotation naming the
    /// branch it was worked on.
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

        let stage = only(
            repo.next("mvp", "grilling"),
            "the stage after this Conversation's own is the one to start",
        );

        assert_eq!(stage.label, "03");

        // And to anybody else it is the stage it says it is: the annotation is
        // about whose it is, not about whether it is done.
        let stage = only(
            repo.next("mvp", "some-other-branch"),
            "stage 02 is still unchecked",
        );

        assert_eq!(stage.label, "02");
    }

    /// A roadmap with three stages in it and none of them ticked, which is what a
    /// branch cut before any of them finished holds: whether each of them is done
    /// is the record's to say here, and the boxes say nothing.
    const UNTICKED: &str = "\
# MVP roadmap

- [ ] 01: Workbench — [brief](01-workbench.md)
- [ ] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md)
";

    /// And the same four stages declaring what they stand on, in the shape of the
    /// roadmap that built this: 01 and 02 stand on nothing and are reorderable with
    /// each other, 03 stands on 02, and 04 waits for both 01 and 03.
    ///
    /// Which is what a declaring roadmap is for: the lowest unticked box is never
    /// the answer here unless 01 is what is ready.
    const DECLARED: &str = "\
# MVP roadmap

- [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies
- [ ] 02: Grilling — [brief](02-grilling.md) — no dependencies
- [ ] 03: Implementation — [brief](03-implementation.md) — after 02
- [ ] 04: Wrap-up — [brief](04-wrap-up.md) — after 01, 03
";

    /// What Verkstead's record says about the stages of this Repo's roadmaps.
    ///
    /// Nothing stopped, which is what the readings this feeds care about: whether
    /// a stage halted is the card's question rather than theirs — see
    /// [`halted`], which is where a record with a stop in it is built.
    ///
    /// And every row says Conversation `0`, which is nobody's: what the
    /// Conversation is asked about is whether this server is holding it, and
    /// nothing is held anywhere these are used — see [`on_it`], which names them.
    fn record<'a>(
        rows: impl IntoIterator<Item = (&'a str, &'a str, store::StageStanding)>,
    ) -> store::StageStandings {
        store::StageStandings::from_rows(
            rows.into_iter()
                .map(|(roadmap, label, standing)| (roadmap, label, standing, false, 0)),
        )
    }

    /// And the same with the stage's Conversation **stopped**, which is the other
    /// half of what the card is told: a stage in flight whose run has stopped is
    /// halted rather than in progress.
    fn halted<'a>(
        rows: impl IntoIterator<Item = (&'a str, &'a str, store::StageStanding)>,
    ) -> store::StageStandings {
        store::StageStandings::from_rows(
            rows.into_iter()
                .map(|(roadmap, label, standing)| (roadmap, label, standing, true, 0)),
        )
    }

    /// A stage that settled is a stage done, whatever the box on the branch being
    /// read says.
    ///
    /// This is the whole point of the record. Stage 01 ticked its own box in its
    /// own finish commit, on its own branch; the branch this reading is off was cut
    /// before that landed and holds a `ROADMAP.md` with nothing ticked in it at
    /// all. With the boxes alone the carry-on would start stage 01 a second time.
    #[test]
    fn a_settled_stage_is_done_however_its_box_reads_on_this_branch() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let stage = only(
            repo.next_with(
                "mvp",
                "anything-else",
                &record([("mvp", "01", store::StageStanding::Settled)]),
            ),
            "stage 01 settled, so stage 02 is the one to start",
        );

        assert_eq!(stage.label, "02");
    }

    /// And a stage the record knows nothing about is done where its box is ticked:
    /// worked by hand or by the old tools, and the boxes are all there is to go on.
    ///
    /// Both halves of the rule in one reading, which is what a roadmap carried on
    /// from before this looks like: stage 01 ticked with no row, stage 02 settled
    /// with no tick, and stage 03 the one left.
    #[test]
    fn a_stage_ticked_with_no_record_of_it_is_done_too() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n",
        );
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let stage = only(
            repo.next_with(
                "mvp",
                "anything-else",
                &record([("mvp", "02", store::StageStanding::Settled)]),
            ),
            "the box speaks for 01 and the record for 02, so stage 03 is the one left",
        );

        assert_eq!(stage.label, "03");
    }

    /// A stage the record says is in flight is neither done nor one to start,
    /// however its box reads — and what stands on it is not started either.
    ///
    /// Which is newly load-bearing, and load-bearing in the other direction from
    /// the box: a stage ticks its own box in its finish commit, before its pull
    /// request has even opened, so a ticked box says *its tasks are done* rather
    /// than *it settled*. Both readings of the box are here — 01 ticked and 02 not
    /// — and neither is started.
    ///
    /// Nor is 03, which used to be: the reading walked past a stage in flight and
    /// started whatever came after it. In an undeclared roadmap 03 stands on 02, so
    /// a stage somebody is on holds up what stands on it rather than being stepped
    /// over.
    #[test]
    fn a_stage_the_record_says_is_in_flight_is_not_started_however_its_box_reads() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n",
        );
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let in_flight = record([
            ("mvp", "01", store::StageStanding::InFlight),
            ("mvp", "02", store::StageStanding::InFlight),
        ]);

        assert_eq!(
            repo.next_with("mvp", "anything-else", &in_flight),
            Next::InFlight {
                roadmap: "mvp".to_owned()
            },
            "the two stages somebody is on are not stages to start, and the stage that \
             stands on one of them is not either",
        );

        // And in flight is not done either: a roadmap whose only unstarted stage is
        // the one this reading just refused has nothing to start, which is not the
        // same statement as the roadmap being complete.
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md)\n",
        );

        assert_eq!(
            repo.next_with("mvp", "anything-else", &in_flight),
            Next::InFlight {
                roadmap: "mvp".to_owned()
            },
        );
    }

    /// The stage a declaring roadmap carries on with is the one whose dependency
    /// settled, rather than the lowest unticked box.
    ///
    /// This is the whole of the change, written as the roadmap that built it: 01
    /// and 02 stand on nothing, 03 stands on 02 and 04 on both 01 and 03. With 02
    /// settled and 01 still in flight, 03 is what starts — and the lowest unticked
    /// box is 01, which is the answer this reading used to give.
    #[test]
    fn a_declared_roadmap_carries_on_with_the_stage_whose_dependency_settled() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let stage = only(
            repo.next_with(
                "mvp",
                "roadmaps/mvp/02-grilling",
                &record([
                    ("mvp", "01", store::StageStanding::InFlight),
                    ("mvp", "02", store::StageStanding::Settled),
                ]),
            ),
            "stage 03 stands on 02 alone, and 02 has settled",
        );

        assert_eq!(stage.label, "03");
    }

    /// And a stage whose dependency is in flight is no stage to start, in a
    /// declaring roadmap and in an undeclared one alike.
    ///
    /// Declared here — 04 stands on 01 and 03, and 01 is in flight — and undeclared
    /// in [`a_stage_the_record_says_is_in_flight_is_not_started_however_its_box_reads`],
    /// which is the same rule read off a roadmap that declares nothing.
    #[test]
    fn a_stage_whose_dependency_is_in_flight_is_not_started() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "04-wrap-up.md", "# 04. Wrap-up\n");

        assert_eq!(
            repo.next_with(
                "mvp",
                "roadmaps/mvp/03-implementation",
                &record([
                    ("mvp", "01", store::StageStanding::InFlight),
                    ("mvp", "02", store::StageStanding::Settled),
                    ("mvp", "03", store::StageStanding::Settled),
                ]),
            ),
            Next::InFlight {
                roadmap: "mvp".to_owned()
            },
            "04 stands on 01 as well as on 03, and 01 has not settled",
        );
    }

    /// Nothing ready and nothing in flight is the roadmap complete, and nothing
    /// ready with a stage still in flight is not: two answers rather than one.
    ///
    /// Which used to be one answer, and announcing a roadmap complete while three
    /// of its stages were being worked is what that would have come to.
    #[test]
    fn nothing_ready_is_the_roadmap_complete_only_where_nothing_is_in_flight() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);

        let settled = [
            ("mvp", "01", store::StageStanding::Settled),
            ("mvp", "02", store::StageStanding::Settled),
            ("mvp", "03", store::StageStanding::Settled),
        ];

        assert_eq!(
            repo.next_with(
                "mvp",
                "roadmaps/mvp/03-implementation",
                &record(
                    settled
                        .into_iter()
                        .chain([("mvp", "04", store::StageStanding::InFlight,)])
                ),
            ),
            Next::InFlight {
                roadmap: "mvp".to_owned()
            },
            "the one stage left is one somebody is on, so there is nothing to start",
        );

        assert_eq!(
            repo.next_with(
                "mvp",
                "roadmaps/mvp/04-wrap-up",
                &record(
                    settled
                        .into_iter()
                        .chain([("mvp", "04", store::StageStanding::Settled,)])
                ),
            ),
            Next::Complete {
                roadmap: "mvp".to_owned()
            },
            "and with that one settled there is nothing left at all",
        );
    }

    /// Four stages standing on nothing, which is more roots than one roadmap has
    /// places: what the limit is read off.
    const FOUR_ROOTS: &str = "\
# MVP roadmap

- [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies
- [ ] 02: Grilling — [brief](02-grilling.md) — no dependencies
- [ ] 03: Implementation — [brief](03-implementation.md) — no dependencies
- [ ] 04: Wrap-up — [brief](04-wrap-up.md) — no dependencies
";

    /// A repository holding that roadmap with a brief for every stage of it, so
    /// that what a reading starts is decided by the places rather than by which
    /// briefs somebody wrote.
    fn four_roots() -> Repo {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR_ROOTS);

        for (label, slug) in [
            ("01", "workbench"),
            ("02", "grilling"),
            ("03", "implementation"),
            ("04", "wrap-up"),
        ] {
            repo.brief(
                "mvp",
                &format!("{label}-{slug}.md"),
                &format!("# {label}. {slug}\n"),
            );
        }

        repo
    }

    /// Every ready stage starts off the one settle, rather than the lowest of
    /// them: 01 and 02 both stand on nothing, so a settle starts both.
    ///
    /// Which is the whole of the scheduler. What it replaces is the rule that one
    /// settle started one stage, and a roadmap shaped like a fan worked through
    /// its fan one prong at a time.
    #[test]
    fn a_settle_starts_every_ready_stage_of_a_declaring_roadmap() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let next = repo.next("mvp", "");

        assert_eq!(
            starting(&next),
            ["01", "02"],
            "both roots start, and 03 stands on 02 which has not settled",
        );
        assert!(
            holding(next).is_empty(),
            "with nothing held: there were places for both",
        );
    }

    /// And an undeclared roadmap starts one stage still, which is what *in order*
    /// comes to: every stage of it stands on the one before it, so there is never
    /// a second ready stage for a second place to be given to.
    #[test]
    fn a_settle_on_an_undeclared_roadmap_still_starts_one_stage() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let next = repo.next("mvp", "");

        assert_eq!(starting(&next), ["01"]);
        assert!(holding(next).is_empty());
    }

    /// Three of one roadmap at once, and the fourth ready stage waits for a place
    /// — and is told it is waiting, so that a roadmap which has gone quiet says
    /// why rather than looking like one the scheduler forgot.
    ///
    /// The lowest-numbered ready stages are the ones that start: the roadmap's
    /// order is still the roadmap's own, and it is what decides between stages
    /// that are otherwise alike.
    #[test]
    fn a_ready_stage_beyond_the_roadmaps_places_waits_and_says_so() {
        let repo = four_roots();

        let next = repo.next("mvp", "");

        assert_eq!(
            starting(&next),
            ["01", "02", "03"],
            "three of one roadmap at once, lowest first",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and the fourth waits: {held:?}");
        assert!(
            held[0].contains("04") && held[0].contains("waiting for a place"),
            "which stage, and what it is waiting for: {held:?}",
        );
    }

    /// A place is taken by every stage the record has in flight, whatever that
    /// stage is doing: one in flight is one fewer place for this settle to give.
    ///
    /// Which is what makes a stage blocked on a Question Set hold its place — the
    /// human's choice, so that a roadmap they have stopped to answer does not fill
    /// up with work behind them.
    #[test]
    fn a_stage_in_flight_takes_one_of_the_roadmaps_places() {
        let repo = four_roots();

        let next = repo.next_with(
            "mvp",
            "",
            &record([("mvp", "01", store::StageStanding::InFlight)]),
        );

        assert_eq!(
            starting(&next),
            ["02", "03"],
            "01 is somebody's, and it is holding one of the three places",
        );

        let held = holding(next);

        assert_eq!(
            held.len(),
            1,
            "so only one place was left to give: {held:?}"
        );
        assert!(held[0].contains("04"), "and 04 is what waits: {held:?}");
    }

    /// And a roadmap already running as many as it runs at once starts nothing at
    /// all — with a sentence per ready stage saying so, rather than the silence a
    /// roadmap with nothing ready keeps.
    #[test]
    fn a_roadmap_with_no_places_left_starts_nothing_and_says_why() {
        let repo = four_roots();

        let next = repo.next_with(
            "mvp",
            "",
            &record([
                ("mvp", "01", store::StageStanding::InFlight),
                ("mvp", "02", store::StageStanding::InFlight),
                ("mvp", "03", store::StageStanding::InFlight),
            ]),
        );

        assert!(
            starting(&next).is_empty(),
            "three of this roadmap are already in flight: {next:?}",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and 04 is the one ready stage: {held:?}");
        assert!(
            held[0].contains("04") && held[0].contains("waiting for a place"),
            "told it is waiting rather than left to look forgotten: {held:?}",
        );
    }

    /// How many places there are is the caller's to say, and what says it is the
    /// `at_once.roadmap_stages` setting — see [`crate::settings::AtOnce`]. One
    /// place runs a declared roadmap in order, whatever its declarations allow.
    #[test]
    fn a_roadmap_with_one_place_starts_one_stage_at_a_time() {
        let repo = four_roots();

        let next = repo.next_limited("mvp", "", &store::StageStandings::default(), 1);

        assert_eq!(starting(&next), ["01"]);
        assert_eq!(
            holding(next).len(),
            3,
            "and the other three roots are told they are waiting",
        );
    }

    /// And the one place a roadmap set to one has is held by the stage in flight
    /// the same way three are: a ready stage does not take the place of a stage
    /// somebody has been asked a question about.
    ///
    /// Which is the setting's own half of *a stage blocked on the human holds its
    /// place*. What the stage is doing never comes into it — in flight by the
    /// record is a place gone — so a roadmap run one at a time starts its next
    /// stage when the one before it settles and not before.
    #[test]
    fn the_one_place_a_roadmap_set_to_one_has_is_held_by_a_stage_in_flight() {
        let repo = four_roots();

        let next = repo.next_limited(
            "mvp",
            "",
            &record([("mvp", "01", store::StageStanding::InFlight)]),
            1,
        );

        assert!(
            starting(&next).is_empty(),
            "01 is somebody's, and it is the only place there is: {next:?}",
        );

        let held = holding(next);

        assert_eq!(held.len(), 3, "and the three roots left wait: {held:?}");
        assert!(
            held[0].contains("02") && held[0].contains("runs one stage at a time"),
            "told what it is waiting for, in the number the setting says: {held:?}",
        );
    }

    /// And a roadmap already over the number it is now allowed starts nothing,
    /// rather than going backwards: the limit was lowered in Settings while three
    /// of its stages were under way.
    ///
    /// Which is the reading's half of *a change stops nothing already running*.
    /// Nothing here can stop a stage — it is asked what may start — so a limit
    /// lowered under work in flight is three stages that carry on and no fourth,
    /// and the ready stage is told it is waiting.
    #[test]
    fn a_roadmap_over_a_lowered_limit_starts_nothing_and_stops_nothing() {
        let repo = four_roots();

        let next = repo.next_limited(
            "mvp",
            "",
            &record([
                ("mvp", "01", store::StageStanding::InFlight),
                ("mvp", "02", store::StageStanding::InFlight),
                ("mvp", "03", store::StageStanding::InFlight),
            ]),
            1,
        );

        assert!(
            starting(&next).is_empty(),
            "three are under way and one place is allowed: {next:?}",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and 04 is the one ready stage: {held:?}");
        assert!(
            held[0].contains("04") && held[0].contains("waiting for a place"),
            "told it is waiting rather than left to look forgotten: {held:?}",
        );
    }

    /// And the **second** limit in front of the first: a ready stage that fits
    /// under its roadmap's places and not under the server's waits, and is told it
    /// is the server it is waiting on.
    ///
    /// A sentence of its own rather than the roadmap's, because what it is waiting
    /// for is a different thing — nothing about this roadmap will free the place —
    /// and a roadmap gone quiet says which of the two it went quiet for. See
    /// [`CONVERSATIONS_AT_ONCE`].
    #[test]
    fn a_ready_stage_beyond_the_servers_places_waits_and_says_so() {
        let repo = four_roots();

        // Four places of its own, so nothing here is the roadmap's limit, and two
        // free across the server: the Conversations holding the other two belong
        // to whatever they belong to.
        let next = repo.next_on_a_server("mvp", &store::StageStandings::default(), 4, 2);

        assert_eq!(
            starting(&next),
            ["01", "02"],
            "two places on the server, and the lowest-numbered two take them",
        );

        let held = holding(next);

        assert_eq!(held.len(), 2, "and the other two wait: {held:?}");

        for (stage, said) in ["03", "04"].iter().zip(&held) {
            assert!(
                said.contains(stage) && said.contains("waiting for a place on the server"),
                "which stage, and which of the two limits it is waiting on: {said:?}",
            );
            assert!(
                !said.contains("this roadmap runs"),
                "and not its roadmap's, which has places to spare: {said:?}",
            );
        }
    }

    /// A server with no place free starts nothing at all, however much room the
    /// roadmap itself has — with a sentence per ready stage, rather than the
    /// silence a roadmap with nothing ready keeps.
    ///
    /// Which is the one that has to be visible: a server whose places are all held
    /// by stages waiting on answers starts nothing more until one is answered, and
    /// that is a thing to say rather than a stall to look like.
    #[test]
    fn a_server_with_no_places_left_starts_nothing_and_says_why() {
        let repo = four_roots();

        let next = repo.next_on_a_server("mvp", &store::StageStandings::default(), 4, 0);

        assert!(
            starting(&next).is_empty(),
            "every place on the server is taken: {next:?}",
        );

        let held = holding(next);

        assert_eq!(held.len(), 4, "and all four ready stages wait: {held:?}");
        assert!(
            held.iter()
                .all(|said| said.contains("waiting for a place on the server")),
            "each told what it is waiting on: {held:?}",
        );
    }

    /// And a server with places to spare leaves the roadmap's own limit to do the
    /// holding, in the roadmap's own words.
    ///
    /// The two limits are both in force and the stricter one is what a stage is
    /// told about. Three of one roadmap and four free on the server is the shape
    /// every default machine is in.
    #[test]
    fn a_stage_held_by_its_roadmap_is_not_told_the_server_is_full() {
        let repo = four_roots();

        let next = repo.next_on_a_server(
            "mvp",
            &store::StageStandings::default(),
            AT_ONCE,
            CONVERSATIONS_AT_ONCE,
        );

        assert_eq!(
            starting(&next),
            ["01", "02", "03"],
            "the roadmap's three, the server having four to give",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and 04 is the one that waits: {held:?}");
        assert!(
            held[0].contains("this roadmap runs") && !held[0].contains("on the server"),
            "told it is its roadmap's places that are taken: {held:?}",
        );
    }

    /// And a place on the server is spent by a stage that **starts** and by
    /// nothing else, the way a roadmap's is: a stage held for a brief nobody wrote
    /// leaves its place to whatever is ready after it.
    ///
    /// Otherwise a server with two places free would run one stage and tell the
    /// next one the machine was full while a place stood free.
    #[test]
    fn a_stage_with_no_brief_leaves_its_place_on_the_server_behind_it() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR_ROOTS);

        // Every brief but 02's, which is the entry that halts itself.
        for (label, slug) in [
            ("01", "workbench"),
            ("03", "implementation"),
            ("04", "wrap-up"),
        ] {
            repo.brief(
                "mvp",
                &format!("{label}-{slug}.md"),
                &format!("# {label}. {slug}\n"),
            );
        }

        let next = repo.next_on_a_server("mvp", &store::StageStandings::default(), 4, 2);

        assert_eq!(
            starting(&next),
            ["01", "03"],
            "02 halted for its own brief and left its place to 03",
        );

        let held = holding(next);

        assert_eq!(held.len(), 2, "and 02 and 04 are both held: {held:?}");
        assert!(
            held[0].contains("02") && held[0].contains("there is nothing there to read"),
            "02 for the brief nobody wrote: {held:?}",
        );
        assert!(
            held[1].contains("04") && held[1].contains("waiting for a place on the server"),
            "and 04 for the place 03 took: {held:?}",
        );
    }

    /// The one free place on the server goes to the **lowest-numbered ready
    /// stage**, whatever else the roadmap is holding.
    ///
    /// Within a roadmap the order is the roadmap's own and it survives the second
    /// limit: the ready stages come back lowest-numbered first and the place is
    /// spent by the first of them, so nothing about how many places there are
    /// reorders them. *Lowest-numbered ready* rather than lowest-numbered — 01 is
    /// settled here and 02 is somebody's, so the roadmap's own order picks 03 out
    /// of what is left.
    ///
    /// Nothing is stored about which of them was held first, and that is the
    /// point: the answer is read afresh off the roadmap's own list every time,
    /// so the order two stages were held back in cannot come to disagree with it.
    #[test]
    fn the_one_free_place_goes_to_the_lowest_numbered_ready_stage() {
        let repo = four_roots();

        // Four places of its own with one already spent, so the roadmap's limit
        // is never what decides between 03 and 04 — and one place on the server.
        let next = repo.next_on_a_server(
            "mvp",
            &record([
                ("mvp", "01", store::StageStanding::Settled),
                ("mvp", "02", store::StageStanding::InFlight),
            ]),
            4,
            1,
        );

        assert_eq!(
            starting(&next),
            ["03"],
            "the lowest-numbered stage that is ready takes the place",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and 04 is the one that waits: {held:?}");
        assert!(
            held[0].contains("04") && held[0].contains("waiting for a place on the server"),
            "told which place it is waiting for: {held:?}",
        );
    }

    /// A roadmap **already at its own limit** spends no place on the server,
    /// however many are free: it starts nothing, so it takes nothing, and the
    /// place is still there for the roadmap the look reads after it.
    ///
    /// The two limits are both in force and the roadmap's is the stricter one
    /// here. A place is spent by a stage that starts and by nothing else — see
    /// [`crate::places`], where each roadmap's reading counts the places itself
    /// for exactly this reason — so a roadmap passed over leaves the place behind
    /// it rather than standing on it.
    #[test]
    fn a_roadmap_at_its_own_limit_spends_no_place_on_the_server() {
        let repo = four_roots();

        // Three of it in flight against its limit of three, and a place free on
        // the server.
        let next = repo.next_on_a_server(
            "mvp",
            &record([
                ("mvp", "01", store::StageStanding::InFlight),
                ("mvp", "02", store::StageStanding::InFlight),
                ("mvp", "03", store::StageStanding::InFlight),
            ]),
            AT_ONCE,
            1,
        );

        assert!(
            starting(&next).is_empty(),
            "its own three places are taken, so nothing of it starts: {next:?}",
        );

        let held = waits_on(next);

        assert_eq!(held.len(), 1, "and 04 is the one ready stage: {held:?}");
        assert!(
            matches!(&held[0], Held::Roadmap(said) if said.contains("04")
                && said.contains("this roadmap runs")
                && !said.contains("on the server")),
            "waiting on its own roadmap rather than on the machine: {held:?}",
        );
    }

    /// And what each held stage is waiting on comes back beside the sentence,
    /// which is what a **look** tells them apart by — see
    /// [`crate::continuing::Brought`].
    ///
    /// Two of the three in one reading, which is as many as one can hold: 02's
    /// brief is missing, so it halts for itself and leaves its place; 01 and 03
    /// take the two places the server has; and 04 waits on the server. The third
    /// is the roadmap's own, and a reading holds either that one or the server's
    /// — whichever of the two limits bites first — never both. See
    /// [`a_roadmap_at_its_own_limit_spends_no_place_on_the_server`] for it.
    ///
    /// Which of the two a stage is waiting on is what a look decides by, so it
    /// cannot be a matter of reading the sentence back.
    #[test]
    fn a_held_stage_comes_back_under_what_it_is_waiting_on() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR_ROOTS);

        // Every brief but 02's, which is the entry that halts itself.
        for (label, slug) in [
            ("01", "workbench"),
            ("03", "implementation"),
            ("04", "wrap-up"),
        ] {
            repo.brief(
                "mvp",
                &format!("{label}-{slug}.md"),
                &format!("# {label}. {slug}\n"),
            );
        }

        let held = waits_on(repo.next_on_a_server("mvp", &store::StageStandings::default(), 4, 2));

        assert!(
            matches!(&held[0], Held::Halted(said) if said.contains("02")),
            "02 halted for the brief nobody wrote, which no place coming free would fix: \
             {held:?}",
        );
        assert!(
            matches!(&held[1], Held::Server(said) if said.contains("04")),
            "and 04 waiting on the server, 01 and 03 having taken its two places: {held:?}",
        );
        assert_eq!(held.len(), 2, "and nothing else was held: {held:?}");
    }

    /// A stage that halts holds up only the stages that stand on it: 04 stands on
    /// 01, which is in flight, and 03 stands on 02, which has settled.
    ///
    /// The scheduler's half of a halt in the middle. What a halted stage is — one
    /// blocked on a Question Set, one waiting to join the chain — never comes into
    /// it: it is in flight by the record, so what stands on it waits and what
    /// stands beside it does not.
    #[test]
    fn a_stage_in_flight_holds_up_its_own_dependents_and_no_others() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let next = repo.next_with(
            "mvp",
            "",
            &record([
                ("mvp", "01", store::StageStanding::InFlight),
                ("mvp", "02", store::StageStanding::Settled),
            ]),
        );

        assert_eq!(
            starting(&next),
            ["03"],
            "02's dependent starts, and 04 stands on 01 as well as on 03",
        );
        assert!(
            holding(next).is_empty(),
            "04 is not ready rather than held: nothing it stands on has settled",
        );
    }

    /// The ready stages come back lowest number first, and a declaring roadmap can
    /// have several of them: 01 and 02 stand on nothing, so both may start.
    ///
    /// The order is what decides which of them start where there are more than
    /// places — see [`a_ready_stage_beyond_the_roadmaps_places_waits_and_says_so`]
    /// — so it is the reading's own and is asserted here.
    #[test]
    fn the_ready_stages_of_a_declaring_roadmap_come_back_lowest_first() {
        let Ready::Stages { lowest, rest, .. } =
            ready("mvp", DECLARED, &store::StageStandings::default(), "")
        else {
            panic!("both roots of this roadmap may start");
        };

        assert_eq!(lowest.label, "01");
        assert_eq!(
            rest.iter().map(|entry| entry.label).collect::<Vec<_>>(),
            ["02"],
            "and 03 stands on 02, which has not settled",
        );
    }

    /// And an undeclared roadmap has one ready stage at most, whatever the record
    /// says: each stage stands on the one before it, so the one below the lowest
    /// unticked box has to have settled before anything above it is ready.
    #[test]
    fn an_undeclared_roadmap_has_one_ready_stage_at_most() {
        let Ready::Stages { lowest, rest, .. } =
            ready("mvp", UNTICKED, &store::StageStandings::default(), "")
        else {
            panic!("the first stage of it stands on nothing");
        };

        assert_eq!(lowest.label, "01");
        assert!(
            rest.is_empty(),
            "02 stands on 01, which nothing says has settled: {:?}",
            rest.iter().map(|entry| entry.label).collect::<Vec<_>>(),
        );
    }

    /// And a roadmap that declares badly has nothing ready at all, with the fault
    /// in the words the judgement refuses it in.
    ///
    /// Refused rather than repaired, and never read in order instead: what the
    /// reading answers is what stops every start, and where that sentence is said
    /// is each of the three readings' own.
    #[test]
    fn a_roadmap_that_declares_badly_has_nothing_ready() {
        let Ready::Misdeclared(why) = ready(
            "mvp",
            "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n",
            &store::StageStandings::default(),
            "",
        ) else {
            panic!("a declaration on one line and not the other is refused");
        };

        assert!(
            why.contains("mvp") && why.contains("02"),
            "which roadmap, and which line to go and read: {why:?}",
        );
    }

    /// A stage the record says was abandoned is left to its box, and nothing here
    /// starts it again on the strength of the record: what it left behind is a
    /// branch by the stage's name, and that is what refuses it where a stage is
    /// started.
    #[test]
    fn an_abandoned_stage_is_left_to_what_its_box_says() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let abandoned = record([("mvp", "01", store::StageStanding::Abandoned)]);

        let stage = only(
            repo.next_with("mvp", "anything-else", &abandoned),
            "an abandoned stage is not a settled one, so its unticked box stands",
        );

        assert_eq!(
            stage.label, "01",
            "the stage is offered and the branch it left behind is what refuses it",
        );

        // And where somebody ticked it off by hand, the box is what there is to go
        // on: abandoned says nothing about whether the work got done.
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n",
        );

        let stage = only(
            repo.next_with("mvp", "anything-else", &abandoned),
            "stage 02 is the one left",
        );

        assert_eq!(stage.label, "02");
    }

    /// The whole of an in-order roadmap, which is every roadmap until stages are
    /// worked side by side: the stage whose settling brought this reading about is
    /// skipped by its own row, and the stage after it starts.
    ///
    /// Its box is unticked on this branch twice over — the tick rode in its own
    /// finish commit on its own branch, and this reading is off the branch of the
    /// stage before it — and the annotation is the roadmap saying whose it is.
    #[test]
    fn a_roadmap_run_in_order_carries_on_from_the_stage_that_settled() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) \
             *(in progress: `roadmaps/mvp/02-grilling`)*\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n",
        );
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let stage = only(
            repo.next_with(
                "mvp",
                "roadmaps/mvp/02-grilling",
                &record([
                    ("mvp", "01", store::StageStanding::Settled),
                    ("mvp", "02", store::StageStanding::Settled),
                ]),
            ),
            "stage 02 has settled, so stage 03 is the one to start",
        );

        assert_eq!(stage.label, "03");
    }

    /// And a stage whose row holds a roadmap and no label is skipped by its own
    /// annotation, which is the only thing that keeps it from being offered its own
    /// stage back.
    ///
    /// Every stage started between ADR-0017 landing and the label being written
    /// down is one of those: the record has rows for the stages around it and
    /// nothing for this one, so the annotation is what says whose stage 02 is.
    #[test]
    fn a_stage_the_record_holds_no_label_for_is_still_skipped_by_its_annotation() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) \
             *(in progress: `roadmaps/mvp/02-grilling`)*\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n",
        );
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

        let stage = only(
            repo.next_with(
                "mvp",
                "roadmaps/mvp/02-grilling",
                &record([("mvp", "01", store::StageStanding::Settled)]),
            ),
            "the stage after this Conversation's own is the one to start",
        );

        assert_eq!(stage.label, "03");
    }

    /// Two Repos may hold roadmaps of one name, and one Repo's stages answer for
    /// nothing in another: the record handed to this reading is the Repo's own.
    ///
    /// Written as the thing the reading cannot do rather than as a fact about the
    /// rows — what keeps the two apart is that the record is read per Repo, which
    /// is `store::stage_standings`' own rule and is asserted where it is read.
    #[test]
    fn what_another_roadmap_of_this_repo_settled_says_nothing_about_this_one() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");

        let stage = only(
            repo.next_with(
                "mvp",
                "anything-else",
                &record([("public-release", "01", store::StageStanding::Settled)]),
            ),
            "nothing has settled in this roadmap, whatever another one has",
        );

        assert_eq!(stage.label, "01");
    }

    /// And a declaration on the line does not hide it, whichever way round the
    /// two were written. They share one tail: the annotation is matched by the
    /// branch in backticks and a declaration holds none, which is what keeps the
    /// stage in flight findable beside one.
    #[test]
    fn a_declaration_on_the_line_does_not_hide_the_stage_in_flight() {
        for tail in [
            "— after 01 *(in progress: `grilling`)*",
            "*(in progress: `grilling`)* — after 01",
        ] {
            let repo = Repo::with(&[]);
            repo.write(
                "mvp",
                &format!(
                    "# MVP roadmap\n\n\
                     - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
                     - [ ] 02: Grilling — [brief](02-grilling.md) {tail}\n\
                     - [ ] 03: Implementation — [brief](03-implementation.md) — after 02\n"
                ),
            );
            repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
            repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");

            let stage = only(
                repo.next("mvp", "grilling"),
                &format!("the stage after this Conversation's own is the one to start: {tail:?}"),
            );

            assert_eq!(stage.label, "03", "{tail:?}");
        }
    }

    /// And a declaration is not mistaken for an annotation: a roadmap every line
    /// of which declares is still a roadmap with no stage in flight, so the
    /// lowest unticked box is the one to start.
    #[test]
    fn a_declaration_is_nobodys_stage_in_flight() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) — after 01 — on windows\n",
        );
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let stage = only(
            repo.next("mvp", "grilling"),
            "stage 02 is unticked and nobody's",
        );

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
            repo.next("mvp", "anything-else"),
            Next::Complete {
                roadmap: "mvp".to_owned()
            },
        );
    }

    /// Including the last stage of one whose box nothing ticked, which is what the
    /// annotation is left saying where there is no record to go on.
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
            repo.next("mvp", "grilling"),
            Next::Complete {
                roadmap: "mvp".to_owned()
            },
        );
    }

    /// And the whole point of naming the roadmap: a roadmap running out is the
    /// roadmap complete, whatever else is in the Worktree with work left in it.
    ///
    /// This is the bug, written as a test. The reading used to walk every
    /// roadmap the branch had touched in name order, so a stage of
    /// `missing-roles` whose own roadmap ran out started stage 14 of
    /// `brain-chat-parity` — a different effort, which nobody had selected.
    /// Adopting that one is the human's act, from *Continue a roadmap*.
    #[test]
    fn a_named_roadmap_running_out_does_not_carry_on_into_another() {
        let repo = Repo::with(&[]);

        // Sorts first, and is somebody else's effort with work left in it.
        repo.write(
            "brain-chat-parity",
            "# Brain chat parity roadmap\n\n\
             - [x] 13: The agent chat — [brief](13-agent-chat.md)\n\
             - [ ] 14: The widget — [brief](14-brain-chat-widget.md)\n",
        );
        repo.brief("brain-chat-parity", "14-brain-chat-widget.md", "# 14.\n");

        // And this Conversation's own, down to its last stage — the one in
        // flight on this branch, whose box nothing is ever going to tick.
        repo.write(
            "missing-roles",
            "# Missing roles roadmap\n\n\
             - [x] 01: Project config — [brief](01-project-config.md)\n\
             - [ ] 02: Grant filters — [brief](02-grant-filters.md) \
             *(in progress: `missing-roles/02-grant-filters`)*\n",
        );

        assert_eq!(
            repo.next("missing-roles", "missing-roles/02-grant-filters"),
            Next::Complete {
                roadmap: "missing-roles".to_owned()
            },
            "the roadmap the record names is the only one read",
        );
    }

    /// And a stage of one roadmap carries on into that roadmap even where the
    /// branch amended another in passing — retiring a deferral, correcting a
    /// decision a later effort recorded.
    #[test]
    fn a_roadmap_this_branch_only_amended_is_not_the_one_carried_on() {
        let repo = Repo::with(&[]);

        repo.write(
            "brain-chat-parity",
            "# Brain chat parity roadmap\n\n\
             - [ ] 14: The widget — [brief](14-brain-chat-widget.md)\n",
        );
        repo.brief("brain-chat-parity", "14-brain-chat-widget.md", "# 14.\n");

        repo.write(
            "missing-roles",
            "# Missing roles roadmap\n\n\
             - [x] 01: Project config — [brief](01-project-config.md)\n\
             - [ ] 02: Grant filters — [brief](02-grant-filters.md) \
             *(in progress: `missing-roles/02-grant-filters`)*\n\
             - [ ] 03: Workflow permissions — [brief](03-workflow-permissions.md)\n",
        );
        repo.brief("missing-roles", "03-workflow-permissions.md", "# 03.\n");

        let stage = only(
            repo.next("missing-roles", "missing-roles/02-grant-filters"),
            "stage 03 of this Conversation's own roadmap is the one to start",
        );

        assert_eq!(stage.roadmap, "missing-roles");
        assert_eq!(stage.label, "03");
    }

    /// A roadmap the record names and the branch does not hold is said out loud
    /// rather than guessed past — renamed, deleted, or never on this branch at
    /// all. The same treatment a stage naming a brief nobody wrote gets.
    #[test]
    fn a_recorded_roadmap_that_is_not_there_is_not_startable() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);

        let Next::Unstartable { why } = repo.next("public-release", "anything-else") else {
            panic!("there is no public-release roadmap on this branch");
        };

        assert!(
            why.contains("public-release") && why.contains("ROADMAP.md"),
            "which roadmap, and what was looked for: {why:?}",
        );
    }

    /// And one whose index plans nothing, which is a directory rather than a
    /// roadmap however it got that way.
    #[test]
    fn a_recorded_roadmap_with_no_stages_in_it_is_not_startable() {
        let repo = Repo::with(&[]);
        repo.write("mvp", "# MVP roadmap\n\nNothing planned yet.\n");

        let Next::Unstartable { why } = repo.next("mvp", "anything-else") else {
            panic!("an index with no entries has no stage to start");
        };

        assert!(
            why.contains("mvp") && why.contains("no stages"),
            "which roadmap, and what is wrong with it: {why:?}",
        );
    }

    /// A stage that cannot be started is said rather than skipped: starting the
    /// one after it would be Verkstead deciding to leave work out.
    ///
    /// Held rather than refusing the whole reading, which is what it used to be:
    /// a brief nobody wrote halts the one stage that names it, and the roadmap's
    /// other ready stages start anyway. Here there are none, so the held sentence
    /// is the whole of the answer.
    #[test]
    fn a_stage_whose_brief_is_missing_is_not_startable() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);

        let next = repo.next("mvp", "anything-else");

        assert!(
            starting(&next).is_empty(),
            "there is no 03-implementation.md to start stage 03 from: {next:?}",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "one stage was ready: {held:?}");
        assert!(
            held[0].contains("03") && held[0].contains("03-implementation.md"),
            "which stage and which brief: {held:?}",
        );
    }

    /// And a stage naming no brief at all is said rather than read: at a commit,
    /// the roadmap's own directory is a path git will happily hand back a listing
    /// of, so the entry with nothing in its link is stopped by name.
    #[test]
    fn a_stage_naming_no_brief_is_not_startable() {
        let repo = Repo::with(&[]);
        repo.write("mvp", "# MVP roadmap\n\n- [ ] 03: Implementation\n");

        let next = repo.next("mvp", "anything-else");

        assert!(
            starting(&next).is_empty(),
            "stage 03 names no brief to be primed from: {next:?}",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "one stage was ready: {held:?}");
        assert!(
            held[0].contains("03") && held[0].contains("no brief"),
            "which stage, and what its line is missing: {held:?}",
        );
    }

    /// And a brief nobody wrote stops the stage that names it and no other: its
    /// siblings start, which is the whole of *each start is its own act*.
    ///
    /// The one place a reading can halt a stage by itself — everything else that
    /// stops one is the making of a branch and a worktree, which is the carry-on's
    /// — so it is the one place this rule is worth reading off a reading.
    #[test]
    fn a_missing_brief_holds_its_own_stage_and_starts_its_siblings() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let next = repo.next("mvp", "");

        assert_eq!(
            starting(&next),
            ["02"],
            "01 and 02 both stand on nothing, and only 02 has a brief to be primed from",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and 01 is the one held: {held:?}");
        assert!(
            held[0].contains("01") && held[0].contains("01-workbench.md"),
            "with the brief nobody wrote named: {held:?}",
        );
    }

    /// And the place that stage would have had goes to the ready stage behind it:
    /// a place is spent by a stage that starts and by nothing else.
    ///
    /// Four roots and three places, with the brief of the lowest one nobody wrote.
    /// Counting how far down the ready stages the reading had walked would run two
    /// of the three it is allowed and tell the fourth its places were taken — which
    /// is a roadmap quietly running short of what it was set to, and a sentence that
    /// is not true.
    #[test]
    fn a_stage_held_for_want_of_a_brief_leaves_its_place_to_the_next() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR_ROOTS);

        for (label, slug) in [
            ("02", "grilling"),
            ("03", "implementation"),
            ("04", "wrap-up"),
        ] {
            repo.brief(
                "mvp",
                &format!("{label}-{slug}.md"),
                &format!("# {label}. {slug}\n"),
            );
        }

        let next = repo.next("mvp", "");

        assert_eq!(
            starting(&next),
            ["02", "03", "04"],
            "three started, which is the three places the roadmap has",
        );

        let held = holding(next);

        assert_eq!(held.len(), 1, "and 01 is the one held: {held:?}");
        assert!(
            held[0].contains("01") && held[0].contains("01-workbench.md"),
            "for the brief nobody wrote rather than for a place: {held:?}",
        );
        assert!(
            !held[0].contains("waiting for a place"),
            "nothing here waited for a place: {held:?}",
        );
    }

    /// A roadmap as the branch that has just settled holds it: stage 03 stands on
    /// 02, which is the stage somebody is still on.
    const BEFORE_THE_EDIT: &str = "\
# Rate limiting roadmap

- [ ] 01: The counter — [brief](01-the-counter.md) — no dependencies
- [ ] 02: The window — [brief](02-the-window.md) — no dependencies
- [ ] 03: The limiter — [brief](03-the-limiter.md) — after 02
";

    /// And the same roadmap with one dependency edited by hand: stage 03 stands on
    /// 01, which has settled. One line different, and a different stage starts.
    const AFTER_THE_EDIT: &str = "\
# Rate limiting roadmap

- [ ] 01: The counter — [brief](01-the-counter.md) — no dependencies
- [ ] 02: The window — [brief](02-the-window.md) — no dependencies
- [ ] 03: The limiter — [brief](03-the-limiter.md) — after 01
";

    /// Where a roadmap being driven is read: the branch at the top of its chain,
    /// which is where the newest of its declarations are.
    ///
    /// A dependency edited by hand and committed there is what decides what starts
    /// next. The settling stage's own branch was cut before the edit and says
    /// nothing may start; the edit says stage 03 may, and the edit is the reading.
    #[test]
    fn a_dependency_edited_at_the_top_of_the_chain_decides_what_starts() {
        let repo = Repo::with(&[]);
        repo.write("rate-limiting", BEFORE_THE_EDIT);
        repo.brief("rate-limiting", "03-the-limiter.md", "# 03. The limiter\n");

        // What the stage that has just settled holds, and what it settled first.
        let cut_before = repo.committed();

        repo.write("rate-limiting", AFTER_THE_EDIT);
        repo.commit_on(
            "roadmaps/rate-limiting/02-the-window",
            "docs: 03 stands on 01 after all",
        );

        let record = record([
            ("rate-limiting", "01", store::StageStanding::Settled),
            ("rate-limiting", "02", store::StageStanding::InFlight),
        ]);

        let top = "roadmaps/rate-limiting/02-the-window";

        let stage = only(
            repo.next_at(
                top,
                &repo.commit_of(top),
                "rate-limiting",
                "roadmaps/rate-limiting/01-the-counter",
                &record,
            ),
            "the edit at the top of the chain has 03 standing on the stage that settled",
        );

        assert_eq!(stage.label, "03");

        assert_eq!(
            repo.next_at(
                "main",
                &cut_before,
                "rate-limiting",
                "roadmaps/rate-limiting/01-the-counter",
                &record,
            ),
            Next::InFlight {
                roadmap: "rate-limiting".to_owned()
            },
            "and the branch that settled was cut before the edit, where 03 stands on the \
             stage somebody is still on",
        );
    }

    /// And a stage *added* by hand at the top of the chain starts, brief and all —
    /// the whole reading being at that one commit, so an entry the settling
    /// branch has never seen is started from a brief it has never held.
    #[test]
    fn a_stage_added_at_the_top_of_the_chain_is_started_from_there() {
        let repo = Repo::with(&[]);
        repo.write(
            "rate-limiting",
            "# Rate limiting roadmap\n\n\
             - [ ] 01: The counter — [brief](01-the-counter.md) — no dependencies\n",
        );

        let cut_before = repo.committed();

        // The stage somebody added while this one was working, brief beside it.
        repo.write(
            "rate-limiting",
            "# Rate limiting roadmap\n\n\
             - [ ] 01: The counter — [brief](01-the-counter.md) — no dependencies\n\
             - [ ] 02: The window — [brief](02-the-window.md) — after 01\n",
        );
        repo.brief("rate-limiting", "02-the-window.md", "# 02. The window\n");
        repo.commit_on(
            "roadmaps/rate-limiting/01-the-counter",
            "docs: a window to count in",
        );

        let record = record([("rate-limiting", "01", store::StageStanding::Settled)]);

        let top = "roadmaps/rate-limiting/01-the-counter";

        let stage = only(
            repo.next_at(top, &repo.commit_of(top), "rate-limiting", top, &record),
            "stage 02 was added at the top of the chain and stands on the one that settled",
        );

        assert_eq!(stage.label, "02");
        assert_eq!(
            stage.brief, "# 02. The window\n",
            "and its brief is read at the same commit as the line that names it",
        );

        assert_eq!(
            repo.next_at("main", &cut_before, "rate-limiting", top, &record),
            Next::Complete {
                roadmap: "rate-limiting".to_owned()
            },
            "where the branch that settled would have called the roadmap finished",
        );
    }

    /// What says a roadmap Conversation chose a roadmap: its branch created one.
    ///
    /// Uncommitted first, which is what a session part-way through its work
    /// leaves, and then committed, which is where it ends up. The answer is the
    /// same either way — a roadmap is created once, and committing it is not a
    /// second creation.
    #[test]
    fn a_branch_that_wrote_one_roadmap_created_one() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.brief("mvp", "03-implementation.md", "# 03.\n");

        assert_eq!(
            repo.created(),
            BTreeSet::from(["mvp".to_owned()]),
            "untracked, which is the roadmap before the commit lands",
        );

        repo.commit();

        assert_eq!(
            repo.created(),
            BTreeSet::from(["mvp".to_owned()]),
            "and added against the base commit, which is it afterwards",
        );
    }

    /// Two roadmaps written in one go are two efforts planned together, and
    /// there is nothing to prefer between them. Both come back, and the caller
    /// records neither.
    #[test]
    fn a_branch_that_wrote_two_roadmaps_created_two() {
        let repo = Repo::with(&[]);
        repo.write("mvp", MVP);
        repo.write(
            "public-release",
            "# Public release roadmap\n\n- [ ] 01: Packaging — [brief](01-packaging.md)\n",
        );

        assert_eq!(
            repo.created(),
            BTreeSet::from(["mvp".to_owned(), "public-release".to_owned()]),
        );
    }

    /// And a roadmap this branch only amended is a roadmap somebody else
    /// planned: touched, and created by nobody here.
    ///
    /// The index was there at the base commit, so however much of it this branch
    /// rewrote — a box ticked, a stage annotated, a brief added beside it —
    /// nothing on this branch created a roadmap.
    #[test]
    fn a_branch_that_only_amended_a_roadmap_created_none() {
        let repo = Repo::with(&[("mvp", MVP)]);

        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [x] 02: Grilling — [brief](02-grilling.md)\n\
             - [x] 03: Implementation — [brief](03-implementation.md)\n",
        );
        repo.brief("mvp", "03-implementation.md", "# 03.\n");

        assert_eq!(
            repo.touched(),
            BTreeSet::from(["mvp".to_owned()]),
            "the branch wrote to it, which is what the pinned list is drawn from",
        );

        assert!(
            repo.created().is_empty(),
            "and created none of it: the index was there before this branch was",
        );
    }

    /// Which of the roadmaps this branch has written to declares badly, in the
    /// judgement's own words and naming the roadmap it is about.
    ///
    /// The undeclared one is no fault — that is every roadmap written before any
    /// of this — and a directory under `docs/roadmaps/` with no index in it is
    /// nothing to judge, exactly as it is nothing to pin.
    #[test]
    fn a_roadmap_declaring_badly_is_named_and_a_directory_is_not() {
        let repo = Repo::with(&[]);

        repo.write("mvp", MVP);
        repo.brief("notes", "01-something.md", "# not a roadmap at all\n");

        assert_eq!(
            misdeclared(repo.path(), &repo.touched()),
            None,
            "a roadmap declaring nothing runs in order, and a directory is not a roadmap",
        );

        repo.write(
            "parallel-stages",
            "# Parallel stages\n\n\
             - [ ] 01: Reading — [brief](01-reading.md) — after 02\n\
             - [ ] 02: Scheduling — [brief](02-scheduling.md) — after 01\n",
        );

        let why = misdeclared(repo.path(), &repo.touched()).expect("a cycle is refused");

        assert!(why.contains("parallel-stages roadmap"), "{why}");
        assert!(why.contains("01 stands on 02"), "{why}");
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

    /// And the same mechanism read the way adoption has to read it: out of a git
    /// directory at a commit, there being no Worktree anywhere in that path.
    ///
    /// A file that is only in the working tree is not one a commit holds, which
    /// is the whole difference between the two readings and worth asserting: the
    /// stage adoption starts works against the commit, not against whatever the
    /// human's checkout happens to be showing.
    #[test]
    fn the_stacking_mechanism_reads_out_of_a_commit_too() {
        let repo = Repo::with(&[]);
        repo.workflow(
            "# Git workflow\n\n## Review process\n\n\
             ### Finish sequence\n\nPush it.\n\n\
             ### Stacking roadmap stages\n\n`gh stack init <predecessor> <new>`\n",
        );

        assert!(
            !stacks_at(repo.path(), &repo.tip()),
            "written and not committed is nothing the commit holds",
        );

        repo.commit();

        assert!(
            stacks_at(repo.path(), &repo.tip()),
            "and committed, it is what the commit records",
        );
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
            "roadmaps/mvp/03-implementation",
            "adopting it takes the stage's own name, as the unattended start does",
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
    fn a_stage_whose_own_branch_is_taken_is_not_abandoned() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        assert_eq!(repo.abandoned().len(), 1, "nothing is on it yet");

        repo.branch("roadmaps/mvp/03-implementation");

        assert!(
            repo.abandoned().is_empty(),
            "`roadmaps/mvp/03-implementation` is taken, so stage 03 is under way somewhere",
        );
    }

    /// And clause 4 under the name the scheme gave a stage before `roadmaps/`
    /// went in front of it. A stage started last week is on a branch of that
    /// shape and nothing has renamed it, so a reading that asked only about the
    /// new name would offer a stage somebody is already working — and the
    /// unattended start would start it a second time.
    #[test]
    fn a_stage_taken_under_its_former_name_is_taken_still() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        repo.branch("mvp/03-implementation");

        assert!(
            repo.abandoned().is_empty(),
            "`mvp/03-implementation` is what stage 03 was called, and it is taken",
        );
        assert_eq!(repo.startable("mvp"), Startable::BranchTaken);
    }

    /// What is left of the collision this scheme is shaped around: a branch
    /// standing where a component of the stage's own path goes. Git keeps a
    /// branch as a file under `refs/heads/`, so it will not make
    /// `roadmaps/mvp/03-implementation` while `roadmaps` is a file — and what
    /// it refuses with goes to the server log rather than to the human.
    ///
    /// So the refusal names the branch. Nothing else in the reading can: the
    /// roadmap is fine, the brief is there, and the stage's own name is free.
    #[test]
    fn a_branch_in_the_stage_branchs_way_is_refused_by_name() {
        for blocker in ["roadmaps", "roadmaps/mvp"] {
            let repo = Repo::with(&[("mvp", MVP)]);
            repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
            repo.commit();

            assert_eq!(repo.abandoned().len(), 1, "nothing is in its way yet");

            repo.branch(blocker);

            assert_eq!(
                repo.startable("mvp"),
                Startable::BranchInTheWay {
                    by: blocker.to_owned(),
                },
                "`{blocker}` is a file where git needs a directory",
            );
            assert!(
                repo.abandoned().is_empty(),
                "so there is nothing to offer either: {blocker}",
            );
        }
    }

    /// And the whole point of the scheme: the roadmap's own Conversation branch
    /// is named for the roadmap, which is what a grilling session calls it, and
    /// it stays in the repository after the Conversation closes. Under the old
    /// shape that branch blocked every stage the roadmap planned, for good.
    #[test]
    fn a_roadmaps_own_branch_does_not_block_its_stages() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        repo.branch("mvp");

        let abandoned = repo.abandoned();

        assert_eq!(abandoned.len(), 1, "`mvp` is the roadmap's own branch");
        assert_eq!(
            abandoned[0].stage.branch(),
            "roadmaps/mvp/03-implementation",
            "which git will make while `mvp` is there, the two sharing no path",
        );
    }

    /// And what the roadmap's own name is in front of it for. A repository is
    /// full of branches somebody named for whatever they were doing, and one of
    /// them reading like a stage's brief says nothing whatever about that stage.
    /// Clause 4 turning on it would take the roadmap out of the notice and the
    /// unattended start would refuse it too — leaving a roadmap that could only
    /// be got going again by renaming a branch by hand.
    #[test]
    fn a_branch_that_merely_reads_like_a_stage_is_nothing_to_do_with_it() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        repo.branch("implementation");

        assert_eq!(
            repo.abandoned().len(),
            1,
            "`implementation` is somebody's own branch, not stage 03 of the mvp roadmap",
        );
        assert!(
            matches!(repo.startable("mvp"), Startable::Stage(_)),
            "so the press has a stage to start: {:?}",
            repo.startable("mvp"),
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
        repo.branch("roadmaps/mvp/03-implementation");

        assert_eq!(repo.startable("mvp"), Startable::BranchTaken);
    }

    /// And a roadmap that declares badly is a clause of its own, carrying the
    /// judgement's own sentence for the press to say.
    ///
    /// Not [`Startable::NoRoadmap`], which is what it used to be folded into:
    /// the roadmap is there and readable, and what is wrong with it is a line the
    /// human can go and fix. The notice and the page are unmoved either way —
    /// both keep only [`Startable::stage`], and there is none — so what this
    /// clause is for is the one path with somebody waiting on the answer.
    #[test]
    fn a_roadmap_that_declares_badly_refuses_with_the_fault_named() {
        let repo = Repo::with(&[(
            "mvp",
            "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n",
        )]);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        let startable = repo.startable("mvp");

        let Startable::Misdeclared { why } = &startable else {
            panic!(
                "a declaration on one line and not the other refuses the whole roadmap: {startable:?}"
            );
        };

        assert!(
            why.contains("mvp") && why.contains("02"),
            "which roadmap, and which line to go and read: {why:?}",
        );
        assert!(
            startable.clone().stage().is_none(),
            "so the notice and the page offer nothing off it either",
        );

        // And the same roadmap with the other line declaring too is a roadmap
        // something can run: nothing here is refusing a declaration for being one.
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) — after 01\n",
        );
        repo.commit();

        assert_eq!(
            repo.startable("mvp")
                .stage()
                .expect("stage 01 stands on nothing, so it may start")
                .stage
                .label,
            "01",
        );
    }

    /// And the carry-on's reading of the same roadmap starts nothing, with the
    /// judgement's sentence to leave on the Timeline.
    ///
    /// The same words at both ends of the refusal, because they are the one
    /// judgement's: a roadmap the human meets refused at the press and refused
    /// again on a Timeline should read as one fault rather than two.
    #[test]
    fn a_roadmap_that_declares_badly_starts_nothing_when_a_stage_settles() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n",
        );
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");

        let Next::Unstartable { why } = repo.next("mvp", "roadmaps/mvp/01-workbench") else {
            panic!("nothing of a roadmap that declares badly may start, here as anywhere");
        };

        assert!(
            why.contains("mvp") && why.contains("02"),
            "which roadmap, and which line to go and read: {why:?}",
        );
    }

    /// Which is what keeps a stage currently mid-flight under Verkstead out of
    /// the list. Everything the stage writes on its own line rides on its own
    /// branch until its pull request merges — the annotation its plan commit puts
    /// there, and the tick its finish commit puts there later — so the
    /// default-tip read sees a roadmap that still has stage 03 open, and the
    /// branch is the only thing saying otherwise.
    #[test]
    fn what_a_stage_wrote_on_its_own_branch_is_invisible_at_the_default_tip() {
        let repo = Repo::with(&[("mvp", MVP)]);
        repo.brief("mvp", "03-implementation.md", "# 03. Implementation\n");
        repo.commit();

        // The stage starts: a branch at its own name, and a plan commit on it
        // saying whose the stage is. Its box stays unticked until its own finish
        // commit, which rides on this branch too.
        repo.write(
            "mvp",
            &MVP.replace(
                "- [ ] 03: Implementation — [brief](03-implementation.md)",
                "- [ ] 03: Implementation — [brief](03-implementation.md) \
                 *(in progress: `implementation`)*",
            ),
        );
        repo.commit_on(
            "roadmaps/mvp/03-implementation",
            "chore: plan the implementation stage",
        );

        assert_eq!(
            at(repo.path(), &repo.tip(), "docs/roadmaps/mvp/ROADMAP.md",),
            Some(MVP.to_owned()),
            "the default branch has none of what the stage wrote",
        );
        assert!(
            repo.abandoned().is_empty(),
            "and the branch is what says the stage is already under way",
        );

        // The branch goes and nothing is running it, which is exactly the state
        // adoption is for.
        run(
            repo.path(),
            &["branch", "-D", "roadmaps/mvp/03-implementation"],
        );

        assert_eq!(repo.abandoned().len(), 1);
    }

    /// The whole of what changes visibly here. Stage 01 settled and nobody has
    /// merged it, so the tick is on that branch and the default branch's tip has
    /// nothing of it — and the branch holding it is the branch stage 01 was worked
    /// on, which is still there.
    ///
    /// Off the boxes alone the reading finds stage 01 unticked, finds its branch in
    /// the Repo and refuses: the roadmap that most needs carrying on is the one
    /// that offers nothing. With the record it offers stage 02, and the notice and
    /// the press name the same one.
    ///
    /// And the notice names stage 01's branch as the base. Both bases offer stage
    /// 02 — the default tip by the record, and the branch by its own ticked box —
    /// and only the branch holds stage 01's commits, so the reading that is behind
    /// gives way to the one that is not.
    #[test]
    fn a_stage_settled_on_an_unmerged_branch_leaves_the_stage_after_it_to_adopt() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        // Stage 01 is worked: a branch of its own, and its finish commit ticking
        // its own box riding on that branch until the pull request merges.
        repo.write(
            "mvp",
            &UNTICKED.replace("- [ ] 01: Workbench", "- [x] 01: Workbench"),
        );
        repo.commit_on("roadmaps/mvp/01-workbench", "chore: finish the workbench");

        assert_eq!(
            repo.startable("mvp"),
            Startable::BranchTaken,
            "off the boxes alone stage 01 is open and its branch is taken, \
             so the roadmap offers nothing at all",
        );

        let record = record([("mvp", "01", store::StageStanding::Settled)]);

        assert_eq!(
            repo.startable_with("mvp", &record)
                .stage()
                .expect("stage 01 settled, so stage 02 is the one to adopt")
                .stage
                .label,
            "02",
            "which is what the press starts",
        );

        assert_eq!(
            repo.abandoned_with(&record)
                .iter()
                .map(|abandoned| (abandoned.stage.label.as_str(), abandoned.behind))
                .collect::<Vec<_>>(),
            [("02", true)],
            "and this reading is behind: stage 01 is done by the record alone here, \
             its tick and its work both being on the branch below",
        );

        assert_eq!(
            notice(&repo.registered(), &record, AT_ONCE)
                .expect("the Repo is holding a roadmap with a stage to adopt")
                .roadmaps
                .iter()
                .map(|roadmap| (
                    roadmap.name.as_str(),
                    roadmap.stage.as_str(),
                    roadmap.base.as_str(),
                ))
                .collect::<Vec<_>>(),
            [("mvp", "02", "roadmaps/mvp/01-workbench")],
            "so the notice draws stage 02 off the branch holding stage 01's work \
             rather than off the default tip, which does not hold it: both bases \
             offer the stage and only one of them can build on what it stands on",
        );
    }

    /// A stage the record says is in flight is refused as in flight — by the
    /// record, rather than by an annotation naming a branch that still exists.
    ///
    /// Which is the refusal that was unreachable here before: a stage Verkstead
    /// started is on a branch of its own from the moment it starts, and the
    /// annotation saying whose it is rides on that branch with it, so at the
    /// default branch's tip the only thing that knew was clause 4. The press told
    /// the human their branch was taken where what was true is that somebody is on
    /// the stage.
    #[test]
    fn a_stage_the_record_says_is_in_flight_is_refused_as_in_flight() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.commit();

        let on_it = record([("mvp", "01", store::StageStanding::InFlight)]);

        assert_eq!(
            repo.startable_with("mvp", &on_it),
            Startable::InFlight,
            "somebody is on stage 01, and the press says so by name",
        );
        assert_eq!(
            notice(&repo.registered(), &on_it, AT_ONCE),
            None,
            "so the Repo is holding nothing to adopt",
        );

        // And the annotation is still the fallback for a stage the record says
        // nothing about, a record holding rows for other stages included: every
        // stage started between ADR-0017 and the label being written down is one of
        // those, and the annotation naming its branch is all there is to go on.
        repo.write(
            "mvp",
            &UNTICKED.replace(
                "- [ ] 02: Grilling — [brief](02-grilling.md)",
                "- [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `somebody-elses`)*",
            ),
        );
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();
        repo.branch("somebody-elses");

        assert_eq!(
            repo.startable_with(
                "mvp",
                &record([("mvp", "01", store::StageStanding::Settled)])
            ),
            Startable::InFlight,
            "stage 01 settled, and stage 02 is annotated on a branch that is there",
        );
    }

    /// And what the adoption offers of a declaring roadmap is the lowest stage that
    /// **may start**, which is not the lowest unticked box: a stage somebody is on
    /// no longer takes the whole roadmap out of the notice, because a stage standing
    /// on nothing beside it may still start.
    ///
    /// The same reading the carry-on goes through, which is what keeps the two
    /// saying one thing about one roadmap.
    #[test]
    fn the_adoption_offers_the_lowest_stage_of_a_declaring_roadmap_that_may_start() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        let Startable::Stage(abandoned) = repo.startable_with(
            "mvp",
            &record([("mvp", "01", store::StageStanding::InFlight)]),
        ) else {
            panic!("stage 02 stands on nothing, so somebody being on 01 does not hold it up");
        };

        assert_eq!(abandoned.stage.label, "02");
        assert!(
            abandoned.beside.is_empty(),
            "and nothing beside it: 01 is somebody's, and 03 and 04 stand on what \
             has not settled",
        );

        // And with both roots taken there is nothing to offer: 03 stands on 02 and
        // 04 on 01 and 03, so nothing may start until one of the two settles —
        // which is the roadmap holding its breath rather than the roadmap complete.
        assert_eq!(
            repo.startable_with(
                "mvp",
                &record([
                    ("mvp", "01", store::StageStanding::InFlight),
                    ("mvp", "02", store::StageStanding::InFlight),
                ]),
            ),
            Startable::InFlight,
        );
    }

    /// And where a declaring roadmap has two stages ready it offers **both**: the
    /// press starts every ready stage there is a place for, so the notice and the
    /// page name every ready stage there is a place for.
    ///
    /// The lowest is the one the Conversation doing the pressing becomes, which is
    /// why it is kept apart from the rest — and the rest are in the roadmap's own
    /// order behind it.
    #[test]
    fn the_adoption_offers_every_ready_stage_of_a_declaring_roadmap() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        let Startable::Stage(abandoned) = repo.startable("mvp") else {
            panic!("01 and 02 stand on nothing, so both of them may start");
        };

        assert_eq!(abandoned.stage.label, "01");
        assert_eq!(
            abandoned
                .beside
                .iter()
                .map(|stage| stage.label.as_str())
                .collect::<Vec<_>>(),
            ["02"],
            "and 02 starts beside it, 03 and 04 standing on what has not settled",
        );

        // The brief is read for each of them, the same way it is read for the
        // lowest: what starts beside the first is started from its own document.
        assert_eq!(abandoned.beside[0].brief, "# 02. Grilling\n");
        assert_eq!(
            abandoned.beside[0].brief_path,
            "docs/roadmaps/mvp/02-grilling.md",
        );

        // And the notice says so too, this being one reading behind both.
        assert_eq!(
            repo.waiting()
                .iter()
                .map(|roadmap| (
                    roadmap.stage.clone(),
                    roadmap
                        .beside
                        .iter()
                        .map(|stage| stage.label.clone())
                        .collect::<Vec<_>>(),
                ))
                .collect::<Vec<_>>(),
            [("01".to_owned(), vec!["02".to_owned()])],
        );
    }

    /// And only as many of them as the roadmap has **places**: the press starts
    /// every ready stage up to `at_once.roadmap_stages`, so the row names every
    /// ready stage up to it.
    ///
    /// A roadmap run one at a time offers its lowest and nothing beside it, which
    /// is what the setting is for — and a roadmap whose every place is held by a
    /// stage somebody is on offers nothing at all, however many of its lines are
    /// ready.
    #[test]
    fn the_adoption_offers_no_more_stages_than_the_roadmap_has_places() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        let Startable::Stage(one_at_a_time) =
            repo.startable_running("mvp", &store::StageStandings::default(), 1)
        else {
            panic!("01 may start whatever the limit is");
        };

        assert_eq!(one_at_a_time.stage.label, "01");
        assert!(
            one_at_a_time.beside.is_empty(),
            "one place, and the stage the Conversation becomes has taken it",
        );

        // And a stage somebody is on is holding one of them, so a roadmap run two
        // at a time with 01 under way has one place left and offers 02 alone.
        let Startable::Stage(one_left) = repo.startable_running(
            "mvp",
            &record([("mvp", "01", store::StageStanding::InFlight)]),
            2,
        ) else {
            panic!("02 stands on nothing, so it may start beside whoever is on 01");
        };

        assert_eq!(one_left.stage.label, "02");
        assert!(one_left.beside.is_empty());

        // And with that one place taken too there is nothing to offer: what this
        // roadmap is waiting for is one of them to settle, which is the same answer
        // as a roadmap with nothing ready.
        assert_eq!(
            repo.startable_running(
                "mvp",
                &record([("mvp", "01", store::StageStanding::InFlight)]),
                1,
            ),
            Startable::InFlight,
        );
    }

    /// A stage of the rest that cannot be started is left out rather than refusing
    /// the press: what the row offers is what pressing does, and a sibling whose
    /// brief nobody has written yet is no reason to withhold the stage beside it.
    ///
    /// The lowest is not treated that way — it is what the Conversation itself
    /// becomes, so what is wrong with it is said by name, which is what the four
    /// clauses above are for.
    #[test]
    fn a_stage_beside_the_lowest_that_cannot_start_is_left_out_of_the_offer() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.commit();

        let Startable::Stage(abandoned) = repo.startable("mvp") else {
            panic!("stage 01's own brief is there, so the press has something to start");
        };

        assert_eq!(abandoned.stage.label, "01");
        assert!(
            abandoned.beside.is_empty(),
            "02 is ready and names a brief nobody wrote, so it is not offered — \
             and 01 is offered all the same",
        );

        // And a branch somebody is already on takes it out of the offer the same
        // way, that being the clause the lowest is refused by name for.
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();
        repo.branch("roadmaps/mvp/02-grilling");

        let Startable::Stage(taken) = repo.startable("mvp") else {
            panic!("stage 01's own branch is free");
        };

        assert!(taken.beside.is_empty());
    }

    /// And the place a stage left out would have had goes to the ready stage
    /// behind it, which is [`next_stage`]'s rule read off the other reading.
    ///
    /// Four roots and three places, with the brief of the second one nobody wrote.
    /// Taking the places before the clauses rather than after them would offer two
    /// of the three the roadmap is allowed and leave 04 unoffered with a place
    /// standing free — so the press would start fewer stages than the settle that
    /// stands in for it would.
    #[test]
    fn a_stage_left_out_of_the_offer_leaves_its_place_to_the_next() {
        let repo = Repo::with(&[]);
        repo.write("mvp", FOUR_ROOTS);

        for (label, slug) in [
            ("01", "workbench"),
            ("03", "implementation"),
            ("04", "wrap-up"),
        ] {
            repo.brief(
                "mvp",
                &format!("{label}-{slug}.md"),
                &format!("# {label}. {slug}\n"),
            );
        }

        repo.commit();

        let Startable::Stage(abandoned) = repo.startable("mvp") else {
            panic!("stage 01's own brief is there, so the press has something to start");
        };

        assert_eq!(abandoned.stage.label, "01");
        assert_eq!(
            abandoned
                .beside
                .iter()
                .map(|stage| stage.label.as_str())
                .collect::<Vec<_>>(),
            ["03", "04"],
            "02 names a brief nobody wrote and is left out, and the place it would \
             have had goes to 04",
        );
    }

    /// A stage whose Conversation was closed before it ever wrapped up did not
    /// settle, and what it left behind is a branch: that branch is what refuses it,
    /// exactly as it did before there was a record. Reopening abandoned work is
    /// nobody's business here.
    #[test]
    fn an_abandoned_stage_is_still_refused_by_the_branch_it_left_behind() {
        let repo = Repo::with(&[]);
        repo.write("mvp", UNTICKED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.commit();
        repo.branch("roadmaps/mvp/01-workbench");

        let record = record([("mvp", "01", store::StageStanding::Abandoned)]);

        assert_eq!(
            repo.startable_with("mvp", &record),
            Startable::BranchTaken,
            "the record says it did not settle, and its branch is still there",
        );

        // And with that branch gone there is nothing to refuse it: an abandoned
        // stage is left to whatever its box says, which is exactly how the boxes
        // alone answered for it.
        run(repo.path(), &["branch", "-D", "roadmaps/mvp/01-workbench"]);

        assert_eq!(
            repo.startable_with("mvp", &record)
                .stage()
                .expect("nothing is on stage 01 any more")
                .stage
                .label,
            "01",
        );
    }

    /// And the annotation still speaks for an abandoned stage, which is the one
    /// way an unwanted row is silenced in the repository other than the box.
    ///
    /// The record saying a Conversation was closed part-way through says nothing
    /// about whether anybody is on the stage now — so somebody carrying that work
    /// on by hand, on a branch of their own, says so where the score is kept, and
    /// it is heeded exactly as it is for a stage the record has never heard of.
    #[test]
    fn an_abandoned_stage_somebody_annotated_is_left_to_them() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            &UNTICKED.replace(
                "- [ ] 01: Workbench — [brief](01-workbench.md)",
                "- [ ] 01: Workbench — [brief](01-workbench.md) *(in progress: `tobi/workbench`)*",
            ),
        );
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.commit();
        repo.branch("tobi/workbench");

        // The stage's own branch is gone — the attempt Verkstead abandoned left
        // nothing standing — so clause 4 has nothing to refuse it by.
        assert_eq!(
            repo.startable_with(
                "mvp",
                &record([("mvp", "01", store::StageStanding::Abandoned)])
            ),
            Startable::InFlight,
            "somebody is on it on a branch of their own, and the line says so",
        );
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

        let notice = notice_of(&store::Repo {
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

    /// A roadmap staged on a branch whose pull request is still open is on that
    /// branch and nowhere else, so the default tip cannot see it — and it is
    /// exactly the effort somebody wants carried on. It comes back named with
    /// the branch it was found on, which is what the press fixes the base to.
    #[test]
    fn a_roadmap_on_an_unmerged_branch_is_waiting_too() {
        let repo = Repo::with(&[]);
        repo.write(
            "missing-roles",
            "# Missing roles roadmap\n\n\
             - [x] 01: Project config — [brief](01-project-config.md)\n\
             - [ ] 02: Grant filters — [brief](02-grant-filters.md)\n",
        );
        repo.brief("missing-roles", "02-grant-filters.md", "# 02.\n");
        repo.commit_on(
            "tobi/missing-roles",
            "docs: stage the missing-roles roadmap",
        );

        let waiting = repo.waiting();

        assert_eq!(
            waiting
                .iter()
                .map(|roadmap| (
                    roadmap.name.as_str(),
                    roadmap.stage.as_str(),
                    roadmap.base.as_str(),
                ))
                .collect::<Vec<_>>(),
            [("missing-roles", "02", "tobi/missing-roles")],
            "found on the branch that holds it, and named with it",
        );
    }

    /// And once that branch merges the default tip holds it, which is the
    /// cheaper reading and the one that needs no base fixed. The branch is not
    /// read at all then — merged is the whole of what keeps this affordable —
    /// so the row that survives is the default's.
    #[test]
    fn a_roadmap_the_default_branch_has_swallowed_needs_no_base() {
        let repo = Repo::with(&[]);
        repo.write(
            "missing-roles",
            "# Missing roles roadmap\n\n- [ ] 01: Project config — [brief](01-config.md)\n",
        );
        repo.brief("missing-roles", "01-config.md", "# 01.\n");
        repo.commit();

        // The same commits, on a branch of its own as well as on the default.
        repo.branch("tobi/missing-roles");

        let waiting = repo.waiting();

        assert_eq!(waiting.len(), 1, "one roadmap, drawn once: {waiting:?}");
        assert_eq!(waiting[0].name, "missing-roles");
        assert_eq!(
            waiting[0].base, "",
            "off the default branch, which is the base a Conversation takes anyway",
        );
    }

    /// Two branches offering the same stage of the same roadmap is one piece of
    /// work, and it is drawn once. The default branch's reading is the one kept:
    /// it is the base that needs no fixing.
    #[test]
    fn the_same_stage_offered_twice_is_drawn_once() {
        let repo = Repo::with(&[]);
        repo.write(
            "mvp",
            "# MVP roadmap\n\n- [ ] 01: Workbench — [brief](01-workbench.md)\n",
        );
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.commit();

        // A branch that has moved on from the default tip, carrying the same
        // roadmap with the same stage left to start.
        std::fs::write(repo.path().join("elsewhere.md"), "# something else\n").unwrap();
        repo.commit_on("somebody/else", "chore: something else entirely");

        let waiting = repo.waiting();

        assert_eq!(waiting.len(), 1, "one stage, one row: {waiting:?}");
        assert_eq!(
            waiting[0].base, "",
            "and the default branch's reading of it"
        );
    }

    #[test]
    fn a_repo_with_nothing_to_adopt_has_no_notice() {
        let repo = Repo::with(&[(
            "mvp",
            "# MVP roadmap\n\n- [x] 01: Workbench — [brief](01-workbench.md)\n",
        )]);

        assert_eq!(
            notice_of(&store::Repo {
                id: 1,
                path: repo.path().to_owned(),
                name: "verkstead".to_owned(),
                default_branch: "main".to_owned(),
            }),
            None,
        );
    }

    /// The notice reads the default branch as origin holds it, so a roadmap
    /// somebody pushed is offered on a checkout that has never pulled it.
    ///
    /// Which is what the page and the press read too — offering a roadmap here
    /// and judging it against another ref there would be a button that refused
    /// itself. No fetch of its own: this is drawn for every registered Repo on
    /// every read of the sidebar, and origin's copy as the last fetch left it is
    /// what the two of them freshen before they act.
    #[test]
    fn a_notice_reads_the_default_branch_as_origin_holds_it() {
        let repo = Repo::with(&[]);

        // The roadmap is committed on origin and nowhere else: this checkout's
        // own `main` has never held it.
        let elsewhere = tempfile::tempdir().unwrap();
        let upstream = elsewhere.path().join("upstream");

        run(
            elsewhere.path(),
            &[
                "clone",
                &repo.path().to_string_lossy(),
                &upstream.to_string_lossy(),
            ],
        );
        run(
            &upstream,
            &["config", "user.email", "test@verkstead.invalid"],
        );
        run(&upstream, &["config", "user.name", "Verkstead Test"]);

        write(&upstream, "mvp", MVP);
        std::fs::write(
            upstream.join(ROADMAPS).join("mvp/03-implementation.md"),
            "# 03. Implementation\n",
        )
        .unwrap();
        run(&upstream, &["add", "-A"]);
        run(&upstream, &["commit", "-m", "docs: the roadmap"]);

        run(
            repo.path(),
            &["remote", "add", "origin", &upstream.to_string_lossy()],
        );
        run(repo.path(), &["fetch", "--quiet", "origin"]);

        let notice = notice_of(&store::Repo {
            id: 1,
            path: repo.path().to_owned(),
            name: "verkstead".to_owned(),
            default_branch: "main".to_owned(),
        })
        .expect("origin holds a roadmap with a stage left to start");

        assert_eq!(
            notice
                .roadmaps
                .iter()
                .map(|roadmap| (roadmap.name.as_str(), roadmap.stage.as_str()))
                .collect::<Vec<_>>(),
            [("mvp", "03")],
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
            notice_of(&store::Repo {
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

        let store = tempfile::tempdir().unwrap();
        let pool = empty_store(&store).await;

        let view = adopting(&pool, repo.registered(), None, "mvp".to_owned(), AT_ONCE).await;

        assert_eq!(view.roadmap, "mvp");
        assert_eq!(view.title, "MVP roadmap");

        let stage = view.stage.expect("that stage is startable");
        assert_eq!(stage.label, "03");
        assert_eq!(stage.title, "Implementation");
        assert_eq!(stage.brief_path, "docs/roadmaps/mvp/03-implementation.md");
        assert_eq!(
            stage.branch, "roadmaps/mvp/03-implementation",
            "the stage's own name, as the unattended start names one",
        );
        assert!(
            view.beside.is_empty(),
            "and nothing beside it: an undeclared roadmap runs in order, \
             so it has one stage ready at a time",
        );
    }

    /// And a declaring roadmap's page names every stage the press would start,
    /// rather than the lowest of them.
    ///
    /// The page and the notice are one reading, so what the pane says the press
    /// would do is what pressing does — and the stage the pressed Conversation
    /// itself becomes is the one kept apart from the rest.
    #[tokio::test]
    async fn an_adoption_page_names_every_stage_the_press_would_start() {
        let repo = Repo::with(&[]);
        repo.write("mvp", DECLARED);
        repo.brief("mvp", "01-workbench.md", "# 01. Workbench\n");
        repo.brief("mvp", "02-grilling.md", "# 02. Grilling\n");
        repo.commit();

        let store = tempfile::tempdir().unwrap();
        let pool = empty_store(&store).await;

        let view = adopting(&pool, repo.registered(), None, "mvp".to_owned(), AT_ONCE).await;

        assert_eq!(
            view.stage.expect("stage 01 stands on nothing").label,
            "01",
            "the stage this Conversation becomes",
        );
        assert_eq!(
            view.beside
                .iter()
                .map(|stage| (stage.label.as_str(), stage.branch.as_str()))
                .collect::<Vec<_>>(),
            [("02", "roadmaps/mvp/02-grilling")],
            "and the one that starts beside it, on a branch of its own",
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

        let store = tempfile::tempdir().unwrap();
        let pool = empty_store(&store).await;

        let at_tip = adopting(&pool, repo.registered(), None, "mvp".to_owned(), AT_ONCE).await;
        assert_eq!(
            at_tip.stage.expect("that stage is startable").label,
            "04",
            "with no override, the default branch's tip is what is read",
        );

        let earlier = adopting(
            &pool,
            repo.registered(),
            Some(before),
            "mvp".to_owned(),
            AT_ONCE,
        )
        .await;
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

        let store = tempfile::tempdir().unwrap();
        let pool = empty_store(&store).await;

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

            let view = adopting(&pool, repo.registered(), base, roadmap.to_owned(), AT_ONCE).await;

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
        repo.branch("roadmaps/mvp/03-implementation");

        let store = tempfile::tempdir().unwrap();
        let pool = empty_store(&store).await;

        let view = adopting(&pool, repo.registered(), None, "mvp".to_owned(), AT_ONCE).await;

        assert_eq!(view.roadmap, "mvp");
        assert_eq!(view.stage, None);
    }
}
