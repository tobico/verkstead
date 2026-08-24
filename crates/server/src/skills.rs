//! The skills Verkstead ships, and how a session comes to be running one.
//!
//! A session's behaviour should be the product's rather than whatever happens
//! to be installed on the machine it runs on. So Verkstead carries its own
//! skills, embedded in the binary as the viewer is (ADR-0004), writes them out
//! under the Data Directory at startup, and every sandbox binds that directory
//! read-only over `~/.claude/skills`. Nothing beside the executable has to be
//! there, and nothing of the host's is bound in for a session to find — the
//! checkout of the skills the host keeps for its own agents is not reachable at
//! all.
//!
//! An account's own skills are hidden by the bind rather than merged with:
//! Verkstead's fork is what a Conversation is grilled by, and a Profile is an
//! account to run as rather than a second opinion about how to work.
//!
//! Installing a skill is not invoking one, and the sandbox has no global
//! `CLAUDE.md` to say what a session is for — the host's is not bound in, and
//! the Profile's `~/.claude` is the human's rather than Verkstead's to write.
//! What sends a session into the grilling skill is therefore [`grilling`]: the
//! prompt it is started on names the skill by the path it is mounted at, above
//! the Brief itself. That is also why the skill carries the ask instruction in
//! its own text — the twelve lines it was forked from say to interview the
//! human and never say how, because on a workstation the global `CLAUDE.md`
//! said it instead.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rust_embed::Embed;

/// The skills as they are written in this repository, one directory per skill.
///
/// Compiled in for a release build and read off disk for a debug one, exactly
/// as the viewer is and for the same reason: editing a skill is then visible to
/// a running `cargo run -p verkstead-cli -- serve` without a recompile.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/skills"]
struct Bundled;

/// Where the skills are mounted inside a sandbox, under whatever HOME is there.
pub(crate) const INSIDE_HOME: &str = ".claude/skills";

/// And what that makes the grilling skill's path, as a session is told to find
/// it. Written with the tilde because that is how an agent reads a path in its
/// own home, and because HOME inside is the server's own rather than a fixed
/// name this could spell out.
const GRILLING: &str = "~/.claude/skills/grilling/SKILL.md";

/// The implementation skill's, the same way.
const IMPLEMENTING: &str = "~/.claude/skills/implementing/SKILL.md";

/// And the breakdown skill's — Verkstead's fork of to-tasks, which is what the
/// task-list direction runs instead of building anything itself.
const BREAKING_DOWN: &str = "~/.claude/skills/breaking-down/SKILL.md";

/// And the staging skill's — Verkstead's fork of to-roadmap, which is what the
/// roadmap direction runs instead of building anything itself.
const STAGING: &str = "~/.claude/skills/staging/SKILL.md";

/// And the fork of next-stage, which the one session a roadmap stage starts
/// with runs inside: the session that re-grounds the stage's brief and writes
/// the backlog the runner then works.
const NEXT_STAGE: &str = "~/.claude/skills/next-stage/SKILL.md";

/// And the fork of next-task, which every session the runner launches is put
/// inside — the task sessions and the finish one alike, because which of them it
/// is, is read off `.tasks/` rather than told.
const NEXT_TASK: &str = "~/.claude/skills/next-task/SKILL.md";

/// And the addressing skill's, which every fix session of a wrap-up runs
/// inside — whichever of the three kinds of feedback dispatched it.
const ADDRESSING: &str = "~/.claude/skills/addressing/SKILL.md";

/// And the reviewing skill's, which the one session a wrap-up starts with runs
/// inside: the fresh context that reads the branch none of the sessions that
/// wrote it ever saw.
const REVIEWING: &str = "~/.claude/skills/reviewing/SKILL.md";

/// And the responding skill's, which a batch of comments left on the pull
/// request after the review is answered inside: the review's propose-then-fix
/// shape again, about what somebody has just said rather than about the branch.
const RESPONDING: &str = "~/.claude/skills/responding/SKILL.md";

/// And the manual-task skill's, which the one-off session a human sets going by
/// hand runs inside — the one skill nothing in the pipeline ever launches.
const MANUAL_TASK: &str = "~/.claude/skills/manual-task/SKILL.md";

/// The bundled skills, installed on the host, ready for a sandbox to bind.
#[derive(Debug, Clone)]
pub struct Skills {
    path: PathBuf,
}

impl Skills {
    /// Write them out under `data_dir`, replacing whatever is already there.
    ///
    /// Replaced rather than written over, so that what a session finds is what
    /// this binary ships and not the union of that with every binary that ran
    /// here before: a skill withdrawn from the product should stop being a skill
    /// sessions are run under.
    ///
    /// Under the Data Directory because that is the one place Verkstead is
    /// given to write, and beside the worktrees for the same reason they are
    /// there: this is something Verkstead made rather than something the human
    /// pointed it at.
    ///
    /// Refused where the binary carries none. A grilling session with no
    /// grilling skill is a session that has been told to read a file that is not
    /// there, and a server that starts anyway would be one whose every
    /// Conversation fails at the far end of the button.
    pub fn installed(data_dir: &Path) -> Result<Skills> {
        let path = data_dir.join("skills");

        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("clearing the installed skills at {}", path.display())
                });
            }
        }

        let mut installed = 0;

        for name in Bundled::iter() {
            let file = Bundled::get(&name)
                .expect("a name the embedding just handed out is one it answers to");

            let written = path.join(name.as_ref());

            if let Some(parent) = written.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("making {}", parent.display()))?;
            }

            std::fs::write(&written, file.data)
                .with_context(|| format!("installing the skill {}", written.display()))?;

            installed += 1;
        }

        if installed == 0 {
            anyhow::bail!(
                "this binary carries no skills: a grilling session is started by naming one, \
                 so every Conversation would be handed a path to nothing"
            );
        }

        tracing::debug!(path = %path.display(), files = installed, "the bundled skills are installed");

        Ok(Skills { path })
    }

    /// Where they landed, which is what a sandbox binds.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// What a grilling session is started on: the Brief, under the line that sends
/// the agent into the bundled grilling skill.
///
/// Said in the prompt rather than left to the sandbox to configure, because the
/// prompt is the one thing a session is certain to read. The Brief goes last and
/// whole — it is the human's own markdown, and nothing here interprets it.
pub(crate) fn grilling(brief: &str) -> String {
    format!(
        "Read {GRILLING} and grill me about the Brief below, the way it says. \
         Nothing else in this session tells you how to reach me.\n\n{brief}"
    )
}

/// And what a grilling relaunched on a stalled Conversation is started on: the
/// same Brief, and under it whatever the human has already settled.
///
/// A grilling that died takes its interview with it — see [`crate::grillings`] —
/// so the relaunch is a grilling from the beginning, which is what the Brief
/// alone gives it. What the digest adds is the one part of an interview that
/// outlives the session having it: the Questions that were asked and the Answers
/// that came back.
///
/// Under the Brief, where the retry note goes and for its reason: the Brief says
/// what the work is, and this says what has already been decided about it — the
/// newer and the less general of the two, so it goes second. The note the human
/// wrote goes after both, being newer and less general still.
///
/// A Conversation with nothing answered yet is the Brief and nothing else. A
/// heading over an empty digest would tell the session that something had been
/// said.
pub(crate) fn grilling_again(brief: &str, settled: &str) -> String {
    let settled = settled.trim();

    if settled.is_empty() {
        return grilling(brief);
    }

    format!(
        "{}\n\n# What has already been asked, and what I said\n\n{settled}\n",
        grilling(brief).trim_end(),
    )
}

/// What an inline implementation session is started on: the two documents it is
/// building from, under the line that sends the agent into the implementation
/// skill.
///
/// Both documents, because they are different things. The Brief is the human's
/// own statement of the work and it is short; the handoff is what a whole
/// grilling settled about it, written by the session that did the grilling. A
/// session given only the second would be building from a summary of the first.
///
/// A grilling that ended without writing one leaves `None`, and the session is
/// primed with the Brief alone. Not a refusal: the work is still described, and
/// an implementation that never started because a document was missing would be
/// a Conversation stuck with nothing to press.
pub(crate) fn implementing(brief: &str, handoff: Option<&str>) -> String {
    on_the_documents(
        &format!("Read {IMPLEMENTING} and build the work described below, the way it says."),
        brief,
        handoff,
    )
}

/// What a retried task-list tail is started on: the Brief, under the line that
/// sends the agent into the breakdown skill.
///
/// One document rather than two, unlike the sessions that build. A task list
/// writes no handoff — the backlog *is* what the grilling settled, committed to
/// the branch — so there is never one for this to carry. The ordinary way into
/// the breakdown is the grilling session reading on with the whole thread still
/// in its context and no prompt sent at all; this is the retry, which grounds
/// itself in the Brief, the repository, and whatever the human wrote when they
/// asked for the tail to be run again.
pub(crate) fn breaking_down(brief: &str) -> String {
    on_the_documents(
        &format!(
            "Read {BREAKING_DOWN} and break the work described below into tasks, the way it says."
        ),
        brief,
        None,
    )
}

/// What a retried roadmap tail is started on: the Brief again, under the line
/// that sends the agent into the staging fork.
///
/// One document for the reason the breakdown gets one, one level up: the stage
/// briefs are what the grilling settled, and nothing crosses out of this
/// Conversation that has to be told anything else. The ordinary way in is the
/// grilling session reading on; this is the retry.
pub(crate) fn staging(brief: &str) -> String {
    on_the_documents(
        &format!(
            "Read {STAGING} and stage the work described below into a roadmap, the way it says."
        ),
        brief,
        None,
    )
}

/// What a stage's first session is started on: the stage brief, under the line
/// that sends the agent into the fork of next-stage — and the one sentence only
/// this kind of session gets, about where its branch came from.
///
/// One document rather than two, alone among these. A stage has no grilling of
/// its own and so no handoff: what a grilling would have settled was settled by
/// the grilling that wrote the roadmap, and the stage brief is what it settled.
/// It arrives as the Conversation's Brief, so it is primed exactly as every
/// other session is primed with one — see [`on_the_documents`].
///
/// `stacked_on` is the predecessor's branch where this stage's branch was made
/// on top of it, and `None` where it came off the default branch. Said because
/// it is the one thing about the stage the session cannot read out of the
/// repository: a branch says what it is descended from, not what somebody meant
/// by it, and what the session does about it — registering the stack the way the
/// repository records — turns on which of the two this is.
pub(crate) fn next_stage(brief: &str, stacked_on: Option<&str>) -> String {
    let prompt = on_the_documents(
        &format!("Read {NEXT_STAGE} and plan the roadmap stage described below, the way it says."),
        brief,
        None,
    );

    let branch = match stacked_on {
        Some(predecessor) => format!(
            "This stage's branch stacks on `{predecessor}`, the branch the stage before it \
             was worked on, which is not merged yet.",
        ),
        None => "This stage's branch came off the repository's default branch, so it is not \
                 stacked on anything."
            .to_owned(),
    };

    format!("{prompt}\n# Where this stage's branch came from\n\n{branch}\n")
}

/// What each session of a backlog is started on: the same two documents again,
/// under the line that sends the agent into the fork of next-task.
///
/// Which task is not said. The fork reads `.tasks/` and takes the lowest number
/// left, which is what the runner reads too — Verkstead decides the step in
/// order to know what to watch for, not to hand it over.
///
/// The documents rather than the task file alone, because a slice of the work is
/// still the work: the task file says where this session stops, and the two
/// documents say what it is a slice of. A session that had only the first would
/// be building to a description written for somebody who had read the other two.
pub(crate) fn next_task(brief: &str, handoff: Option<&str>) -> String {
    on_the_documents(
        &format!(
            "Read {NEXT_TASK} and work the next task of the backlog for the work described \
             below, the way it says."
        ),
        brief,
        handoff,
    )
}

/// What the review session is started on: the same two documents again, under
/// the line that sends the agent into the reviewing skill.
///
/// The documents rather than the branch alone, because a review is a reading of
/// work against what it was for: the diff says what was done and these two say
/// what was meant. What is deliberately *not* here is anything about how the
/// work was built — the tasks, the sessions, the order they ran in. This session
/// is the first thing to see the branch whole, and priming it with the shape the
/// work was cut into would be handing it the very frame the sessions that wrote
/// it were each stuck inside.
///
/// `said` is what was written on the pull request before this session started —
/// the comments whole, in the order they were said in, with where each was said.
/// It goes *last*, under the documents, where the newest and least general thing
/// goes in every other prompt here: the documents say what the work is, and this
/// says what somebody has already said about it. A pull request nobody has
/// written on carries none of it, rather than a heading saying nothing was said.
pub(crate) fn reviewing(brief: &str, handoff: Option<&str>, said: Option<&str>) -> String {
    let prompt = on_the_documents(
        &format!(
            "Read {REVIEWING} and review the branch this worktree is on, the way it says. The \
             work described below is what it was meant to be."
        ),
        brief,
        handoff,
    );

    match said {
        Some(said) => format!(
            "{prompt}\n# What has been said on the pull request\n\n{}\n",
            said.trim()
        ),
        None => prompt,
    }
}

/// What a batch session is started on: the comments it is about, under the two
/// documents and the line that sends the agent into the responding skill.
///
/// The same three pieces the review gets, in the same order and for the same
/// reasons — the documents say what the work is, and what was said goes last
/// because it is the newest and least general thing. What differs is which
/// comments and how many: the review is given everything standing on the pull
/// request when it starts, and this is given one batch of what was said after
/// it.
///
/// `said` is never empty here, unlike the review's. A batch session exists
/// because something was said, so there is no version of this prompt with
/// nothing under the heading.
pub(crate) fn responding(brief: &str, handoff: Option<&str>, said: &str) -> String {
    let prompt = on_the_documents(
        &format!(
            "Read {RESPONDING} and answer what has just been said on this branch's pull \
             request, the way it says. The work described below is what it was meant to be."
        ),
        brief,
        handoff,
    );

    format!(
        "{prompt}\n# What has just been said on the pull request\n\n{}\n",
        said.trim()
    )
}

/// What a fix session is started on: the feedback to address, under the two
/// documents the work was built from and the line that sends the agent into the
/// addressing skill.
///
/// One function for all three callers — a failed check, and the fixes either
/// proposal was answered with where the session that proposed them never landed
/// them — because they are one job: somebody has said that work already pushed
/// is not right yet, and what differs is the feedback rather than anything about
/// how to take it. Three prompts saying the same thing in three places is three
/// things to keep true.
///
/// The feedback goes *last*, under the documents rather than over them, for the
/// reason a retry note does: it is the newest thing said and the least general.
/// The documents say what the work is; this says what is wrong with it.
pub(crate) fn addressing(brief: &str, handoff: Option<&str>, feedback: &str) -> String {
    let prompt = on_the_documents(
        &format!(
            "Read {ADDRESSING} and address the feedback at the end of this prompt, the way it \
             says."
        ),
        brief,
        handoff,
    );

    format!(
        "{prompt}\n# The feedback to address\n\n{}\n",
        feedback.trim()
    )
}

/// What a Manual Task's session is started on: the instruction the human typed,
/// under the line that sends the agent into the manual-task skill.
///
/// The instruction and nothing else — alone among these, no Brief and no
/// handoff. A Manual Task is outside the pipeline in every sense: it is not a
/// slice of the work the two documents describe, and a session primed with them
/// would be one told that the thing it was asked to do was part of something
/// else. What the human typed is the whole of what they meant.
///
/// It goes last and whole, where every other prompt's last thing goes: it is
/// the human's own markdown, and nothing here interprets it.
pub(crate) fn manual_task(instruction: &str) -> String {
    format!(
        "Read {MANUAL_TASK} and do what I have asked for below, the way it says. \
         Nothing else in this session tells you how to reach me.\n\n\
         # What I have asked for\n\n{}\n",
        instruction.trim()
    )
}

/// The same prompt, with what the human said when they asked for the step to be
/// tried again.
///
/// Written under the documents rather than over them, because it is the newest
/// thing said and the least general: the Brief and the handoff describe the work,
/// and this describes what to do differently this time. "Try again but leave the
/// migration alone" is only worth writing if it reaches whatever can act on it,
/// and the prompt is the one thing a session is certain to read.
///
/// A retry with nothing written alongside is the prompt unchanged. The ordinary
/// remedy is a human who has read the evidence and thinks the step is worth
/// another run as it stands, and a heading over an empty note would be the
/// session told that something had been said.
pub(crate) fn retrying(prompt: &str, note: &str) -> String {
    let note = note.trim();

    if note.is_empty() {
        return prompt.to_owned();
    }

    format!("{prompt}\n# What I said when I asked you to try this again\n\n{note}\n")
}

/// The body they are all primed with, under whichever opening line names the
/// skill.
fn on_the_documents(opening: &str, brief: &str, handoff: Option<&str>) -> String {
    let mut prompt = format!(
        "{opening} Nothing else in this session tells you how to reach me.\n\n\
         # The Brief this started from\n\n{brief}\n"
    );

    if let Some(handoff) = handoff {
        prompt.push_str(&format!(
            "\n# What the grilling settled, in its own words\n\n{handoff}\n"
        ));
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the skill is read as, whichever way this build carries it.
    fn skill(name: &str) -> String {
        let file = Bundled::get(name).unwrap_or_else(|| panic!("{name} is one of the skills"));

        String::from_utf8(file.data.to_vec()).expect("a skill is markdown")
    }

    /// Where the shared commit-summary block starts, and the last line of it.
    /// Every skill that commits work carries the block word for word; what
    /// follows it differs from skill to skill, so the end is found by its own
    /// last line rather than by whatever section comes after.
    const SUMMARY_BLOCK: &str = "### What the message body says";
    const SUMMARY_BLOCK_END: &str = "the in-process throttle it replaces goes away.\n";

    /// The block as one skill carries it, cut out so that the five can be held
    /// against each other.
    fn summary_block(name: &str) -> String {
        let text = skill(name);
        let start = text
            .find(SUMMARY_BLOCK)
            .unwrap_or_else(|| panic!("{name} should ask for the commit's summary:\n{text}"));
        let rest = &text[start..];
        let end = rest
            .find(SUMMARY_BLOCK_END)
            .unwrap_or_else(|| panic!("{name} should carry the whole block:\n{rest}"))
            + SUMMARY_BLOCK_END.len();

        rest[..end].to_string()
    }

    /// The whole reason the fork exists: the twelve lines it came from say to
    /// interview the human and never say how, because on a workstation the
    /// global `CLAUDE.md` said it — and inside a sandbox there is no such file.
    #[test]
    fn the_grilling_skill_says_how_to_reach_the_human() {
        let grilling = skill("grilling/SKILL.md");

        assert!(
            grilling.contains("verkstead guide"),
            "the Guide is where an agent learns to ask, and it ships inside the binary"
        );
        assert!(
            grilling.contains("verkstead ask"),
            "and the ask is what actually reaches the human"
        );
        assert!(
            !grilling.contains("askance"),
            "the real askance stays installed on the host: the agent-facing surface in here \
             is Verkstead's own"
        );
    }

    /// A grilling ends by the agent's own closing move, so the skill has to say
    /// what that move is. Nothing else will tell it: there is no button that
    /// ends a grilling, and a session that never proposed wrapping up would grill
    /// until the human aborted it.
    #[test]
    fn the_grilling_skill_says_how_a_grilling_ends() {
        let grilling = skill("grilling/SKILL.md");

        assert!(
            grilling.contains("proposal:"),
            "the closing Set is marked by the block it carries, so the skill has to name it"
        );

        for direction in ["inline", "task-list", "roadmap"] {
            assert!(
                grilling.contains(direction),
                "the skill should name the {direction} direction, which is one of the three"
            );
        }

        assert!(
            grilling.contains("rationale"),
            "the chooser draws the agent's reasoning beside the choices, and a proposal \
             without one is refused"
        );
    }

    /// A pick lets the session proceed and never makes it, so the skill has to
    /// say what proceeding *is* and what the way back is.
    ///
    /// Nothing enforces this end of it. Verkstead watches for the picked
    /// direction's artifact and for nothing else, so a session that answered a
    /// pick by writing one of the other two would have decided the direction in
    /// the human's place — and the only thing standing between it and that is
    /// the skill saying so.
    #[test]
    fn the_grilling_skill_says_a_pick_is_argued_with_by_proposing_again() {
        let grilling = skill("grilling/SKILL.md");

        let (_, after) = grilling
            .split_once("### After they pick")
            .expect("what a pick means is a thing the skill carries");
        let (picked, _) = after
            .split_once("### When they pick inline")
            .expect("with the three branches after it");

        assert!(
            picked.contains("does not make you"),
            "a pick lets the session proceed and never makes it: {picked}"
        );
        assert!(
            picked.contains("propose again") || picked.contains("Propose again"),
            "and the way to argue with one is another proposal: {picked}"
        );
        assert!(
            picked.contains("never do"),
            "which leaves writing a different artifact as the thing it may not \
             do — the decision the chooser exists to take out of its hands: \
             {picked}"
        );
        assert!(
            !grilling.contains("on one Set and no others"),
            "and a proposal is no longer one-per-grilling: a refused round is \
             followed by another, and so is a pick that leaves something open: \
             {grilling}"
        );
    }

    /// A task-list pick does not end the grilling: the same session writes the
    /// backlog, holding everything the grilling settled. Nothing else tells it so
    /// — no second prompt is sent, because no second session is started — so the
    /// skill it is already inside has to carry the branch and name the skill it
    /// reads on into.
    #[test]
    fn the_grilling_skill_says_to_break_the_work_down_where_a_task_list_is_picked() {
        let grilling = skill("grilling/SKILL.md");

        assert!(
            grilling.contains(BREAKING_DOWN),
            "the branch is a skill to read, named by the path the sandbox mounts it at: \
             {grilling}"
        );
        assert!(
            grilling.contains("Do not start\ntask 01"),
            "the backlog is where this session stops: the tasks are the runner's, \
             a fresh session each: {grilling}"
        );
    }

    /// Nor does a roadmap pick, and for the same reason one level up: the stage
    /// briefs are worth what the context that settled them can put in them. Same
    /// branch to carry, and one thing more to say — this tail goes past the
    /// artifact to the pull request, where the breakdown's stops at the commit.
    #[test]
    fn the_grilling_skill_says_to_stage_the_work_where_a_roadmap_is_picked() {
        let grilling = skill("grilling/SKILL.md");

        assert!(
            grilling.contains(STAGING),
            "the branch is a skill to read, named by the path the sandbox mounts it at: \
             {grilling}"
        );
        assert!(
            grilling.contains("pull request"),
            "and this one carries the branch the rest of the way, which the session \
             has to know before it stops at the commit: {grilling}"
        );
        assert!(
            grilling.contains("Do not start stage 01"),
            "the roadmap is where this session stops: each stage is a Conversation of \
             its own, on a branch of its own: {grilling}"
        );
    }

    /// A session is pointed at the skill by the prompt, and nothing else in the
    /// sandbox says what it is for.
    #[test]
    fn a_grilling_session_is_started_by_naming_the_skill() {
        let prompt = grilling("# Rate limiting\n\nThe API has none.\n");

        assert!(
            prompt.contains(GRILLING),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.ends_with("# Rate limiting\n\nThe API has none.\n"),
            "and the Brief is what follows, whole: {prompt:?}"
        );
    }

    /// The handoff is the whole reason an inline implementation is a fresh
    /// session, so the skill that ends a grilling has to say where it goes —
    /// and it is the path the sandbox mounts, not one an agent may improvise.
    ///
    /// On the far side of the pick, which is the other half of it: an inline
    /// pick is what the handoff is written *for*, and one written before the
    /// closing Set would be handing over an understanding the human had not yet
    /// answered.
    #[test]
    fn the_grilling_skill_says_where_the_handoff_is_written() {
        let grilling = skill("grilling/SKILL.md");

        assert!(
            grilling.contains(crate::handoffs::HANDOFF_INSIDE),
            "the skill has to name the path Verkstead reads the handoff back from"
        );
        assert!(
            grilling.contains("handoff"),
            "and call it what the Timeline and the workbench call it"
        );

        let (before, after) = grilling
            .split_once("### When they pick inline")
            .expect("the inline pick is a branch the skill carries");

        assert!(
            !before.contains(crate::handoffs::HANDOFF_INSIDE),
            "nothing is written before the pick: a refused proposal would cost a \
             rewrite, and the Set is meant to be cheap to send — {before}"
        );
        assert!(
            after.contains(crate::handoffs::HANDOFF_INSIDE),
            "and the inline branch is where it is written: {after}"
        );
    }

    /// And the other two pick no handoff up at all. What they settle is the
    /// artifact they commit, so a handoff on either path would be a second
    /// record of the plan that nothing downstream reads.
    #[test]
    fn the_grilling_skill_writes_no_handoff_on_the_picks_that_commit_one() {
        let grilling = skill("grilling/SKILL.md");

        let (_, tails) = grilling
            .split_once("### When they pick a task list")
            .expect("the task-list pick is a branch the skill carries");

        let (task_list, rest) = tails
            .split_once("### When they pick a roadmap")
            .expect("and the roadmap pick is the one after it");
        let (roadmap, _) = rest
            .split_once("### When they don't accept")
            .expect("with the refusal branch after that");

        for (named, branch) in [("task list", task_list), ("roadmap", roadmap)] {
            assert!(
                !branch.contains(crate::handoffs::HANDOFF_INSIDE),
                "a {named} pick writes the plan into the repository and no handoff \
                 anywhere: {branch}"
            );
        }
    }

    /// No gate anywhere in the implementation: the agent commits on its own,
    /// and feedback consolidates when the branch is reviewed as a whole.
    #[test]
    fn the_implementation_skill_says_to_commit_without_asking() {
        let implementing = skill("implementing/SKILL.md");

        assert!(
            implementing.contains("Commit"),
            "committing is what an implementation session is for"
        );
        assert!(
            implementing.contains("Nothing waits on approval"),
            "and there is no gate in front of it: {implementing}"
        );
        assert!(
            implementing.contains("verkstead ask"),
            "the one way to the human, for the questions the handoff did not settle"
        );
    }

    /// What the breakdown produces is a `.tasks/` backlog in the repository, so
    /// the fork has to say what to write and where — Verkstead reads it back off
    /// the branch and owns none of it.
    #[test]
    fn the_breakdown_skill_says_what_a_task_list_is_made_of() {
        let breaking_down = skill("breaking-down/SKILL.md");

        for named in [".tasks/", "TODO.md", "NN-<slug>.md"] {
            assert!(
                breaking_down.contains(named),
                "the backlog is files in the repository, and {named} is one of them"
            );
        }

        assert!(
            breaking_down.contains("sequential"),
            "the order is the dependency, which is the one rule the breakdown has"
        );
    }

    /// The fork drops what a workstation-driven flow assumes and Verkstead
    /// supplies instead: the branch is already made, the feature is already
    /// chosen, and the plan commit is not something to ask permission for.
    #[test]
    fn the_breakdown_skill_drops_what_verkstead_already_supplies() {
        let breaking_down = skill("breaking-down/SKILL.md");

        assert!(
            breaking_down.contains("The branch is\nalready made"),
            "a session that made its own branch would leave the work off the \
             Conversation's: {breaking_down}"
        );
        assert!(
            breaking_down.contains("Nothing waits on approval"),
            "and the plan commit has no gate in front of it, as no commit here does"
        );
        assert!(
            !breaking_down.contains("/next-task") && !breaking_down.contains("/clear"),
            "nobody is at a terminal to run a slash command, and Verkstead runs the \
             backlog itself: {breaking_down}"
        );
    }

    /// Two sessions can be reading this: the grilling one carrying on from the
    /// pick, and a fresh one launched because that tail was retried. They differ
    /// in what the reader has to ground itself in — its own conversation, or the
    /// Brief and the repository — so the skill has to say both.
    #[test]
    fn the_breakdown_skill_works_from_both_ways_in() {
        let breaking_down = skill("breaking-down/SKILL.md");

        assert!(
            breaking_down.contains("the grilling session, reading on"),
            "the ordinary way in is the session that settled the work carrying on: \
             {breaking_down}"
        );
        assert!(
            breaking_down.contains("a fresh session"),
            "and the other is a retried tail: {breaking_down}"
        );
        assert!(
            breaking_down.contains("there is no handoff document"),
            "which is grounded in the Brief, the repository and the retry note — a \
             task list writes no handoff for it to have been handed: {breaking_down}"
        );
    }

    /// The breakdown quiz is the human's decision to make, and it reaches them
    /// the only way anything does.
    #[test]
    fn the_breakdown_skill_puts_its_quiz_through_the_cli() {
        let breaking_down = skill("breaking-down/SKILL.md");

        assert!(
            breaking_down.contains("verkstead guide") && breaking_down.contains("verkstead ask"),
            "the quiz is an ordinary Set, asked the way every Set is: {breaking_down}"
        );
        assert!(
            !breaking_down.contains("proposal:"),
            "and ordinary ones: the `proposal` block is the grilling's closing move, \
             and this runs after one was accepted"
        );
    }

    /// What the staging produces is a `docs/roadmaps/` roadmap in the
    /// repository, so the fork has to say what to write and where — Verkstead
    /// reads it back off the branch and owns none of it.
    #[test]
    fn the_staging_skill_says_what_a_roadmap_is_made_of() {
        let staging = skill("staging/SKILL.md");

        for named in ["docs/roadmaps/", "ROADMAP.md", "NN-<slug>.md"] {
            assert!(
                staging.contains(named),
                "the roadmap is files in the repository, and {named} is one of them"
            );
        }

        assert!(
            staging.contains("sequential"),
            "the order is the dependency here as it is in a backlog: {staging}"
        );
    }

    /// The formats are the repository's rather than Verkstead's: what this fork
    /// writes is read back by whoever starts a stage, and by a human who wrote
    /// their roadmap by hand. So the two headings the readers turn on — the
    /// `## Stages` checkbox list and the brief's sections — have to be the ones
    /// they already write.
    #[test]
    fn the_staging_skill_writes_the_formats_the_stage_readers_expect() {
        let staging = skill("staging/SKILL.md");

        assert!(
            staging.contains("## Stages") && staging.contains("- [ ] 01: <title> — [brief]("),
            "the checkbox list is what says how far the effort has got: {staging}"
        );

        for section in [
            "## Goal",
            "## Decisions in force",
            "## Proposed tasks (provisional)",
            "## Re-verify at start",
        ] {
            assert!(
                staging.contains(section),
                "a stage brief carries {section}, because starting the stage reads it"
            );
        }
    }

    /// The fork drops what a workstation-driven flow assumes and Verkstead
    /// supplies instead, and gains what only Verkstead's shape needs: the branch
    /// is already made, nobody approves the commit, and the stages are
    /// Conversations of their own rather than work to start here.
    #[test]
    fn the_staging_skill_drops_what_verkstead_already_supplies() {
        let staging = skill("staging/SKILL.md");

        assert!(
            staging.contains("The branch is\nalready made"),
            "a session that made its own branch would leave the work off the \
             Conversation's: {staging}"
        );
        assert!(
            staging.contains("Nothing waits on approval"),
            "and the roadmap commit has no gate in front of it, as no commit here does"
        );
        assert!(
            staging.contains("Do not start stage 01"),
            "each stage is a Conversation of its own, on a branch of its own: {staging}"
        );
        assert!(
            !staging.contains("/next-stage") && !staging.contains("/to-tasks"),
            "nobody is at a terminal to run a slash command: {staging}"
        );
    }

    /// And it carries the branch the rest of the way, exactly as the next-task
    /// fork's finish does: a roadmap is work like any other work, so it goes for
    /// review like any other work. How is the target repository's own business.
    #[test]
    fn the_staging_skill_opens_the_pull_request_the_repositorys_own_way() {
        let staging = skill("staging/SKILL.md");

        assert!(
            staging.contains("docs/agents/git-workflow.md") && staging.contains("Finish sequence"),
            "the process is the repository's, read out of the file that records it: {staging}"
        );
        assert!(
            staging.contains("gh stack submit --auto"),
            "a stacked branch is submitted as a stack: {staging}"
        );
        assert!(
            staging.contains("gh pr create --draft"),
            "and an unstacked one opens a draft PR of its own: {staging}"
        );
    }

    /// Two sessions can be reading this one too: the grilling session carrying on
    /// from the pick, and a fresh one launched because that tail was retried.
    /// They differ in what the reader has to ground itself in, so the skill has
    /// to say both.
    #[test]
    fn the_staging_skill_works_from_both_ways_in() {
        let staging = skill("staging/SKILL.md");

        assert!(
            staging.contains("the grilling session, reading on"),
            "the ordinary way in is the session that settled the work carrying on: \
             {staging}"
        );
        assert!(
            staging.contains("a fresh session"),
            "and the other is a retried tail: {staging}"
        );
        assert!(
            staging.contains("there is no handoff document"),
            "which is grounded in the Brief, the repository and the retry note — a \
             roadmap writes no handoff for it to have been handed: {staging}"
        );
    }

    /// The stage list is the human's decision to make, and it reaches them the
    /// only way anything does.
    #[test]
    fn the_staging_skill_puts_its_stage_list_through_the_cli() {
        let staging = skill("staging/SKILL.md");

        assert!(
            staging.contains("verkstead guide") && staging.contains("verkstead ask"),
            "the stage list is an ordinary Set, asked the way every Set is: {staging}"
        );
        assert!(
            !staging.contains("proposal:"),
            "and an ordinary one: the `proposal` block is the grilling's closing move, \
             and this runs after one was accepted"
        );
    }

    /// The Brief, into the third skill. What differs is the line above it, which
    /// is the whole of what sends a session one way or another.
    #[test]
    fn a_roadmap_session_is_started_on_the_brief_inside_the_fork() {
        let prompt = staging("# Rate limiting\n\nThe API has none.\n");

        assert!(
            prompt.contains(STAGING),
            "the fork is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            !prompt.contains(IMPLEMENTING) && !prompt.contains(BREAKING_DOWN),
            "and nothing sends this session to build or slice the work instead: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none."),
            "the Brief goes in whole: {prompt:?}"
        );
        assert!(
            !prompt.contains("What the grilling settled"),
            "and nothing is said about a document a roadmap never has: {prompt:?}"
        );
    }

    /// Why a stage is planned in a session of its own rather than when the
    /// roadmap was written: the brief's chunking is provisional, and the code has
    /// moved under it since. Re-grounding it is the whole of what this fork adds.
    #[test]
    fn the_next_stage_fork_re_grounds_the_brief_against_the_code() {
        let next_stage = skill("next-stage/SKILL.md");

        assert!(
            next_stage.contains("Re-verify at start")
                && next_stage.contains("Proposed tasks (provisional)"),
            "the two sections of a brief that say what to check and what to correct: \
             {next_stage}"
        );
        assert!(
            next_stage.contains("sequential"),
            "and what comes out is a backlog, whose order is its dependency: {next_stage}"
        );
    }

    /// The breakdown quiz, which is the one thing in a whole roadmap's run that
    /// stops it — and stops it naturally, being a blocking ask.
    #[test]
    fn the_next_stage_fork_puts_its_breakdown_to_the_human() {
        let next_stage = skill("next-stage/SKILL.md");

        assert!(
            next_stage.contains("verkstead guide") && next_stage.contains("verkstead ask"),
            "the quiz is an ordinary Set, asked the way every Set is: {next_stage}"
        );
        assert!(
            !next_stage.contains("proposal:"),
            "and an ordinary one: the `proposal` block is the grilling's closing move, \
             and a stage has no grilling of its own"
        );
    }

    /// What it produces is a `.tasks/` backlog the runner then works, so it has
    /// to write the same files the fork of next-task reads — and the line that
    /// says this backlog is a stage's.
    #[test]
    fn the_next_stage_fork_writes_the_backlog_the_runner_works() {
        let next_stage = skill("next-stage/SKILL.md");

        for named in [".tasks/", "TODO.md", "NN-<slug>.md", "Roadmap stage:"] {
            assert!(
                next_stage.contains(named),
                "the backlog is files in the repository, and {named} is one of them"
            );
        }

        assert!(
            next_stage.contains("Nothing waits on approval"),
            "and the plan commit has no gate in front of it, as no commit here does"
        );
        assert!(
            !next_stage.contains("Do not start on task 01\n"),
            "the task after the plan is Verkstead's to launch a session for"
        );
    }

    /// The roadmap keeps its own score, and the plan commit is what moves it:
    /// the stage before this one ticked, and this one annotated with the branch
    /// it is being worked on — which is also what stops Verkstead starting this
    /// stage twice.
    #[test]
    fn the_next_stage_fork_moves_the_roadmaps_own_score() {
        let next_stage = skill("next-stage/SKILL.md");

        assert!(
            next_stage.contains("ROADMAP.md"),
            "the score is kept in the roadmap's index: {next_stage}"
        );
        assert!(
            next_stage.contains("*(in progress: `<branch>`)*"),
            "and the stage in flight is annotated with the branch, which is the fact \
             rather than the prose: {next_stage}"
        );
        assert!(
            next_stage.contains("`- [x]`"),
            "the stage before it is ticked, its work having settled: {next_stage}"
        );
    }

    /// Stacking is the repository's mechanism rather than Verkstead's, so the
    /// fork is told where to read it and told not to invent one.
    #[test]
    fn the_next_stage_fork_stacks_the_repositorys_own_way() {
        let next_stage = skill("next-stage/SKILL.md");

        assert!(
            next_stage.contains("docs/agents/git-workflow.md")
                && next_stage.contains("### Stacking roadmap stages"),
            "the mechanism is the repository's, read out of the file that records it: \
             {next_stage}"
        );
        assert!(
            next_stage.contains("Do not invent one"),
            "and where there is none there is none: {next_stage}"
        );
        assert!(
            !next_stage.contains("/next-stage") && !next_stage.contains("/to-tasks"),
            "nobody is at a terminal to run a slash command: {next_stage}"
        );
    }

    /// A stage session is put inside the fork the same way every other session
    /// is — and primed with one document rather than two, there being no
    /// grilling of its own to have written a handoff.
    #[test]
    fn a_stage_session_is_started_on_its_brief_inside_the_fork() {
        let prompt = next_stage("# 05. Roadmap direction\n\nThe third Direction.\n", None);

        assert!(
            prompt.contains(NEXT_STAGE),
            "the fork is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.contains("The third Direction."),
            "and the stage brief is what it is primed with: {prompt:?}"
        );
        assert!(
            !prompt.contains("What the grilling settled"),
            "nothing is said about a document a stage never had: {prompt:?}"
        );
        assert!(
            !prompt.contains(BREAKING_DOWN) && !prompt.contains(NEXT_TASK),
            "and nothing sends this session to break down a feature or work a task: \
             {prompt:?}"
        );
    }

    /// Where the branch came from is the one thing about a stage the session
    /// cannot read out of the repository, so it is the one thing it is told.
    #[test]
    fn a_stage_session_is_told_whether_its_branch_is_stacked() {
        let stacked = next_stage("# 05. Roadmap direction\n", Some("wrap-up"));

        assert!(
            stacked.contains("`wrap-up`"),
            "the predecessor is named, because registering the stack needs it: {stacked:?}"
        );

        let alone = next_stage("# 05. Roadmap direction\n", None);

        assert!(
            alone.contains("default branch") && !alone.contains("stacks on"),
            "and a stage off the default branch is told that plainly: {alone:?}"
        );
    }

    /// One task per session is the whole reason there is a session per task, and
    /// the fork has to say so: nothing else will. The done-signal the runner
    /// watches is the file gone and committed, so the deletion and the commit
    /// are the two things it cannot leave out.
    #[test]
    fn the_next_task_fork_works_one_task_and_commits_it() {
        let next_task = skill("next-task/SKILL.md");

        for named in [".tasks/", "TODO.md", "NN-<slug>.md"] {
            assert!(
                next_task.contains(named),
                "the backlog is files in the repository, and {named} is one of them"
            );
        }

        assert!(
            next_task.contains("lowest-numbered"),
            "which task is decided by the same rule the runner decides it by: {next_task}"
        );
        assert!(
            next_task.contains("rm .tasks/NN-<slug>.md") && next_task.contains("git commit"),
            "the file being gone and committed is what says the task is done: {next_task}"
        );
        assert!(
            next_task.contains("Nothing waits on approval"),
            "and there is no gate in front of that commit, as there is in front of none: \
             {next_task}"
        );
    }

    /// The finish step is the other half of what the fork decides, and the
    /// runner watches it the same way: `TODO.md` gone and committed.
    #[test]
    fn the_next_task_fork_finishes_the_feature_by_taking_the_list_away() {
        let next_task = skill("next-task/SKILL.md");

        assert!(
            next_task.contains("git rm .tasks/TODO.md"),
            "taking the list away is what says the feature is finished: {next_task}"
        );
    }

    /// And it carries the branch the rest of the way: pushed, and opened as a
    /// draft pull request. How is the target repository's own business, so what
    /// the fork carries is the instruction to read and follow its process —
    /// naming both shapes, because which one applies is a fact about the branch.
    #[test]
    fn the_next_task_fork_opens_the_pull_request_the_repositorys_own_way() {
        let next_task = skill("next-task/SKILL.md");

        assert!(
            next_task.contains("docs/agents/git-workflow.md")
                && next_task.contains("Finish sequence"),
            "the process is the repository's, read out of the file that records it: {next_task}"
        );
        assert!(
            next_task.contains("gh stack submit --auto"),
            "a stacked branch is submitted as a stack: {next_task}"
        );
        assert!(
            next_task.contains("gh pr create --draft"),
            "and an unstacked one opens a draft PR of its own: {next_task}"
        );
        assert!(
            !next_task.contains("Do not push"),
            "nothing holds the branch back any more — the finish carries it to a PR: \
             {next_task}"
        );
        assert!(
            next_task.contains("Nothing waits on approval here either"),
            "and there is no gate in front of that either, as there is in front of none: \
             {next_task}"
        );
    }

    /// The fork drops what a workstation-driven flow assumes and Verkstead
    /// supplies instead: nobody is at a terminal to approve a commit, to clear a
    /// context, or to be asked whether to land the feature.
    #[test]
    fn the_next_task_fork_drops_the_gates_verkstead_supplies() {
        let next_task = skill("next-task/SKILL.md");

        assert!(
            !next_task.contains("/clear") && !next_task.contains("/next-task"),
            "nobody is at a terminal to clear a context or run a slash command: {next_task}"
        );
        assert!(
            !next_task.contains("OK to proceed") && !next_task.contains("Clear the context"),
            "and none of the three gates the workstation flow stops at: {next_task}"
        );
        assert!(
            next_task.contains("verkstead guide") && next_task.contains("verkstead ask"),
            "the one way to the human, for what the task file cannot settle: {next_task}"
        );
    }

    /// One skill for three callers is the whole reason it is one skill, so it
    /// has to name all three: a failed check, a review finding and a comment on
    /// the pull request are one job, and three skills saying it would be three
    /// things to keep true.
    #[test]
    fn the_addressing_skill_is_written_for_all_three_kinds_of_feedback() {
        let addressing = skill("addressing/SKILL.md");

        for named in ["check that failed", "finding from the review", "comment"] {
            assert!(
                addressing.contains(named),
                "the skill should say it serves a {named}: {addressing}"
            );
        }
    }

    /// What a fix session has to leave behind, and the one way it differs from
    /// every other session here: the branch is already on a pull request, so a
    /// fix that stays local is one nothing re-runs and nobody sees.
    #[test]
    fn the_addressing_skill_says_to_commit_and_push_without_asking() {
        let addressing = skill("addressing/SKILL.md");

        assert!(
            addressing.contains("git commit") && addressing.contains("git push"),
            "the commit is the fix and the push is what puts it on the pull request: \
             {addressing}"
        );
        assert!(
            addressing.contains("Nothing waits on approval"),
            "and there is no gate in front of either, as there is in front of none: {addressing}"
        );
        assert!(
            !addressing.contains("gh pr create"),
            "the pull request already exists, and a second one would be a fix nobody \
             asked for: {addressing}"
        );
        assert!(
            addressing.contains("verkstead guide") && addressing.contains("verkstead ask"),
            "the one way to the human, for feedback the codebase cannot settle: {addressing}"
        );
    }

    /// The scope is the feedback and nothing beside it, which is what makes a
    /// fix reviewable against the thing that was asked for.
    #[test]
    fn the_addressing_skill_keeps_the_fix_to_the_feedback() {
        let addressing = skill("addressing/SKILL.md");

        assert!(
            addressing.contains("Keep to the feedback"),
            "anything else it notices is somebody else's piece of feedback: {addressing}"
        );
        assert!(
            addressing.contains("Fix the cause"),
            "and the cause rather than the symptom: {addressing}"
        );
    }

    /// A fix session is put inside the skill the same way every other is, and
    /// primed with the documents *and* the feedback — which is the one thing
    /// only this kind of session is told.
    #[test]
    fn a_fix_session_is_started_on_the_documents_with_the_feedback_last() {
        let prompt = addressing(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            "The Rust check is failing.",
        );

        assert!(
            prompt.contains(ADDRESSING),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole: {prompt:?}"
        );
        assert!(
            prompt.find("In-process counter.") < prompt.find("The Rust check is failing."),
            "and the feedback comes last: it is the newest thing said and the least \
             general — {prompt:?}"
        );
    }

    /// What the reviewing skill has to be, now that the session it runs is the
    /// whole of a wrap-up: it reads the branch, it proposes, and it lands what
    /// was agreed to.
    #[test]
    fn the_reviewing_skill_proposes_and_then_fixes() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("You propose, and then you fix what was agreed to"),
            "nothing is changed before the human has said so, and everything they \
             accepted is changed here: {reviewing}"
        );
        assert!(
            reviewing.contains("git commit") && reviewing.contains("git push"),
            "which means this session lands its own work, unlike the review this \
             replaced — {reviewing}"
        );
        assert!(
            reviewing.contains("gh pr diff"),
            "what it reads is the pull request the work is on: {reviewing}"
        );
    }

    /// The half of it the human decides: one Set, a Question per finding, and the
    /// block that says which Answer to each means fix it.
    #[test]
    fn the_reviewing_skill_says_how_a_finding_becomes_work() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("review:") && reviewing.contains("findings:"),
            "the Set is marked by the block it carries, so the skill has to name it: \
             {reviewing}"
        );
        assert!(
            reviewing.contains("fix: Q1.1"),
            "and name the Option that means fix it, in the Guide's own notation: \
             {reviewing}"
        );
        assert!(
            reviewing.contains("verkstead guide") && reviewing.contains("verkstead ask"),
            "put through the CLI like every other Set: {reviewing}"
        );
        assert!(
            !reviewing.contains("proposal:"),
            "and carrying no proposal: this runs long after a grilling ended — {reviewing}"
        );
    }

    /// The other half: the session waits for the answers rather than being ended
    /// on the ask, and what it does with them is what the human wrote.
    #[test]
    fn the_reviewing_skill_waits_for_the_answers_and_acts_on_what_they_said() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("The answers are yours to wait for"),
            "nothing ends this session on the ask and nobody else is dispatched: \
             {reviewing}"
        );
        assert!(
            reviewing.contains("background command"),
            "so the ask that blocks for hours is run as one: {reviewing}"
        );
        assert!(
            reviewing.contains("part of the instruction"),
            "what they wrote beside a yes changes what is done about it: {reviewing}"
        );
        assert!(
            reviewing.contains("declined is over"),
            "and a finding they declined is left alone: {reviewing}"
        );
    }

    /// A review with nothing to raise asks nothing, and says so where the human
    /// is already looking: the last line a session prints is what its Timeline
    /// row shows. Nothing to raise is both halves, now that the comments are the
    /// other source of a decision.
    #[test]
    fn the_reviewing_skill_says_what_to_do_having_found_nothing() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("Ask nothing"),
            "a Set with no findings is a row for the human to dismiss: {reviewing}"
        );
        assert!(
            reviewing.contains("last thing you print"),
            "and what it found is said where the Timeline will show it: {reviewing}"
        );
        assert!(
            reviewing.contains("both halves"),
            "but a branch it would not have touched still proposes about what was \
             said on it: {reviewing}"
        );
    }

    /// The comments are the other half of what a review proposes about, and the
    /// only session that will ever act on them — so the skill has to say both
    /// that they are folded into the one Set and that none of them is acted on
    /// before the human has answered.
    #[test]
    fn the_reviewing_skill_folds_the_pull_requests_comments_into_its_one_set() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("What has been said on the pull request"),
            "the comments are named by the heading they arrive under: {reviewing}"
        );
        assert!(
            reviewing.contains("You are the only session that will act on these"),
            "nothing else is dispatched about them, so leaving one out answers \
             nobody: {reviewing}"
        );
        assert!(
            reviewing.contains("still a proposal until they have said yes"),
            "and a comment is not an instruction: it goes into the Set like every \
             other finding — {reviewing}"
        );
    }

    /// A check that goes red while the review waits on the human has nobody
    /// dispatched to it — the review is holding the Worktree — so the woken
    /// session deals with it itself, before its push.
    #[test]
    fn the_reviewing_skill_folds_a_red_check_into_the_woken_session() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("gh pr checks"),
            "the woken session reads the pull request's own check state: {reviewing}"
        );
        assert!(
            reviewing.find("gh pr checks") < reviewing.find("git push"),
            "and fixes what is failing before it pushes, so the push is what puts \
             the fix back in front of the checks — {reviewing}"
        );
        assert!(
            reviewing.contains("nothing to propose about a red check"),
            "a red check is the branch being broken rather than a decision, so it \
             never becomes a finding: {reviewing}"
        );
        assert!(
            reviewing.contains("whatever they decided"),
            "and a review whose every finding was declined still fixes one: \
             {reviewing}"
        );
    }

    /// The review session is put inside the skill the same way every other is,
    /// and primed with the two documents that say what the work was *for*.
    #[test]
    fn a_review_session_is_started_on_the_documents_inside_the_skill() {
        let prompt = reviewing(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            None,
        );

        assert!(
            prompt.contains(REVIEWING),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole: {prompt:?}"
        );
        assert!(
            !prompt.contains(ADDRESSING) && !prompt.contains(NEXT_TASK),
            "and no other skill is named: what this session does once it has reviewed \
             is the reviewing skill's own — {prompt:?}"
        );
        assert!(
            !prompt.contains("What has been said on the pull request"),
            "a pull request nobody has written on carries no heading saying so: \
             {prompt:?}"
        );
    }

    /// And what was said on the pull request goes in last, where the newest and
    /// least general thing goes in every prompt here.
    #[test]
    fn a_review_session_is_told_what_was_said_on_the_pull_request_last() {
        let prompt = reviewing(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            Some("**tobico** said on `src/window.rs` line 12:\n\nThis is the wrong way round."),
        );

        assert!(
            prompt.contains("This is the wrong way round.")
                && prompt.contains("`src/window.rs` line 12"),
            "what was said and where, whole: {prompt:?}"
        );
        assert!(
            prompt.find("In-process counter.") < prompt.find("This is the wrong way round."),
            "under the documents: they say what the work is and this says what \
             somebody has already said about it — {prompt:?}"
        );
    }

    /// What the responding skill has to be: the review's shape about a batch of
    /// comments, which is the whole of what stops one being acted on ungated.
    #[test]
    fn the_responding_skill_proposes_before_it_changes_anything() {
        let responding = skill("responding/SKILL.md");

        assert!(
            responding.contains("You propose, and then you fix what was agreed to"),
            "nothing anybody wrote is acted on before the human has said so, and \
             everything they accepted is done here: {responding}"
        );
        assert!(
            responding.contains("Change nothing yet"),
            "which means the session that reads the batch changes nothing until the \
             answers arrive: {responding}"
        );
        assert!(
            responding.contains("still a proposal until they have said yes"),
            "a comment is not an instruction, however plainly it is written: \
             {responding}"
        );
        assert!(
            responding.contains("git commit") && responding.contains("git push"),
            "and then it lands its own work, which is why nothing else is \
             dispatched: {responding}"
        );
    }

    /// The half the human decides: one Set for the batch, a Question per comment
    /// worth doing something about, and the block that says which Answer means
    /// do it.
    #[test]
    fn the_responding_skill_says_how_a_comment_becomes_work() {
        let responding = skill("responding/SKILL.md");

        assert!(
            responding.contains("review:") && responding.contains("findings:"),
            "the Set is marked by the block it carries, so the skill has to name it: \
             {responding}"
        );
        assert!(
            responding.contains("fix: Q1.1"),
            "and name the Option that means do it, in the Guide's own notation: \
             {responding}"
        );
        assert!(
            responding.contains("verkstead guide") && responding.contains("verkstead ask"),
            "put through the CLI like every other Set: {responding}"
        );
        assert!(
            responding.contains("The answers are yours to wait for"),
            "and nothing ends this session on the ask, as nothing ends the review's: \
             {responding}"
        );
    }

    /// A batch that asks for nothing asks nothing, and says so where the human is
    /// already looking: the last line a session prints is what its Timeline row
    /// shows.
    #[test]
    fn the_responding_skill_says_what_to_do_with_a_batch_that_asks_for_nothing() {
        let responding = skill("responding/SKILL.md");

        assert!(
            responding.contains("Ask nothing"),
            "a Set with nothing in it is a row for the human to dismiss: \
             {responding}"
        );
        assert!(
            responding.contains("last thing you print"),
            "and what was said is answered where the Timeline will show it: \
             {responding}"
        );
    }

    /// A batch session is put inside the skill the same way every other is, and
    /// primed with the two documents *and* the comments it is about.
    #[test]
    fn a_batch_session_is_started_on_the_documents_with_the_comments_last() {
        let prompt = responding(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            "**tobico** said on `src/window.rs` line 12:\n\nThis is the wrong way round.",
        );

        assert!(
            prompt.contains(RESPONDING),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole: {prompt:?}"
        );
        assert!(
            prompt.contains("This is the wrong way round.")
                && prompt.contains("`src/window.rs` line 12"),
            "and so does what was said, with where it was said: {prompt:?}"
        );
        assert!(
            prompt.find("In-process counter.") < prompt.find("This is the wrong way round."),
            "under the documents: they say what the work is and this says what \
             somebody has just said about it — {prompt:?}"
        );
        assert!(
            !prompt.contains(ADDRESSING) && !prompt.contains(REVIEWING),
            "and no other skill is named: a batch is neither a fix nor a review — \
             {prompt:?}"
        );
    }

    /// A session is put inside the skill by its prompt, and primed with the two
    /// documents the work is described by.
    #[test]
    fn an_implementation_session_is_started_on_the_brief_and_the_handoff() {
        let prompt = implementing(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
        );

        assert!(
            prompt.contains(IMPLEMENTING),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole: {prompt:?}"
        );
        assert!(
            prompt.find("The API has none.") < prompt.find("In-process counter."),
            "the Brief comes first: it is what the handoff is about"
        );
    }

    /// A grilling that skipped half its closing move still leaves work to do.
    #[test]
    fn an_implementation_without_a_handoff_is_started_on_the_brief_alone() {
        let prompt = implementing("# Rate limiting\n\nThe API has none.\n", None);

        assert!(prompt.contains("The API has none."));
        assert!(
            !prompt.contains("What the grilling settled"),
            "nothing is said about a document that was never written: {prompt:?}"
        );
    }

    /// The Brief, into the other skill. What differs is the line above it, which
    /// is the whole of what sends a session one way or the other.
    #[test]
    fn a_breakdown_session_is_started_on_the_brief_inside_the_fork() {
        let prompt = breaking_down("# Rate limiting\n\nThe API has none.\n");

        assert!(
            prompt.contains(BREAKING_DOWN),
            "the fork is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            !prompt.contains(IMPLEMENTING),
            "and nothing sends this session to build the work instead: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none."),
            "the Brief goes in whole: {prompt:?}"
        );
        assert!(
            !prompt.contains("What the grilling settled"),
            "and nothing is said about a document a task list never has: {prompt:?}"
        );
    }

    /// What a manual task's session has to leave behind, and the gate it does
    /// not stop at — the same two things every other working skill here says,
    /// because it is implementation-flavoured rather than grilling-flavoured.
    #[test]
    fn the_manual_task_skill_says_to_commit_without_asking() {
        let manual_task = skill("manual-task/SKILL.md");

        assert!(
            manual_task.contains("git commit"),
            "committing what it changed is how a manual task reports: {manual_task}"
        );
        assert!(
            manual_task.contains("Nothing waits on approval"),
            "and there is no gate in front of it, as there is in front of none: {manual_task}"
        );
        assert!(
            !manual_task.contains("gh pr create"),
            "what happens to the branch after a manual task is the human's to decide: \
             {manual_task}"
        );
    }

    /// The one thing this skill says differently from every other: it *may* ask,
    /// and nothing compels it to. A manual task is usually one instruction that
    /// already says what it means, and a session that asked about work it
    /// understood would idle for hours over nothing.
    #[test]
    fn the_manual_task_skill_leaves_asking_to_the_agents_judgement() {
        let manual_task = skill("manual-task/SKILL.md");

        assert!(
            manual_task.contains("verkstead guide") && manual_task.contains("verkstead ask"),
            "the one way to the human, for an instruction that cannot be carried out as \
             it stands: {manual_task}"
        );
        assert!(
            manual_task.contains("may** put a Question Set")
                && manual_task.contains("nothing here says you have to"),
            "asking is offered rather than required: {manual_task}"
        );
        assert!(
            !manual_task.contains("proposal:"),
            "and this is not a grilling: the `proposal` block is a grilling's closing \
             move, and a manual task is outside the pipeline altogether — {manual_task}"
        );
    }

    /// The scope is the instruction and nothing beside it, which is what makes a
    /// manual task reviewable against what the human actually typed.
    #[test]
    fn the_manual_task_skill_keeps_the_work_to_the_instruction() {
        let manual_task = skill("manual-task/SKILL.md");

        assert!(
            manual_task.contains("Keep to what was asked"),
            "anything else it notices is another manual task: {manual_task}"
        );
        assert!(
            manual_task.contains("commit nothing"),
            "and an instruction that changes no files is a manual task done rather than \
             one failed: {manual_task}"
        );
    }

    /// A manual session is put inside the skill the same way every other is, and
    /// primed with the instruction *alone* — which is the one thing that makes
    /// this prompt different from all of them.
    #[test]
    fn a_manual_session_is_started_on_the_instruction_and_nothing_else() {
        let prompt = manual_task("Rebase this onto `main` and force-push.\n");

        assert!(
            prompt.contains(MANUAL_TASK),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.ends_with("Rebase this onto `main` and force-push.\n"),
            "and the instruction is what follows, whole: {prompt:?}"
        );
        assert!(
            !prompt.contains("The Brief this started from")
                && !prompt.contains("What the grilling settled"),
            "neither document goes in: a manual task is not a slice of the work they \
             describe — {prompt:?}"
        );
        assert!(
            !prompt.contains(IMPLEMENTING) && !prompt.contains(NEXT_TASK),
            "and nothing sends this session to build the work or work a task instead: \
             {prompt:?}"
        );
    }

    /// A grilling started again is the same grilling — the same skill and the
    /// same Brief — with the log of what has already been settled under it, so
    /// that it does not open by asking what the human answered yesterday.
    #[test]
    fn a_relaunched_grilling_is_told_what_has_already_been_settled() {
        let prompt = grilling_again(
            "# Rate limiting\n\nThe API has none.\n",
            "## How it counts\n\n**Q1** Per key or per address?\n\nPer key\n",
        );

        assert!(
            prompt.contains(GRILLING),
            "it is a grilling, started the way every grilling is: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none."),
            "on the Brief it was always about: {prompt:?}"
        );
        assert!(
            prompt.contains("# What has already been asked, and what I said"),
            "under a heading that says what the digest is: {prompt:?}"
        );
        assert!(
            prompt.ends_with("**Q1** Per key or per address?\n\nPer key\n"),
            "and the digest goes last, being the least general thing said: {prompt:?}"
        );
    }

    /// And a grilling that died before its first Set came back is the Brief
    /// alone. A heading over nothing would tell the session that something had
    /// been said, which is exactly the thing this is for.
    #[test]
    fn a_relaunched_grilling_with_nothing_answered_is_started_on_the_brief_alone() {
        let brief = "# Rate limiting\n\nThe API has none.\n";

        assert_eq!(
            grilling_again(brief, "   \n"),
            grilling(brief),
            "nothing is added to the prompt at all",
        );
    }

    /// The workbench shows a commit's message body beside its diff, and nothing
    /// else tells a session to write one — so every skill that commits work has
    /// to say it. One wording across the six, because six wordings would be
    /// six things to keep true and the human reads them as one convention.
    #[test]
    fn every_skill_that_commits_work_asks_for_the_commits_summary() {
        let block = summary_block("next-task/SKILL.md");

        for name in [
            "implementing/SKILL.md",
            "manual-task/SKILL.md",
            "addressing/SKILL.md",
            "reviewing/SKILL.md",
            "responding/SKILL.md",
        ] {
            assert_eq!(
                summary_block(name),
                block,
                "{name} should carry the same block, word for word"
            );
        }
    }

    /// And what the block says, held once: which commits get a summary, when the
    /// diagram is required, what kind of diagram it is, and that it comes first.
    #[test]
    fn the_summary_block_says_which_commits_and_what_goes_in_one() {
        let block = summary_block("next-task/SKILL.md");

        assert!(
            block.contains("delivers work") && block.contains("as its message body"),
            "the body is the summary, and it is the delivering commits that carry one: \
             {block}"
        );
        for bookkeeping in [
            "backlog commit",
            "roadmap commit",
            "the finish commit",
            "ADR",
        ] {
            assert!(
                block.contains(bookkeeping),
                "and {bookkeeping} is bookkeeping, which carries none: {block}"
            );
        }
        assert!(
            block.contains("file's deletion rides along with the code"),
            "a task's commit is still a delivering one, deletion and all: {block}"
        );
        assert!(
            block.contains("more than three changed lines"),
            "the diagram is required past a threshold rather than always: {block}"
        );
        assert!(
            block.contains("delta rather than the system"),
            "and it is the delta diagram the retired gates Topic taught: {block}"
        );
        assert!(
            block.contains("Tag each node `new`, `modified`") && block.contains("`removed`"),
            "tagged with what happened to each part, which is what the viewer colours \
             from the diff's own shades: {block}"
        );
        assert!(
            block.contains("Around ten nodes"),
            "and small enough to read on a phone: {block}"
        );
        assert!(
            block.find("The diagram first") < block.find("The prose after it"),
            "the glance comes before the reading, in the block as in the message: {block}"
        );
    }

    /// The skills that only keep the books are left alone: a plan, a roadmap or
    /// a stage's backlog is not work delivered, and a summary over one would be
    /// a diagram of a file that was written rather than of a change.
    #[test]
    fn the_bookkeeping_skills_ask_for_no_summary() {
        for name in [
            "breaking-down/SKILL.md",
            "staging/SKILL.md",
            "next-stage/SKILL.md",
        ] {
            let text = skill(name);

            assert!(
                !text.contains(SUMMARY_BLOCK),
                "{name} commits bookkeeping, which carries no summary:\n{text}"
            );
        }
    }

    /// And the one bookkeeping commit inside a working skill stays subject-only
    /// for the same reason: taking the list away delivers nothing.
    #[test]
    fn the_next_task_forks_finish_commit_stays_subject_only() {
        let next_task = skill("next-task/SKILL.md");

        assert!(
            next_task.contains(r#"git commit -m "chore: finish <feature-name>""#),
            "the finish is a subject and nothing under it: {next_task}"
        );
    }

    /// Installing is what puts them where a sandbox can bind them.
    #[test]
    fn the_skills_are_written_out_under_the_data_directory() {
        let state = tempfile::tempdir().unwrap();

        let skills = Skills::installed(state.path()).expect("this binary carries skills");

        assert_eq!(skills.path(), state.path().join("skills"));
        assert_eq!(
            std::fs::read_to_string(skills.path().join("grilling/SKILL.md")).unwrap(),
            skill("grilling/SKILL.md"),
            "what is installed is what the binary carries"
        );
    }

    /// A skill this binary does not ship is not one a session should find,
    /// however many earlier binaries left it there.
    #[test]
    fn what_an_earlier_install_left_behind_does_not_survive() {
        let state = tempfile::tempdir().unwrap();
        let stale = state.path().join("skills/withdrawn");

        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("SKILL.md"), "# gone\n").unwrap();

        Skills::installed(state.path()).expect("this binary carries skills");

        assert!(
            !stale.exists(),
            "a withdrawn skill is still installed: {}",
            stale.display()
        );
    }
}
