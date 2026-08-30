//! Starting the next stage of a roadmap, with nobody asked.
//!
//! This is the piece that makes the whole pipeline unattended rather than merely
//! gateless. A wrap-up settles — see [`crate::settling`] — and the stage after
//! the one that settled starts: a Conversation of its own, on a branch of its
//! own, in a session inside the bundled fork of next-stage. That fork writes
//! `.tasks/`, at which point the runner takes over and works the backlog to
//! empty, which finishes and opens a pull request, which wraps up, which starts
//! the stage after it. Nothing in the loop asks for permission.
//!
//! **One thing stops it**, and it stops it naturally: the fork's breakdown quiz
//! is a blocking ask, so the stage waits there and its Conversation carries
//! *blocked on you* until the human answers from wherever they are.
//!
//! **A stage is a Conversation of its own** rather than the old one carrying on.
//! A Conversation is one Repo, one branch and one Worktree, and a stage is one
//! branch and one review unit — so it cannot be anything else. It is created
//! against the same Repo, under the same Profiles, primed with the stage brief as
//! its Brief, and it goes straight to Implementing: the grilling that would have
//! settled the work already happened, and the brief is what it settled.
//!
//! **What is read is the Worktree**, by the same rule the pinned stage list is
//! drawn by — see [`crate::stages`] — so the list the human is watching and the
//! stage that starts next cannot come to disagree.
//!
//! **What is decided is where the branch goes**, and only that. Stages stack on
//! the unmerged predecessor where the target repository records how, and
//! Verkstead reads whether that block is there rather than carrying a stacking
//! mechanism of its own: the session follows what the block says, because the
//! mechanism is the repository's. Where there is no block there is no convention
//! to invent, so the branch comes off the default branch and the Timeline says
//! so.
//!
//! **And the companions come across with it.** A stage is given everything a
//! human would have settled before pressing anything, and the parent
//! Conversation's companion repos belong in that list for the same reason the
//! Pairings do: a stage has no draft moment of its own, so there is nowhere else
//! the set could come from. Read-only ones come across as they are and are
//! checked out detached at whatever their base resolves to now; read-write ones
//! cut a branch of their own per stage, named after the stage's branch, and
//! standing on the predecessor stage's companion branch wherever the stage's own
//! branch stands on the predecessor's. Every one of them is checked out in the
//! same act as the stage's own worktree and recorded with it — and a companion
//! that cannot be delivered starts nothing, the way everything else that stops a
//! stage stops it.
//!
//! Nothing here is refused for and nothing is returned. It runs at the end of an
//! unattended run with nobody watching, and what it has to say it says on the
//! Timeline as a notice — which is what a decision taken while nobody was looking
//! is owed. Where the roadmap itself moved on — a stage started, or the last one
//! finished — the devices are told as well, because a notice on a Timeline
//! nobody has open reaches nobody at all.

use std::path::{Path, PathBuf};

use verkstead_schema::Nudge;

use crate::AppState;
use crate::stages::{self, Next, Stage};
use crate::store;
use crate::worktrees;

/// Start the stage after `conversation_id`'s, where there is one.
///
/// Called when a wrap-up settles, on every Conversation rather than on the ones
/// somebody thought were roadmap stages: whether this is a stage of anything is
/// read off the branch, and a Conversation whose branch has written to no roadmap
/// quietly is not one.
pub(crate) async fn carry_on(state: AppState, conversation_id: i64) {
    let Some(conversation) = load(&state, conversation_id).await else {
        return;
    };

    let (Some(worktree), Some(base)) = (
        conversation.worktree.clone(),
        conversation.base_commit.clone(),
    ) else {
        // No Worktree is a closed Conversation, and no base commit is one that
        // never started grilling. Neither can have written to a roadmap.
        return;
    };

    let branch = conversation.branch.clone();

    // Both readings together, off the runtime's threads: a git read and a
    // handful of file reads.
    let read = tokio::task::spawn_blocking({
        let worktree = worktree.clone();
        move || {
            (
                stages::next_stage(&worktree, &base, &branch),
                stages::stacks(&worktree),
            )
        }
    })
    .await;

    let (next, stacks) = match read {
        Ok(read) => read,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a roadmap had left failed");
            return;
        }
    };

    let stage = match next {
        Next::Stage(stage) => *stage,
        // The ordinary Conversation: an inline run or a feature's backlog, whose
        // branch touched no roadmap. Nothing to carry on and nothing to say —
        // there was never a roadmap for the human to wonder about.
        Next::NoRoadmap => return,
        Next::Complete { roadmap } => {
            tracing::info!(
                conversation_id,
                roadmap,
                "every stage of the roadmap is done, so nothing was started",
            );

            say(
                &state,
                conversation_id,
                &format!(
                    "Every stage of the `{roadmap}` roadmap is done, so there is no stage to \
                     start. The roadmap is complete."
                ),
            )
            .await;

            // A stage completing is a milestone, and this is the last one
            // completing: there is no stage after it to be announced by, so the
            // roadmap running out is what the devices are told instead.
            crate::push::told(
                &state.pool,
                conversation_id,
                crate::push::News::RoadmapComplete { roadmap },
            );

            return;
        }
        Next::Unstartable { why } => {
            tracing::warn!(conversation_id, why, "the next stage could not be started");

            return say(
                &state,
                conversation_id,
                &format!("The next stage of the roadmap could not be started: {why}."),
            )
            .await;
        }
    };

    start(&state, &conversation, conversation_id, stage, stacks).await;
}

/// Start `stage` as a Conversation of its own against the same Repo.
///
/// The order is the order [`crate::conversations::start_grilling`] does the same
/// job in, and for the same reason: git makes the branch and the worktree, and
/// only then does the store hear about it. A row saying a stage is under way with
/// nothing checked out would be a Conversation nothing could run and nothing
/// would clean up.
///
/// Everything that stops it stops it with a notice on the Timeline of the
/// Conversation that has just settled. That is where the human is looking — the
/// stage that would have carried it on does not exist, so there is no Timeline of
/// its own to say anything on.
async fn start(
    state: &AppState,
    conversation: &store::Conversation,
    settled: i64,
    stage: Stage,
    stacks: bool,
) {
    let branch = stage.branch();
    let repo = conversation.repo.path.clone();

    // Every Profile is the predecessor's: a stage is the same work by the same
    // hands, one branch further on. The implementation one is what the session
    // runs under, and without it there is nothing to run.
    if conversation.implementation_pairing.is_none() {
        return say(
            state,
            settled,
            &format!(
                "Stage {} of the `{}` roadmap is next, and this Conversation's implementation \
                 Profile has gone, so there is no account to run it under. Nothing was started.",
                stage.label, stage.roadmap,
            ),
        )
        .await;
    }

    // A branch by that name already is a stage somebody — or some earlier run —
    // has started already. Refused rather than worked around: the alternative is
    // a second Conversation quietly doing a stage that is already under way, on a
    // branch named after neither of them.
    if taken(&repo, &branch).await {
        return say(
            state,
            settled,
            &format!(
                "Stage {} of the `{}` roadmap is next, and `{branch}` is already a branch of \
                 this repository — so it looks to have been started already. Nothing was \
                 started.",
                stage.label, stage.roadmap,
            ),
        )
        .await;
    }

    // Where the branch goes, which is the whole of what Verkstead decides about
    // stacking. The predecessor's branch is the one this stage's work builds on,
    // and its tip is where the branch starts.
    let stacked_on = stacks.then(|| conversation.branch.clone());

    let from = match stacked_on.clone() {
        // A stacked stage stands on the predecessor's branch, which is work on
        // this machine and nowhere else: there is no remote copy of it to be
        // behind, so there is nothing a fetch could freshen.
        Some(predecessor) => predecessor,

        // An unstacked one comes off the default branch, and what that means is
        // what origin is holding rather than wherever this checkout's copy of it
        // was last left — so the remote-tracking refs are made current before
        // anything reads them.
        None => match fresh(&repo, &conversation.repo.default_branch).await {
            Some(from) => from,

            // Nobody is at a button to refuse with: this runs at the end of an
            // unattended run, so a fetch git would not make halts the stage with
            // a notice naming it. Halted rather than carried on with, because a
            // stage branched off refs nobody can vouch for is a whole stage of
            // work starting from the wrong place.
            None => {
                return say(
                    state,
                    settled,
                    &format!(
                        "Stage {} of the `{}` roadmap is next, and git would not fetch from \
                         this repository's remote — so what its branch would come off cannot \
                         be trusted to be what origin is holding. Nothing was started, and \
                         the server log says why the fetch failed.",
                        stage.label, stage.roadmap,
                    ),
                )
                .await;
            }
        },
    };

    let started = store::start_conversation(&state.pool, conversation.repo.id, &branch).await;

    let id = match started {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::error!(
                settled,
                "the Repo the stage would be against has gone, so nothing was started"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, settled, "starting the next stage's Conversation failed");
            return;
        }
    };

    // Everything the human would have settled before pressing anything: who it
    // runs as, what it is about, and which repositories it works alongside. All
    // of it while it is still drafting, which is the only state any of them can
    // be recorded in.
    if let Err(error) = settle(state, id, conversation, &stage).await {
        tracing::error!(error = ?error, settled, stage = id, "preparing the next stage's Conversation failed");

        say(
            state,
            settled,
            &format!(
                "Stage {} of the `{}` roadmap could not be given everything it inherits from \
                 this Conversation: {error}. Nothing was started.",
                stage.label, stage.roadmap,
            ),
        )
        .await;

        return gave_up(state, id).await;
    }

    let path = worktrees::worktree_path(&state.data_dir, id, &conversation.repo.name, &branch);

    // Every checkout the stage needs, asked of git before any of them is made:
    // the branch off `from`, and one per companion it has just inherited. The
    // whole list is planned first for the reason a grill start plans one — that
    // is what lets a stage that cannot be given one companion start with nothing
    // left behind anywhere.

    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        let branch = branch.clone();
        let from = from.clone();
        let data = state.data_dir.clone();
        let companions = conversation.companions.clone();
        let predecessor = stacked_on.clone();
        let checkouts = state.checkouts.clone();

        move || {
            let Some(commit) = worktrees::resolve(&repo, &from) else {
                return Err(Halted::Own);
            };

            let mut planned = vec![Checkout {
                companion: None,
                repo,
                path,
                branch: Some(branch.clone()),
                commit: commit.clone(),
            }];

            for companion in companions {
                // Whatever has been planned so far: until they exist the
                // filesystem cannot tell two directories apart, so two
                // companions coming off one branch name would otherwise be
                // handed the same one. See [`worktrees::unclaimed_path`].
                let claimed: Vec<PathBuf> = planned
                    .iter()
                    .map(|checkout| checkout.path.clone())
                    .collect();

                planned.push(beside(
                    &data,
                    id,
                    &branch,
                    predecessor.as_deref(),
                    companion,
                    &claimed,
                )?);
            }

            // And only now, held from the first directory this makes to the
            // record naming it, as a grill start holds it and for its reason: a
            // directory made and not yet recorded is one the sweep of orphaned
            // worktrees would read as nobody's. Here rather than around the
            // whole of this, because [`beside`] fetches once per companion and
            // a fetch has no deadline to answer within. See
            // [`crate::AppState::checkouts`].
            let making = checkouts.blocking_lock_owned();

            make(&planned)?;

            Ok((commit, recorded(&planned), making))
        }
    })
    .await;

    let (commit, checkouts, making) = match made {
        Ok(Ok(made)) => made,
        Ok(Err(halted)) => {
            say(state, settled, &halted.said(&stage, &branch, &from)).await;

            return gave_up(state, id).await;
        }
        Err(error) => {
            tracing::error!(error = ?error, settled, stage = id, "making the next stage's worktrees failed");
            return gave_up(state, id).await;
        }
    };

    match store::start_stage(
        &state.pool,
        id,
        &commit,
        &path,
        stacked_on.as_deref(),
        &checkouts,
    )
    .await
    {
        Ok(store::Staged::Started) => {}
        Ok(refused) => {
            tracing::error!(
                settled,
                stage = id,
                refused = ?refused,
                "the next stage's Conversation could not be set working",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, settled, stage = id, "recording the next stage failed");
            return;
        }
    }

    // Recorded, so the sweep would keep them. What follows says so on two
    // Timelines and launches a session, and none of it makes a directory.
    drop(making);

    // What Verkstead decided, on both Timelines: on the stage's, because the
    // branch it is on was nobody's choice but this; and on the settled one,
    // because that is where the human was watching when it happened.
    say(
        state,
        id,
        &begun(&stage, &branch, stacked_on.as_deref(), &from),
    )
    .await;
    say(
        state,
        settled,
        &format!(
            "Stage {} of the `{}` roadmap — *{}* — has started as a Conversation of its own, \
             on `{branch}`.",
            stage.label, stage.roadmap, stage.title,
        ),
    )
    .await;

    tracing::info!(
        settled,
        stage = id,
        branch,
        label = stage.label,
        roadmap = stage.roadmap,
        "the next stage of the roadmap has started",
    );

    // And the devices, because a roadmap moving on is the milestone the human
    // would otherwise find out about by opening the sidebar: the stage before
    // this one is complete and this one is already running, and none of it was
    // asked for. Told about the stage that started rather than the one that
    // settled — that is where the work is now, so that is what tapping it opens.
    crate::push::told(
        &state.pool,
        id,
        crate::push::News::StageStarted {
            label: stage.label.clone(),
            roadmap: stage.roadmap.clone(),
        },
    );

    // Both Timelines were said by the notices above, each on its own. What is
    // left to say is the sidebar: there is a Conversation in the list that was
    // not there a moment ago, and an open page should show it without being
    // reloaded.
    state.nudges.announce(Nudge::Conversations);

    // Taken here rather than by the planning, which is started from more than one
    // place and takes the registration from all of them — see
    // [`crate::runner::plan_stage`].
    let driving = state.drivers.driving(id);

    tokio::spawn(crate::runner::plan_stage(
        state.clone(),
        id,
        stacked_on,
        driving,
    ));
}

/// What the stage's own Timeline is told: which stage it is, and where its
/// branch came from.
///
/// Both halves said plainly, including the half that is an absence — a stage off
/// the default branch because the repository records no way to stack one is a
/// decision, and one the human may want to do something about.
fn begun(stage: &Stage, branch: &str, stacked_on: Option<&str>, from: &str) -> String {
    // The brief named rather than linked: it is a path in a Worktree the
    // workbench has no route to, and a link that went nowhere would be worse
    // than the path itself.
    let started = format!(
        "Stage {} of the `{}` roadmap — *{}* — started from `{}` with nobody asked.",
        stage.label, stage.roadmap, stage.title, stage.brief_path,
    );

    match stacked_on {
        Some(predecessor) => format!(
            "{started} Its branch `{branch}` stacks on `{predecessor}`, the branch of the stage \
             before it, the way this repository's `{}` records.",
            stages::GIT_WORKFLOW,
        ),
        None => format!(
            "{started} Its branch `{branch}` came off `{from}`: this repository's `{}` records \
             no way to stack a roadmap stage on the one before it, and there is no convention \
             to invent.",
            stages::GIT_WORKFLOW,
        ),
    }
}

/// One checkout a stage start is about to make: which repository, where it goes,
/// what it holds and what it came off.
///
/// The stage's own and each of its companions in the one shape, because from the
/// moment they are planned they are the same thing — a worktree of a registered
/// repository. What differs between them is two fields, and both of them read as
/// what they are: a companion is named, and a checkout that holds no branch is
/// detached.
struct Checkout {
    /// The companion Repo this is a checkout of — its id and what it is called —
    /// or `None` for the stage's own.
    ///
    /// The id is what the record is written against, and the name is what a
    /// notice says. Together they are the whole of what a companion's checkout
    /// needs that the stage's own does not.
    companion: Option<(i64, String)>,

    /// The repository the worktree is made from.
    repo: PathBuf,

    /// Where the checkout goes, under the Data Directory.
    path: PathBuf,

    /// The branch to cut, or `None` for a detached checkout — which is what a
    /// read-only companion gets, having nothing to commit and no business taking
    /// a name in somebody else's repository.
    branch: Option<String>,

    /// The commit it comes off.
    commit: String,
}

/// Why a stage was not started, once there was git to ask.
///
/// Every one of them halts the stage rather than carrying it on without: nobody
/// is at a button to be refused, and a stage that quietly built without a
/// repository the roadmap was grilled against is a worse outcome than a stage
/// that waited.
enum Halted {
    /// The stage's own checkout: git would not say what its branch comes off, or
    /// would not make the worktree.
    ///
    /// One case rather than two, as it always was: what the human does about
    /// either is look at the repository the branch was going into.
    Own,

    /// One of the companions it inherits, named — because *which one* is the
    /// whole of what the human needs.
    Companion { repo: String, why: Why },
}

/// What git would not do for a companion, in the order it is asked: the three
/// [`beside`] asks before anything is made, and then the making itself.
enum Why {
    FetchFailed,
    NoBaseCommit,
    BranchExists,
    WorktreeRefused,
}

impl Halted {
    /// The notice this goes on the settled Conversation's Timeline as.
    ///
    /// Which repository and what git would not do, and then that nothing was
    /// started and nothing was left behind — because the human reading it is
    /// deciding whether there is a half-made stage somewhere to go and tidy, and
    /// there never is.
    fn said(&self, stage: &Stage, branch: &str, from: &str) -> String {
        let start = format!("Stage {} of the `{}` roadmap", stage.label, stage.roadmap);

        match self {
            Self::Own => format!(
                "{start} could not be given a worktree on `{branch}` off `{from}`. Nothing was \
                 started.",
            ),
            Self::Companion { repo, why } => format!(
                "{start} works alongside `{repo}`, the companion repository it inherits from \
                 this Conversation, and {}. Nothing was started, and nothing it had begun to \
                 check out was left behind.",
                why.said(),
            ),
        }
    }
}

impl Why {
    /// The clause that goes after the repository's name.
    fn said(&self) -> &'static str {
        match self {
            Self::FetchFailed => {
                "git would not fetch from that repository's remote — so what its checkout would \
                 come off cannot be trusted to be what origin is holding, and the server log \
                 says why the fetch failed"
            }
            Self::NoBaseCommit => "what its checkout comes off resolves to no commit there",
            Self::BranchExists => {
                "the branch this stage would cut in it is already a branch of that repository"
            }
            Self::WorktreeRefused => "git would not make its checkout",
        }
    }
}

/// Ask git everything one inherited companion's checkout turns on, and come back
/// with what it will be.
///
/// **A read-write companion is cut a branch named after the stage's own**,
/// whatever the predecessor's row said — see [`settle`], which is where the
/// typed name is dropped. Where the stage's own branch stands on the
/// predecessor's, this branch stands on the predecessor's companion branch in
/// the same repository: that is where the work it builds on is, the predecessor
/// having committed in it and its pull request there being unmerged for just as
/// long. Which is `predecessor`'s whole job — the settled Conversation's branch
/// where the stage stacks, and `None` where it does not.
///
/// A branch on this machine and nowhere else has no remote copy to be behind, so
/// a stacked companion asks for no fetch, exactly as a stacked stage's own
/// branch does not.
///
/// **Everything else fetches, then resolves, then checks the branch** — the
/// grill start's order, for the grill start's reasons. An unstacked stage's
/// read-write companion comes off the base its row names, and a read-only one is
/// detached at whatever that base comes to at this moment, that being the only
/// commit anything will ever be able to name it by. A companion whose repository
/// has no remote has nothing to fetch and is never halted for it.
///
/// Nothing is made here. What comes back is a plan, and the making waits until
/// every checkout has one: that is what lets a stage that cannot be given one
/// companion start without having checked out another.
fn beside(
    data: &Path,
    id: i64,
    branch: &str,
    predecessor: Option<&str>,
    companion: store::Companion,
    claimed: &[PathBuf],
) -> Result<Checkout, Halted> {
    let repo = companion.repo.path.clone();
    let halted = |why| Halted::Companion {
        repo: companion.repo.name.clone(),
        why,
    };

    // The row this stage will hold is the predecessor's with the branch name
    // taken off, so what it is called resolves through the mirroring rule rather
    // than being assigned here — one place for that rule, and it is
    // [`store::Companion::branch_for`].
    let cut = store::Companion {
        branch: String::new(),
        ..companion.clone()
    }
    .branch_for(branch);

    // And the predecessor's own name in this repository, resolved the same way
    // against the branch the settled Conversation was worked on. `None` on a
    // read-only companion and on an unstacked stage, both of which come off the
    // configured base instead.
    let stands_on = predecessor.and_then(|predecessor| companion.branch_for(predecessor));

    let named = match &stands_on {
        Some(stands_on) => stands_on.clone(),
        None => {
            if let worktrees::Fetched::Failed(said) = worktrees::fetch(&repo) {
                tracing::error!(
                    said,
                    repo = %repo.display(),
                    "fetching a companion Repo's remotes failed, so the next stage is not being started",
                );

                return Err(halted(Why::FetchFailed));
            }

            // The branch of that repository's own the human picked while
            // drafting, or its default branch as origin holds it — the rule the
            // stage's own base follows, asked of the companion's repository.
            match companion.base_ref.clone() {
                Some(picked) => picked,
                None => worktrees::default_ref(&repo, &companion.repo.default_branch),
            }
        }
    };

    let Some(commit) = worktrees::resolve(&repo, &named) else {
        return Err(halted(Why::NoBaseCommit));
    };

    // A name already taken in that repository is somebody else's work — this
    // stage has never been started, so nothing of its own can be holding it.
    if let Some(cut) = &cut
        && worktrees::branch_exists(&repo, cut)
    {
        return Err(halted(Why::BranchExists));
    }

    // Named for the Repo and what the checkout holds, as the stage's own is: the
    // branch where there is one, and otherwise the base it stands at — a
    // read-only companion holds no branch to be named for.
    let holds = cut.clone().unwrap_or(named);

    Ok(Checkout {
        companion: Some((companion.repo.id, companion.repo.name.clone())),
        path: worktrees::unclaimed_path(data, id, &companion.repo.name, &holds, claimed),
        repo,
        branch: cut,
        commit,
    })
}

/// Make every checkout of a start, or unmake the ones already made and say which
/// one would not be.
///
/// The one place a stage start creates anything, which is what makes *nothing
/// left behind* something to hold rather than something to hope for. What is
/// unwound is directory and branch together — see [`worktrees::unmake`] —
/// because a branch cut moments ago by a start that then halted holds nothing
/// worth keeping.
fn make(planned: &[Checkout]) -> Result<(), Halted> {
    for (nth, checkout) in planned.iter().enumerate() {
        let made = match &checkout.branch {
            Some(branch) => {
                worktrees::add(&checkout.repo, &checkout.path, branch, &checkout.commit)
            }
            None => worktrees::add_detached(&checkout.repo, &checkout.path, &checkout.commit),
        };

        if made {
            continue;
        }

        // This one included, and first: an `add` that fell over may have made
        // the directory, or the branch, or neither, and what is being unwound is
        // whatever it did get as far as. The rest newest first, which is the
        // order they were made in reversed.
        for done in planned[..=nth].iter().rev() {
            worktrees::unmake(&done.repo, &done.path, done.branch.as_deref());
        }

        return Err(match &checkout.companion {
            Some((_, repo)) => Halted::Companion {
                repo: repo.clone(),
                why: Why::WorktreeRefused,
            },
            None => Halted::Own,
        });
    }

    Ok(())
}

/// Where each companion of a start was checked out and what it came off, for the
/// record that follows the work.
///
/// The commit as well as the directory, because a companion's base is a *name*
/// on its row and a name moves: a read-only companion is detached at whatever
/// that name came to at this moment, and this is the only thing that will ever
/// know which commit that was.
///
/// The stage's own is not among them: it goes on the row the store has always
/// kept for it, one per Conversation.
fn recorded(planned: &[Checkout]) -> Vec<store::CompanionWorktree> {
    planned
        .iter()
        .filter_map(|checkout| {
            let (repo_id, _) = checkout.companion.as_ref()?;

            Some(store::CompanionWorktree {
                repo_id: *repo_id,
                path: checkout.path.clone(),
                base_commit: Some(checkout.commit.clone()),
            })
        })
        .collect()
}

/// Give the new Conversation everything a human would have settled before
/// pressing anything: the Pairings, the stage brief as its Brief, and the
/// companion repos the work goes on alongside.
///
/// The one inheritance funnel, which is why the companions are here rather than
/// beside it: a stage has no draft moment of its own, so everything it would
/// have been set up with has to arrive in the one act, and a set copied
/// somewhere else would be a second place for it to be forgotten.
///
/// The Pairings are the predecessor's, every one of them. The implementation one
/// is what the work runs under, the review one is what looks at what it built,
/// and the grilling one is carried across because a stage
/// steered into a second round later is grilled by whatever the roadmap's work
/// has been grilled by all along.
///
/// Carried whole, model and all — and a predecessor whose Profile was chosen
/// before pairings existed carries no model, which leaves this stage running on
/// the same Profile's own model, exactly as its predecessor did.
///
/// **The companions come across in the mode they were in**, off the same three
/// presses the setup card makes, this Conversation being a Draft with no branch
/// made — which is the only state any of them is allowed in. Every one of the
/// predecessor's, because a stage of a roadmap grilled against a repository is
/// the work that roadmap settled and needs what it was settled against.
///
/// **What does not come across is a typed branch name.** A companion branch
/// somebody named while drafting is the roadmap Conversation's own, and two
/// stages sharing one companion branch would be two review units on one branch
/// with two pull requests fighting over it. So the row is left mirroring, and
/// the stage's own branch is what its companion branches are called.
async fn settle(
    state: &AppState,
    id: i64,
    conversation: &store::Conversation,
    stage: &Stage,
) -> anyhow::Result<()> {
    // Whichever of the three the predecessor picked, the rows that run no
    // session included: a stage inherits what its roadmap was settled with, and
    // *not grilled* and *not reviewed* are as much settled choices as an
    // account.
    match &conversation.grilling_pairing {
        store::Picked::Nothing => {}
        store::Picked::Skipped => {
            store::skip_grilling(&state.pool, id).await?;
        }
        store::Picked::Under(grilling) => {
            store::set_grilling_pairing(
                &state.pool,
                id,
                grilling.profile.id,
                grilling.model.as_deref(),
            )
            .await?;
        }
    }

    if let Some(implementation) = &conversation.implementation_pairing {
        store::set_implementation_pairing(
            &state.pool,
            id,
            implementation.profile.id,
            implementation.model.as_deref(),
        )
        .await?;
    }

    match &conversation.review_pairing {
        store::Picked::Nothing => {}
        store::Picked::Skipped => {
            store::skip_review(&state.pool, id).await?;
        }
        store::Picked::Under(review) => {
            store::set_review_pairing(&state.pool, id, review.profile.id, review.model.as_deref())
                .await?;
        }
    }

    store::save_brief(&state.pool, id, &stage.brief).await?;

    for companion in &conversation.companions {
        let named = &companion.repo.name;

        // Added the way the card adds one — read-only, on the default-branch
        // rule and mirroring — and then moved to what the predecessor's row
        // says. A Repo that has left the registry between the predecessor being
        // read and this write is the one thing that can refuse it, and it is
        // said with the repository named: a stage quietly built without a
        // repository the roadmap was grilled against is the worse outcome.
        let added = store::add_companion(&state.pool, id, companion.repo.id).await?;

        if added != store::Adding::Added {
            anyhow::bail!("`{named}` could not be put on it ({added:?})");
        }

        if companion.mode == store::CompanionMode::ReadWrite {
            configured(
                store::configure_companion(
                    &state.pool,
                    id,
                    companion.repo.id,
                    store::Change::Mode(store::CompanionMode::ReadWrite),
                )
                .await?,
                named,
            )?;
        }

        // The base as the human picked it, which is what an unstacked stage's
        // checkout comes off and what a read-only one is detached at. `None` is
        // the default-branch rule, which is what a fresh row already holds.
        if let Some(base) = companion.base_ref.as_deref() {
            configured(
                store::configure_companion(
                    &state.pool,
                    id,
                    companion.repo.id,
                    store::Change::Base(Some(base)),
                )
                .await?,
                named,
            )?;
        }
    }

    Ok(())
}

/// What a press on an inherited companion's row came to, with the repository
/// named where it came to nothing.
///
/// Only ever a race — the stage is drafting and the row was written a moment
/// ago — but a refusal swallowed here would leave a stage running against a
/// companion in the wrong mode, which is a session writing where it was never
/// given leave to.
fn configured(configured: store::Configured, repo: &str) -> anyhow::Result<()> {
    match configured {
        store::Configured::Saved => Ok(()),
        refused => anyhow::bail!("`{repo}` could not be set up on it ({refused:?})"),
    }
}

/// Stop the half-made Conversation, where a stage got as far as a record and no
/// further.
///
/// Closed rather than left drafting, because drafting is a Conversation waiting
/// for a human to write a Brief and press a button — and this one is a stage
/// nobody is going to start by hand. Closing is the work stopping wherever it
/// was, which is exactly what happened.
///
/// Nothing is left checked out by the time this can run, so there is nothing to
/// clean up but the row: the stage's worktree and its companions' are made in
/// one act that unmakes whatever it managed before it halted — see [`make`].
async fn gave_up(state: &AppState, id: i64) {
    if let Err(error) = store::close_conversation(&state.pool, id).await {
        tracing::error!(error = ?error, conversation_id = id, "stopping a half-made stage failed");
    }
}

/// Whether `repo` already has a branch by that name.
async fn taken(repo: &Path, branch: &str) -> bool {
    let repo = repo.to_owned();
    let branch = branch.to_owned();

    tokio::task::spawn_blocking(move || worktrees::branch_exists(&repo, &branch))
        .await
        .unwrap_or_else(|error| {
            tracing::error!(error = ?error, "asking whether a branch was taken failed");
            // Reads as taken, which is the right way round for the one thing it
            // decides: what is on the other side of it is making a branch and
            // letting an agent loose on it.
            true
        })
}

/// The name an unstacked stage's branch comes off, with `repo`'s
/// remote-tracking refs made current first — or `None` where git would not
/// fetch.
///
/// The rule a grilling starts by, applied where nobody pressed anything: the
/// default branch means what origin is holding, and a remote-tracking ref is
/// only ever as fresh as the last fetch. Without this a stage comes off
/// wherever the human's own copy of the default branch was last left, which on
/// a machine that has not pulled for a week is a week of other people's work
/// missing from every stage after it.
///
/// A repository with no remote has nothing to fetch and nothing to be stale
/// against, so it comes off its own default branch and is never refused for it.
/// A join that failed says nothing either way, and nothing either way is not
/// permission to branch: it reads as a fetch that failed, which is the same way
/// round [`taken`] falls.
async fn fresh(repo: &Path, default: &str) -> Option<String> {
    let repo = repo.to_owned();
    let default = default.to_owned();

    let read = tokio::task::spawn_blocking(move || {
        if let worktrees::Fetched::Failed(said) = worktrees::fetch(&repo) {
            tracing::error!(
                said,
                repo = %repo.display(),
                "fetching a Repo's remotes failed, so the next stage is not being started",
            );

            return None;
        }

        Some(worktrees::default_ref(&repo, &default))
    })
    .await;

    read.unwrap_or_else(|error| {
        tracing::error!(error = ?error, "fetching before a stage started failed");
        None
    })
}

/// Put a notice on a Timeline.
///
/// Nothing is refused for: by the time anything here has something to say, what
/// it is saying has already happened. A notice that could not be written is a
/// line in the log and no more.
async fn say(state: &AppState, conversation_id: i64, markdown: &str) {
    match store::note(&state.pool, conversation_id, markdown).await {
        Ok(true) => state.nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => tracing::error!(
            conversation_id,
            "there is no Conversation left to say anything on"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "putting a notice on a Timeline failed")
        }
    }
}

/// The Conversation whose wrap-up has settled, or `None` where there is nothing
/// left to read.
async fn load(state: &AppState, conversation_id: i64) -> Option<store::Conversation> {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(conversation) => conversation,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation that settled failed");
            None
        }
    }
}
