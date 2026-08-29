//! The skills Verkstead ships, and how a session comes to be running one.
//!
//! A session's behaviour should be the product's rather than whatever happens
//! to be installed on the machine it runs on. So Verkstead carries its own
//! skills, embedded in the binary as the viewer is (ADR-0004), writes them out
//! under the Data Directory at startup, and every sandbox binds that directory
//! read-only at [`INSIDE`]. Nothing beside the executable has to be there, and
//! nothing of the host's is bound in for a session to find — the checkout of
//! the skills the host keeps for its own agents is not reachable at all.
//!
//! An account's own skills are hidden rather than merged with: Verkstead's fork
//! is what a Conversation is grilled by, and a Profile is an account to run as
//! rather than a second opinion about how to work. The mount used to do that
//! hiding by landing on the account's own path; a mount at a path no backend
//! owns covers nothing, so what a sandbox puts over [`CLAUDE_INSIDE_HOME`]
//! instead is [`Skills::nothing`].
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

use crate::store;

/// The skills as they are written in this repository, one directory per skill.
///
/// Compiled in for a release build and read off disk for a debug one, exactly
/// as the viewer is and for the same reason: editing a skill is then visible to
/// a running `cargo run -p verkstead-cli -- serve` without a recompile.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/skills"]
struct Bundled;

/// Where the skills are mounted inside a sandbox.
///
/// A directory of Verkstead's own, beside the `/verkstead/bin` the executable
/// reaches a sandbox in and made by its bind exactly as that one is, so what is
/// there is what this binary ships and nothing the host put beside it. A path
/// no backend owns rather than the one whichever backend is running discovers
/// skills in: what a session reads is named by the prompt, and a prompt can say
/// any path (ADR-0011).
pub(crate) const INSIDE: &str = "/verkstead/skills";

/// And where a Claude session would otherwise find skills of its own, under
/// whatever HOME a sandbox has: the account's, kept in the Profile's directory,
/// which [`Skills::nothing`] is bound over instead.
pub(crate) const CLAUDE_INSIDE_HOME: &str = ".claude/skills";

/// And what [`INSIDE`] makes the grilling skill's path, as a session is told to
/// find it. Spelled out rather than written against a home: the mount is at a
/// path of Verkstead's own, absolute and the same for every session.
const GRILLING: &str = "/verkstead/skills/grilling/SKILL.md";

/// The implementation skill's, the same way.
const IMPLEMENTING: &str = "/verkstead/skills/implementing/SKILL.md";

/// And the staging skill's — Verkstead's fork of to-roadmap, which is what the
/// roadmap direction runs instead of building anything itself.
const STAGING: &str = "/verkstead/skills/staging/SKILL.md";

/// And the fork of next-stage, which the one session a roadmap stage starts
/// with runs inside: the session that re-grounds the stage's brief and writes
/// the backlog the runner then works.
const NEXT_STAGE: &str = "/verkstead/skills/next-stage/SKILL.md";

/// And the fork of next-task, which every session the runner launches is put
/// inside — the task sessions and the finish one alike, because which of them it
/// is, is read off `.tasks/` rather than told.
const NEXT_TASK: &str = "/verkstead/skills/next-task/SKILL.md";

/// And the submitting skill's, which the one session launched over a finish
/// that left no pull request runs inside: the work is built and committed, and
/// the pull request it should have gone for review on is the one thing missing.
const SUBMITTING: &str = "/verkstead/skills/submitting/SKILL.md";

/// And the addressing skill's, which every fix session of a wrap-up runs
/// inside — whichever of the three kinds of feedback dispatched it.
const ADDRESSING: &str = "/verkstead/skills/addressing/SKILL.md";

/// And the reviewing skill's, which the one session a wrap-up starts with runs
/// inside: the fresh context that reads the branch none of the sessions that
/// wrote it ever saw.
const REVIEWING: &str = "/verkstead/skills/reviewing/SKILL.md";

/// And the responding skill's, which a batch of comments left on the pull
/// request after the review is answered inside: the review's propose-then-fix
/// shape again, about what somebody has just said rather than about the branch.
const RESPONDING: &str = "/verkstead/skills/responding/SKILL.md";

/// And the instruction skill's, which the session a steer into Implementing
/// writes its way into runs inside.
///
/// The one session a human sets going by hand, and it is the pipeline's own:
/// what follows it is whatever the branch then holds, rather than a
/// Conversation left standing beside the work its session did — see
/// [`crate::runner::instructed`].
const INSTRUCTION: &str = "/verkstead/skills/instruction/SKILL.md";

/// And the following-up skill's, which the session a steer into Follow-up
/// launches runs inside.
///
/// A conversation rather than a step: the human's follow-up brief is acted on,
/// and then rounds of ordinary Question Sets go back and forth until they have
/// nothing else. What ends one is the system's business rather than the
/// session's, so the skill says nothing about it.
const FOLLOWING_UP: &str = "/verkstead/skills/following-up/SKILL.md";

/// The bundled skills, installed on the host, ready for a sandbox to bind.
#[derive(Debug, Clone)]
pub struct Skills {
    path: PathBuf,

    /// And an empty directory beside them, which is what covers the account's
    /// own — see [`Skills::nothing`].
    nothing: PathBuf,
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

        let nothing = data_dir.join("nothing");

        std::fs::create_dir_all(&nothing)
            .with_context(|| format!("making {}", nothing.display()))?;

        Ok(Skills { path, nothing })
    }

    /// Where they landed, which is what a sandbox binds at [`INSIDE`].
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// And an empty directory of Verkstead's own, which a sandbox binds
    /// read-only over [`CLAUDE_INSIDE_HOME`].
    ///
    /// What the mount used to do by standing on the account's own path: an
    /// account's skills are hidden rather than merged with, and the case that
    /// guards is an older fork of the ones Verkstead ships sitting in the
    /// Profile's directory. Kept empty rather than made fresh per sandbox
    /// because nothing is ever written into it — the bind is read-only, so a
    /// session cannot fill it in and then read from it.
    pub fn nothing(&self) -> &Path {
        &self.nothing
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
/// Under the Brief, and for the reason everything written under one is: the
/// Brief says what the work is, and this says what has already been decided
/// about it — the newer and the less general of the two, so it goes second.
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

/// And what an inline session on a Conversation that was never grilled is
/// started on: the Brief alone, under the same line, and the paragraph that says
/// there was no grilling.
///
/// Said rather than left to be inferred from an absent handoff, because the two
/// are different situations and only one of them is a plan. A grilling that died
/// before writing its handoff leaves a session that should build what the
/// interview settled and cannot read it; this is a human who chose not to be
/// interviewed, and the Brief is the whole of what they decided.
///
/// Which is why the paragraph says what to do with what the Brief leaves open. A
/// session that guesses at a real decision builds the wrong thing quietly; one
/// that asks reaches the human on their phone and builds the right thing.
///
/// The skill is the same implementation skill an ordinary inline run reads, and
/// it knows this run happens: it says a Conversation can be started with no
/// grilling, that the Brief is the whole of the agreement where one was, and
/// that the instruction about what the Brief leaves open is here rather than
/// there. The split is deliberate — the skill is where a session learns what
/// kind of run this is, and the prompt is where it is told what to do about it,
/// because only the prompt knows which kind this one is.
pub(crate) fn ungrilled(brief: &str) -> String {
    format!(
        "{}\n# Nothing was grilled\n\nThis work was not put through a grilling: \
         the Brief above is the whole of the plan, and there is no handoff \
         because there was no interview to write one. Build what it describes. \
         Where it leaves a real decision open — one that changes what gets built \
         rather than how it is spelled — put that to me as a blocking ask rather \
         than guessing at it.\n",
        implementing(brief, None),
    )
}

/// What a roadmap Conversation's own work is started on where Resume launches it:
/// the Brief, under the line that sends the agent into the staging fork.
///
/// One document rather than two, unlike the sessions that build. A roadmap
/// writes no handoff — the stage briefs are what the grilling settled, committed
/// to the branch — so there is never one for this to carry. The ordinary way in
/// is the grilling session reading on with the whole thread still in its context
/// and no prompt sent at all; this is Resume, which grounds itself in the Brief
/// and the repository.
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

/// What the session sent after a finish that opened nothing is started on: the
/// same two documents again, under the line that sends the agent into the
/// submitting skill.
///
/// The documents rather than a bare instruction to push, for the reason every
/// other session here is given them: a pull request is titled and described for
/// the work it carries, and what that work was for is written in the Brief and
/// the handoff rather than anywhere the branch could say it. The commits say
/// what was built; these two say what it was meant to be.
pub(crate) fn submitting(brief: &str, handoff: Option<&str>) -> String {
    on_the_documents(
        &format!(
            "Read {SUBMITTING} and get the work already committed on this branch onto a \
             pull request, the way it says."
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
/// `on` is every pull request the work ended up on, where it ended up on more
/// than one: each of them named with its number, the repository it was opened
/// in, its URL and the worktree to read it in. One review reads the whole of the
/// work and the whole of it may be several branches, so the session is told where
/// each of them is — it starts in the Conversation's own worktree and both `git`
/// and `gh` read their repository from wherever they are run, so a `gh pr diff`
/// left where the session landed would read one repository's half of the work
/// twice and the other's never.
///
/// A Conversation whose work touched nothing else carries none of it and is told
/// what it is told today. There is nothing there to say: the branch this worktree
/// is on is the whole of the work, which is what the opening line says already
/// and what the skill falls back on.
///
/// `said` is what was written on those pull requests before this session started
/// — the comments whole, in the order they were said in, with where each was
/// said. It goes *last*, under the documents and under the pull requests they
/// were left on, where the newest and least general thing goes in every other
/// prompt here: the documents say what the work is, the list says where it is,
/// and this says what somebody has already said about it. A pull request nobody
/// has written on carries none of it, rather than a heading saying nothing was
/// said.
pub(crate) fn reviewing(
    brief: &str,
    handoff: Option<&str>,
    on: Option<&str>,
    said: Option<&str>,
) -> String {
    let mut prompt = on_the_documents(
        &format!(
            "Read {REVIEWING} and review the branch this worktree is on, the way it says. The \
             work described below is what it was meant to be."
        ),
        brief,
        handoff,
    );

    if let Some(on) = on {
        prompt = format!(
            "{prompt}\n# The pull requests this work is on\n\nThe work reached more than one \
             repository, so it is on a pull request in each of them, and reviewing it is \
             reading every one of them. Read each of them where it lives — `git` and `gh` \
             both read the repository from wherever they are run.\n\n{}\n",
            on.trim()
        );
    }

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
/// comments and how many: the review is given everything standing on every one
/// of the pull requests when it starts, and this is given one batch of what was
/// said on one of them after it.
///
/// Which pull request that is, and which worktree to answer it in, are in `said`
/// rather than said here: a Conversation ends on one per repository it was worked
/// in, and a session sent at a companion's would otherwise read the diff of the
/// repository it started in — see [`crate::comments::feedback`].
///
/// `said` is never empty here, unlike the review's. A batch session exists
/// because something was said, so there is no version of this prompt with
/// nothing under the heading.
pub(crate) fn responding(brief: &str, handoff: Option<&str>, said: &str) -> String {
    let prompt = on_the_documents(
        &format!(
            "Read {RESPONDING} and answer what has just been said on the pull request named \
             at the end of this prompt, the way it says. The work described below is what it \
             was meant to be."
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
/// reason everything written under them goes there: it is the newest thing said
/// and the least general. The documents say what the work is; this says what is
/// wrong with it.
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

/// What an instruction session is started on: the documents the work is written
/// down in, and under them the instruction the human steered it with.
///
/// The documents *and* the instruction, both. This is a session the pipeline
/// carries on from, working the same branch as everything before it — so it is
/// told what the work is, and then told what it is for.
///
/// The instruction goes *last*, under the documents rather than over them, for
/// the reason everything written under them goes there: it is the newest thing
/// said and the least general. The documents say what the work is; this says
/// what to do about it now.
pub(crate) fn instruction(brief: &str, handoff: Option<&str>, instruction: &str) -> String {
    let prompt = on_the_documents(
        &format!(
            "Read {INSTRUCTION} and do what I have asked for at the end of this prompt, the \
             way it says."
        ),
        brief,
        handoff,
    );

    format!(
        "{prompt}\n# What I have asked for\n\n{}\n",
        instruction.trim()
    )
}

/// What a follow-up session is started on: the documents the work is written
/// down in, and under them the brief the human steered it into Follow-up with.
///
/// The instruction session's shape, and for the same reason — this is a session
/// the human set going by hand, so it is told what the work is and then told
/// what they want taken up about it. What differs is that the brief opens a
/// conversation rather than naming one job: the session answers it, does what it
/// asks, and goes on asking until they are done.
///
/// The brief goes *last*, under the documents rather than over them, for the
/// reason everything written under them goes there: it is the newest thing said
/// and the least general. The documents say what the work is; this says what
/// they want to follow up about it now.
///
/// **Except where the follow-up is being picked up again**, which is what
/// `settled` carries: the rounds it has already been through, under the brief
/// they were about. A follow-up that lost its session lost the conversation it
/// was having — see [`crate::follow_ups`] — so the relaunch is a fresh session
/// on the same brief, and what the digest adds is the one part of that
/// conversation the Timeline kept. Every follow-up a steer starts hands in
/// nothing here: a heading over an empty digest would tell the session that
/// something had already been said.
pub(crate) fn following_up(
    brief: &str,
    handoff: Option<&str>,
    follow_up: &str,
    settled: &str,
) -> String {
    let prompt = on_the_documents(
        &format!(
            "Read {FOLLOWING_UP} and follow up on this branch's pull request, the way it says."
        ),
        brief,
        handoff,
    );

    let prompt = format!(
        "{prompt}\n# What I want to follow up on\n\n{}\n",
        follow_up.trim()
    );

    let settled = settled.trim();

    if settled.is_empty() {
        return prompt;
    }

    format!("{prompt}\n# What you have already asked, and what I said\n\n{settled}\n")
}

/// The same prompt, with the Answers to the Conversation's Deferred Asks that
/// no session has been told about yet.
///
/// Written under the documents rather than over them, because it is the newest
/// thing said and the least general: the Brief and the handoff describe the work,
/// and this is what the human has since decided about it. A Deferred Ask is one
/// whose Answer does not change the work about to be done, so it reaches the
/// session that does the work after rather than the one that asked.
///
/// Nothing to fold is the prompt unchanged, which is every session of an
/// ordinary run: a heading over an empty digest would tell one that something
/// had been decided.
pub(crate) fn folded(prompt: &str, answers: &str) -> String {
    let answers = answers.trim();

    if answers.is_empty() {
        return prompt.to_owned();
    }

    format!("{prompt}\n# What I have since said about the deferred questions\n\n{answers}\n")
}

/// The same prompt again, with the companion repos the Conversation was
/// configured with listed under it.
///
/// One listing, on **every** session prompt of the Conversation — the grilling
/// one included — because a companion is checked out from grill start to close
/// and a session that was not told about it would be one standing beside a
/// directory it has no reason to look in. Appended where every session is
/// launched from rather than written into each prompt builder, so that a
/// builder added later cannot forget it.
///
/// Neutral, and deliberately: each companion is named with where it is, what it
/// holds and whether it may be written to, and nothing here says what to do
/// about any of that. The Brief is what says what the work is, and the agent
/// reads it — a prompt that told a session to go and use a repository would be
/// Verkstead deciding the work from a configuration screen.
///
/// Nothing about dev shells either, for the same reason. A companion with a
/// flake of its own is entered by the agent, `nix` being on the sandbox's
/// `PATH` — see [`crate::sandbox::under_dev_shell`].
///
/// A Conversation with no companions is the prompt unchanged, which is most of
/// them: a heading over an empty list would tell a session that something had
/// been configured.
///
/// `branch` is the Conversation's own, which is what a companion left to
/// mirror is called.
pub(crate) fn alongside(prompt: &str, branch: &str, companions: &[store::Companion]) -> String {
    let listed: Vec<String> = companions
        .iter()
        .filter_map(|companion| {
            // Only the ones that are actually checked out. Before grilling
            // starts there are none, and there is no session then either — so
            // this is the companion added to a Conversation whose checkout is
            // somehow gone, which is a row to leave unsaid rather than a path to
            // send a session to.
            let worktree = companion.worktree.as_ref()?;

            // A read-write companion holds a branch, mirroring resolved; a
            // read-only one is detached at the commit its base came to when the
            // checkout was made. The commit rather than the branch it was
            // resolved through, because the two are only the same thing on the
            // day: a session told it was on `main` would be told something that
            // stops being true the next time anybody pushes. The name is what a
            // checkout recorded before Verkstead kept the commit has to fall
            // back on.
            let holding = match companion.branch_for(branch) {
                Some(branch) => format!("on branch `{branch}`"),
                None => format!(
                    "detached at `{}`",
                    companion.base_commit.clone().unwrap_or_else(|| companion
                        .base_ref
                        .clone()
                        .unwrap_or_else(|| companion.repo.default_branch.clone()))
                ),
            };

            let mode = match companion.mode {
                store::CompanionMode::ReadOnly => "read-only",
                store::CompanionMode::ReadWrite => "read-write",
            };

            Some(format!(
                "- `{}` at `{}`, {holding}, {mode}.",
                companion.repo.name,
                worktree.display(),
            ))
        })
        .collect();

    if listed.is_empty() {
        return prompt.to_owned();
    }

    format!(
        "{}\n\n# Companion repositories\n\nThis Conversation is configured with other \
         repositories, checked out beside the worktree this session starts in.\n\n{}\n",
        prompt.trim_end(),
        listed.join("\n"),
    )
}

/// And the same prompt once more, with the one instruction the first session of
/// a Conversation nobody has named gets: pick the branch a name.
///
/// A Conversation is started on a name Verkstead invented, because there has to
/// be a branch to cut and nobody has thought about the work yet. Nothing is
/// drawn under that name — see the store's `Conversation::naming` — and this is
/// the other half of that: the session that reads the Brief first is the first
/// thing in the system that knows what the work is about, so naming the branch
/// is its job.
///
/// Said as an instruction with a reason rather than as a rule, and said last, so
/// that it is read as the first thing to do rather than as what the session is
/// for. What it asks for is one `git branch -m` before anything lands on the
/// branch: a rename with commits already on it works exactly as well, and a
/// human reading the pull request later sees one name rather than two.
///
/// Nothing is asked back. Verkstead reads the rename off the checkout the way it
/// reads commits — see [`crate::renames`] — so a session that renames has
/// reported it, and one that leaves the name alone has settled for it.
///
/// A branch nobody is waiting on is the prompt unchanged, which is every session
/// but one: a Conversation the human named had nothing to leave to anybody, and
/// after the first session the name is the Conversation's whatever it is.
pub(crate) fn naming(prompt: &str, naming: bool) -> String {
    if !naming {
        return prompt.to_owned();
    }

    format!(
        "{}\n\n# This branch has no name yet\n\nThe branch this session starts on \
         carries a name Verkstead invented at random, because the work had not \
         been read by anybody when it was cut. Switch it to a short kebab-case \
         name taken from what the Brief above is about — `git branch -m <name>` \
         in this worktree — before anything lands on it, and carry on. There is \
         nobody to ask and nothing to report: the rename is read off the \
         checkout, and the name is left as it is by leaving it alone.\n",
        prompt.trim_end(),
    )
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
    /// The breakdown skill's path — Verkstead's fork of to-tasks, which is what
    /// the task-list direction runs instead of building anything itself.
    ///
    /// The one mount path no prompt here names. Nothing launches a session into
    /// the breakdown: the grilling session reads on into it, in the grilling
    /// skill's own words, so what this is for is holding that promise to a test.
    const BREAKING_DOWN: &str = "/verkstead/skills/breaking-down/SKILL.md";

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
    /// The example's closing fence, which is the last line of the block now the
    /// diagram comes after the prose. The opening fence names its language, so
    /// this matches the closing one and nothing else.
    const SUMMARY_BLOCK_END: &str = "    ```\n";

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

    /// The heading the shared companion block opens with. The three skills that
    /// end a piece of work carry it word for word, and it runs to the next
    /// heading — there being nothing else in the section.
    const COMPANION_BLOCK: &str = "### And every companion repository you committed in";

    /// That block as one skill carries it, cut out so that the three can be held
    /// against each other.
    fn companion_block(name: &str) -> String {
        let text = skill(name);
        let start = text.find(COMPANION_BLOCK).unwrap_or_else(|| {
            panic!("{name} should carry every companion it committed in to a pull request:\n{text}")
        });
        let rest = &text[start + COMPANION_BLOCK.len()..];
        let end = rest.find("\n#").map_or(rest.len(), |at| at + 1);

        format!("{COMPANION_BLOCK}{}", &rest[..end])
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
    /// until the human closed it.
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

    /// And the inline session carries its own branch the rest of the way: pushed,
    /// and opened as a draft pull request the target repository's own way. There is
    /// no step after this one, so a skill that held the branch back would leave the
    /// run with nowhere to go.
    #[test]
    fn the_implementation_skill_opens_the_pull_request_itself() {
        let implementing = skill("implementing/SKILL.md");

        assert!(
            implementing.contains("docs/agents/git-workflow.md")
                && implementing.contains("Finish sequence"),
            "the process is the repository's, read out of the file that records it: \
             {implementing}"
        );
        assert!(
            implementing.contains("gh stack submit --auto"),
            "a stacked branch is submitted as a stack: {implementing}"
        );
        assert!(
            implementing.contains("gh pr create --draft"),
            "and an unstacked one opens a draft PR of its own: {implementing}"
        );
        assert!(
            !implementing.contains("Do not push"),
            "nothing holds the branch back any more — the session that built the work \
             carries it to a PR: {implementing}"
        );
        assert!(
            implementing.contains("Nothing waits on approval here either"),
            "and there is no gate in front of that either, as there is in front of \
             none: {implementing}"
        );
    }

    /// And a session that arrives to find the work already committed carries that
    /// to the pull request rather than reading *nothing to build* as *nothing to
    /// do*.
    ///
    /// The case Resume makes: the first session built the work and went before it
    /// pushed, and the fresh one launched over it is the only thing left that can
    /// finish the run.
    #[test]
    fn the_implementation_skill_says_what_a_second_session_does() {
        let implementing = skill("implementing/SKILL.md");

        assert!(
            implementing.contains("Nothing to build is not nothing to do"),
            "a session that finds the work already committed carries it on rather \
             than ending on nothing: {implementing}"
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

    /// One session reads this, and the skill says which: the grilling one,
    /// carrying on from the pick with everything it settled still in its
    /// context. Nothing else launches a breakdown, so a skill that offered a
    /// second way in would be describing a session nobody starts.
    #[test]
    fn the_breakdown_skill_is_read_by_the_grilling_session_carrying_on() {
        let breaking_down = skill("breaking-down/SKILL.md");

        assert!(
            breaking_down.contains("reading on"),
            "the way in is the session that settled the work carrying on: \
             {breaking_down}"
        );
        assert!(
            breaking_down.contains("no handoff document"),
            "and there is nothing else for it to have been handed — a task list \
             writes no handoff: {breaking_down}"
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
            "and the other is the one Resume launches: {staging}"
        );
        assert!(
            staging.contains("no handoff document"),
            "which is grounded in the Brief and the repository — a roadmap writes \
             no handoff for it to have been handed: {staging}"
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
    /// watches is the entry ticked and committed, so the box and the commit are
    /// the two things it cannot leave out.
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
            next_task.contains(r#""- [ ] NN: ..." becomes "- [x] NN: ...""#)
                && next_task.contains("git commit"),
            "the box ticked and committed is what says the task is done: {next_task}"
        );
        assert!(
            !next_task.contains("rm .tasks/NN-<slug>.md"),
            "and the task file stays where it is, going with the rest of the backlog at the \
             finish rather than one at a time: {next_task}"
        );
        assert!(
            next_task.contains("Nothing waits on approval"),
            "and there is no gate in front of that commit, as there is in front of none: \
             {next_task}"
        );
    }

    /// The finish step is the other half of what the fork decides, and the
    /// runner watches it the same way: `TODO.md` gone and committed. The whole
    /// of `.tasks/` goes with it — the task files are kept as they are worked,
    /// so the finish is the one place any of them is taken away.
    #[test]
    fn the_next_task_fork_finishes_the_feature_by_taking_the_backlog_away() {
        let next_task = skill("next-task/SKILL.md");

        assert!(
            next_task.contains("git rm -r .tasks/"),
            "taking the backlog away, list and task files together, is what says the feature \
             is finished: {next_task}"
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

    /// And the finish extends to the companions. A Conversation working alongside
    /// read-write repositories ends on one pull request per repository it
    /// committed in, opened the way *that* repository says — and the three skills
    /// that end a piece of work are the only place a session is told so.
    ///
    /// Word for word across the three, the way the commit-summary block is: what a
    /// finish does about a companion is one instruction, and three wordings of it
    /// would be three things to keep true.
    #[test]
    fn the_finish_skills_carry_every_companion_they_committed_in_to_its_own_pull_request() {
        let block = companion_block("next-task/SKILL.md");

        for named in ["implementing/SKILL.md", "staging/SKILL.md"] {
            assert_eq!(
                companion_block(named),
                block,
                "{named} should say what a finish does about a companion in the same \
             words the others do",
            );
        }

        assert!(
            block.contains("docs/agents/git-workflow.md"),
            "the process followed is the companion's own, read out of its own file: \
         {block}"
        );
        assert!(
            block.contains("worktree"),
            "and it is followed in that companion's checkout rather than this one: \
         {block}"
        );
        assert!(
            block.contains("Only the ones holding commits"),
            "a companion nobody committed in is nothing to carry anywhere: {block}"
        );
        assert!(
            block.contains("stops the run"),
            "and one committed in and left without a pull request is a stop rather \
         than something wrap-up carries on past: {block}"
        );
    }

    /// And where the finish stopped between its commit and its pull request, the
    /// session sent after it is sent for the pull request and nothing else: the
    /// work is built and committed, so a skill that read as *build the feature*
    /// would put a second run on a branch the human is about to read as finished.
    #[test]
    fn the_submitting_skill_says_the_work_is_already_built() {
        let submitting = skill("submitting/SKILL.md");

        assert!(
            submitting.contains("already built and committed"),
            "what is missing is the pull request rather than the work: {submitting}"
        );
        assert!(
            submitting.contains("Do not start anything else"),
            "so there is nothing else for this session to pick up: {submitting}"
        );
        assert!(
            !submitting.contains("lowest-numbered"),
            "and no backlog left to work through: {submitting}"
        );
        assert!(
            submitting.contains("verkstead guide") && submitting.contains("verkstead ask"),
            "the one way to the human, for what the branch cannot settle: {submitting}"
        );
    }

    /// And it opens the pull request the same way the finish step would have —
    /// the repository's own process, both shapes named, no gate in front of it.
    /// One wording across the three, because the branch does not care which
    /// session got there.
    #[test]
    fn the_submitting_skill_opens_the_pull_request_the_repositorys_own_way() {
        let submitting = skill("submitting/SKILL.md");

        assert!(
            submitting.contains("docs/agents/git-workflow.md")
                && submitting.contains("Finish sequence"),
            "the process is the repository's, read out of the file that records it: \
             {submitting}"
        );
        assert!(
            submitting.contains("gh stack submit --auto"),
            "a stacked branch is submitted as a stack: {submitting}"
        );
        assert!(
            submitting.contains("gh pr create --draft"),
            "and an unstacked one opens a draft PR of its own: {submitting}"
        );
        assert!(
            submitting.contains("Nothing waits on approval"),
            "and there is no gate in front of it, as there is in front of none: {submitting}"
        );
    }

    /// The session is put inside the skill the same way every other one is, and
    /// primed with the same two documents: the pull request is titled and
    /// described for work whose point is written in them rather than in the diff.
    #[test]
    fn a_submitting_session_is_started_on_the_documents_inside_the_skill() {
        let prompt = submitting(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# Handoff\n\nA fixed window.\n"),
        );

        assert!(
            prompt.contains(SUBMITTING),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("A fixed window."),
            "and both documents are what it is primed with: {prompt:?}"
        );
        assert!(
            !prompt.contains(NEXT_TASK) && !prompt.contains(IMPLEMENTING),
            "and nothing sends this session to work a task or build the feature again: \
             {prompt:?}"
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

    /// A fix session may be sent at a pull request that is not the one in the
    /// worktree it starts in, so the skill has to say where to work.
    ///
    /// A Conversation ends on a pull request per repository it committed in, and
    /// every one of them is watched. Both `git` and `gh` read their repository
    /// from wherever they are run, so a session sent at a companion's pull
    /// request and left where it landed would ask the wrong repository how its
    /// checks were getting on — and *do not touch any other branch* would forbid
    /// exactly the branch it was sent to.
    #[test]
    fn the_addressing_skill_sends_the_session_to_the_worktree_the_feedback_names() {
        let addressing = skill("addressing/SKILL.md");

        assert!(
            addressing.contains("worktree the feedback named")
                || addressing.contains("worktree to work in"),
            "the feedback names where to work, and the skill says to go there: \
             {addressing}"
        );
        assert!(
            addressing.contains("cd"),
            "which is a directory to change into before anything else: {addressing}"
        );
        assert!(
            addressing.contains("do not touch any branch")
                && addressing.contains("beyond the one you were sent to"),
            "and what it must leave alone is every branch but that one, rather than \
             every branch but the one it started on: {addressing}"
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
    /// The half of it the human decides: one Set, a Question per finding, and
    /// every credible way to fix it offered as an Option the human picks
    /// between — with leaving it alone always among them.
    #[test]
    fn the_reviewing_skill_says_how_a_finding_becomes_work() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("Each credible way to fix it is an Option of its own"),
            "a finding is not fix-or-leave: the ways of fixing it are what the human \
             picks between — {reviewing}"
        );
        assert!(
            reviewing.contains("Leave it is always offered"),
            "and declining stays possible on every finding: {reviewing}"
        );
        assert!(
            reviewing.contains("Recommend the one you would take"),
            "with the way the review would take starred: {reviewing}"
        );
        assert!(
            reviewing.contains("Nothing else goes on the Set") && reviewing.contains("no marker"),
            "the Set is a plain Question Set, carrying no block that says which Option \
             means fix it: {reviewing}"
        );
        assert!(
            !reviewing.contains("findings:") && !reviewing.contains("fix: Q"),
            "so nothing in it teaches the findings grammar the schema no longer reads: \
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

    /// The escape hatch: a finding too big for the sitting may offer to be split
    /// out instead — an Option beside the ways of fixing it, like any other —
    /// and what the session then writes for it is a backlog rather than a fix.
    #[test]
    fn the_reviewing_skill_offers_a_split_only_where_the_work_is_too_big() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("Split it out as its own work"),
            "splitting it out is an ordinary Option, worded like the rest: {reviewing}"
        );
        assert!(
            reviewing.contains("Offer it rarely, and never by default"),
            "and offered only where the work is genuinely too big — an ordinary review \
             carries no split at all: {reviewing}"
        );
        assert!(
            reviewing.contains("`.tasks/` backlog") && reviewing.contains("TODO.md"),
            "what a split pick is owed is a backlog, written here: {reviewing}"
        );
        assert!(
            reviewing.contains("Do not build any of it"),
            "and written rather than built, because a backlog is worked a session at a \
             time: {reviewing}"
        );
        assert!(
            reviewing.contains("Verkstead reads the branch rather than the"),
            "the committed backlog being the whole of what says it was split out: \
             {reviewing}"
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
            reviewing.contains("ask each pull request how its checks are getting on"),
            "each of them, asked where that one lives — a suite asked about from \
             the wrong worktree is somebody else's: {reviewing}"
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

    /// One review reads the whole of the work, and the whole of it may be a pull
    /// request per repository the Conversation was worked in — so the skill is
    /// written for several: a diff read in each worktree, each suite asked about
    /// where that pull request lives, and a push from every worktree it committed
    /// in.
    ///
    /// Both `git` and `gh` read their repository from wherever they are run, so a
    /// session that stayed in the worktree it started in would read one
    /// repository's half of the work twice and the companion's never.
    #[test]
    fn the_reviewing_skill_reads_every_pull_request_where_it_lives() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("The pull requests this work is on"),
            "the prompt's own listing is what names them, and the skill says so: \
             {reviewing}"
        );
        assert!(
            reviewing.contains("cd <the worktree that pull request is in>"),
            "each of them read where it lives, which is a directory to change into \
             first: {reviewing}"
        );
        assert!(
            reviewing.contains("Where your prompt lists none"),
            "and a Conversation that touched nothing else reviews the branch it is \
             standing in, exactly as it always did: {reviewing}"
        );
        assert!(
            reviewing.contains("push each worktree you committed in"),
            "a repository it fixed something in and did not push is a decision \
             nobody can see: {reviewing}"
        );
        assert!(
            reviewing.contains("do not touch any branch")
                && reviewing.contains("beyond the ones you were sent to"),
            "and what it must leave alone is every branch but those, rather than \
             every branch but the one it started on: {reviewing}"
        );
    }

    /// One Set across the whole of the work, whatever repositories it reached —
    /// and a finding about a companion says which repository it is about, so the
    /// Option the human picks says what would change and where.
    ///
    /// The backlog is the one thing that does not move: Verkstead reads it off the
    /// Conversation's own branch, so a list written in a companion's worktree is
    /// work nothing would ever start.
    #[test]
    fn the_reviewing_skill_puts_one_set_across_the_repositories() {
        let reviewing = skill("reviewing/SKILL.md");

        assert!(
            reviewing.contains("One Set for the whole of the work"),
            "one review, one Set, however many repositories it read: {reviewing}"
        );
        assert!(
            reviewing.contains("names the repository it is about"),
            "with a finding about a companion saying so, so the pick says what \
             would change and where: {reviewing}"
        );
        assert!(
            reviewing.contains("In the worktree you started in"),
            "and the backlog written where Verkstead reads one from, whichever \
             repository the split-out finding is about: {reviewing}"
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
        assert!(
            !prompt.contains("The pull requests this work is on"),
            "and a Conversation whose work touched nothing else is told what it has \
             always been told, the branch this worktree is on being the whole of it: \
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
            None,
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

    /// A review of work that reached more than one repository is told where every
    /// one of its pull requests is, between the documents and what was said on
    /// them.
    ///
    /// One review reads the whole of the work, and a session that started in the
    /// Conversation's own worktree could not find the other half of it: `git` and
    /// `gh` both read their repository from wherever they are run.
    #[test]
    fn a_review_session_is_told_every_pull_request_the_work_is_on() {
        let prompt = reviewing(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            Some(
                "- pull request #41 of `verkstead`, at https://github.com/tobico/verkstead/pull/41 \
                 — its worktree is at `/srv/work/verkstead-rate-limiting`.\n\
                 - pull request #7 of `askance`, at https://github.com/tobico/askance/pull/7 — \
                 its worktree is at `/srv/work/askance-rate-limiting`.",
            ),
            Some(
                "**tobico** said on pull request #7 of `askance`:\n\nThis is the wrong way round.",
            ),
        );

        assert!(
            prompt.contains("#41") && prompt.contains("#7") && prompt.contains("askance"),
            "each pull request by its number and the repository it was opened in: \
             {prompt:?}"
        );
        assert!(
            prompt.contains("https://github.com/tobico/askance/pull/7")
                && prompt.contains("`/srv/work/askance-rate-limiting`"),
            "with the URL and the worktree to read it in: {prompt:?}"
        );
        assert!(
            prompt.find("In-process counter.") < prompt.find("#41"),
            "under the documents, which say what the work is: {prompt:?}"
        );
        assert!(
            prompt.find("#41") < prompt.find("This is the wrong way round."),
            "and over what was said on them, which is the newest and least general \
             thing here — {prompt:?}"
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
    /// worth doing something about, and every credible way of doing it offered
    /// as an Option — with leaving it alone always among them.
    #[test]
    fn the_responding_skill_says_how_a_comment_becomes_work() {
        let responding = skill("responding/SKILL.md");

        assert!(
            responding.contains("Each credible way to do it is an Option of its own"),
            "a comment is not do-it-or-leave: the ways of doing it are what the human \
             picks between — {responding}"
        );
        assert!(
            responding.contains("Leave it is always offered"),
            "and declining stays possible on every one of them: {responding}"
        );
        assert!(
            responding.contains("Nothing else goes on the Set") && responding.contains("no marker"),
            "the Set is a plain Question Set, carrying no block that says which Option \
             means do it: {responding}"
        );
        assert!(
            !responding.contains("findings:") && !responding.contains("fix: Q"),
            "so nothing in it teaches the findings grammar the schema no longer reads: \
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
        assert!(
            responding.contains("Nothing is split out here"),
            "and the escape hatch is the review's alone: a batch session that split \
             something out would be owed a backlog nobody reads it for — {responding}"
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

    /// And a Conversation whose human picked *No grilling* is told so, which is
    /// a different thing from a handoff that failed to arrive: the Brief is the
    /// plan, and what it leaves open is asked about rather than guessed at.
    #[test]
    fn an_ungrilled_implementation_is_told_the_brief_is_the_whole_plan() {
        let prompt = ungrilled("# Rate limiting\n\nThe API has none.\n");

        assert!(
            prompt.contains(IMPLEMENTING),
            "the same skill an ordinary inline run reads: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none."),
            "primed with the Brief, whole: {prompt:?}"
        );
        assert!(
            !prompt.contains("What the grilling settled"),
            "and with no handoff, there having been no interview: {prompt:?}"
        );
        assert!(
            prompt.contains("Nothing was grilled") && prompt.contains("blocking ask"),
            "said in words, along with what to do about what the Brief leaves \
             open: {prompt:?}"
        );
        assert!(
            prompt.find("The API has none.") < prompt.find("Nothing was grilled"),
            "under the Brief, which is what it is about"
        );
    }

    /// What the instruction skill has to say that no other working skill here
    /// does: the pipeline carries on from here.
    ///
    /// The whole reason it is a skill of its own. A session told that what
    /// happens to the branch next is the human's would be one that lined the
    /// work up for somebody to come and look at it, and what actually follows an
    /// instruction session is Verkstead reading the branch and starting the next
    /// thing.
    #[test]
    fn the_instruction_skill_hands_what_follows_to_the_pipeline() {
        let instruction = skill("instruction/SKILL.md");

        assert!(
            instruction.contains("The pipeline carries on after you"),
            "what follows this session is the machine's rather than its own: {instruction}"
        );
        assert!(
            !instruction.contains("the human's to decide"),
            "rather than a branch left lined up for somebody to come and look at: \
             {instruction}"
        );
        assert!(
            !instruction.contains("gh pr create"),
            "and it does not carry the branch anywhere itself: {instruction}"
        );
    }

    /// And what it has to leave behind, which is the same thing every working
    /// skill here leaves behind: a commit, made without asking anybody.
    ///
    /// Committing is also how this one *reports*. Nothing reads what an
    /// instruction session prints to decide whether the work landed — what is
    /// read is the branch — so a session that changed files and left them
    /// uncommitted is one that stops the Conversation.
    #[test]
    fn the_instruction_skill_says_to_commit_and_then_stop() {
        let instruction = skill("instruction/SKILL.md");

        assert!(
            instruction.contains("git commit"),
            "committing what it changed is how the session reports: {instruction}"
        );
        assert!(
            instruction.contains("Nothing waits on approval"),
            "and there is no gate in front of it, as there is in front of none: \
             {instruction}"
        );
        assert!(
            instruction.contains("Keep to what was asked"),
            "the scope is the instruction and nothing beside it: {instruction}"
        );
        assert!(
            instruction.contains("do not start the next task"),
            "and the step after this one belongs to the session Verkstead starts for \
             it: {instruction}"
        );
    }

    /// An instruction session is put inside its own skill by its prompt, and
    /// primed with the documents *and* the instruction.
    ///
    /// The documents because it works the same branch as everything before it,
    /// and the instruction last because it is the newest thing said and the
    /// least general: they say what the work is, and it says what to do about it
    /// now.
    #[test]
    fn an_instruction_session_is_started_on_the_documents_and_the_instruction() {
        let prompt = instruction(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            "Move the counter into Redis.\n",
        );

        assert!(
            prompt.contains(INSTRUCTION),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            !prompt.contains(IMPLEMENTING),
            "and not the inline implementation's, which says something else about \
             what follows: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole, this being a session on the same work as \
             every other: {prompt:?}"
        );
        assert!(
            prompt.ends_with("# What I have asked for\n\nMove the counter into Redis.\n"),
            "and the instruction is the last thing said, under them: {prompt:?}"
        );
    }

    /// The follow-up brief is the human's own words typed at this session, so it
    /// is acted on rather than put back to them — the instruction doctrine, and
    /// not responding's propose-everything-first.
    #[test]
    fn the_following_up_skill_acts_on_the_brief_rather_than_proposing_it_back() {
        let following_up = skill("following-up/SKILL.md");

        assert!(
            following_up.contains("The brief is written to this session"),
            "the words are theirs and they are aimed here: {following_up}"
        );
        assert!(
            following_up.contains("ambiguous, destructive, or beyond"),
            "and what is asked about first is only that: {following_up}"
        );
        assert!(
            !following_up.contains("Change nothing yet"),
            "nothing is held back for a proposal round, unlike a batch of comments: \
             {following_up}"
        );
    }

    /// And what it does with the human is what every other session does: an
    /// ordinary Set, put through the CLI, with the answers they are owed leading
    /// it so that each round reaches their phone.
    #[test]
    fn the_following_up_skill_runs_rounds_of_ordinary_question_sets() {
        let following_up = skill("following-up/SKILL.md");

        assert!(
            following_up.contains("verkstead guide") && following_up.contains("verkstead ask"),
            "the Guide is where an agent learns to ask, and the CLI is how a Set goes: \
             {following_up}"
        );
        assert!(
            following_up.contains("ordinary Question Set"),
            "nothing about this session's Sets is special: {following_up}"
        );
        assert!(
            following_up.contains("The answers lead"),
            "and what they asked, answered, is what opens each one: {following_up}"
        );
        assert!(
            following_up.contains("`postscript` is an ordinary postscript"),
            "the close of the Set is the close of any Set: {following_up}"
        );
        assert!(
            following_up.contains("go round again"),
            "and one Set is a round rather than the session: {following_up}"
        );
    }

    /// Each round's work is pushed before the next ask, so the pull request shows
    /// what has been done and its checks run while the human composes.
    #[test]
    fn the_following_up_skill_pushes_each_round_before_it_asks() {
        let following_up = skill("following-up/SKILL.md");

        assert!(
            following_up.contains("git push"),
            "this branch is already on a pull request, so a round that stayed local \
             is one nobody can see: {following_up}"
        );
        assert!(
            following_up.contains("before you ask them anything"),
            "and it goes before the Set rather than after the answers: {following_up}"
        );
        assert!(
            !following_up.contains("gh pr create"),
            "the pull request exists, and this session opens nothing: {following_up}"
        );
    }

    /// How a follow-up ends is the system's business: the mark rides the human's
    /// Response and never reaches the agent, so the skill has nothing to say
    /// about it and must not invent a mechanism of its own.
    #[test]
    fn the_following_up_skill_says_nothing_about_how_a_follow_up_ends() {
        let following_up = skill("following-up/SKILL.md");

        assert!(
            following_up.contains("finish your turn"),
            "it simply stops asking when it has nothing to ask: {following_up}"
        );
        for ending in ["Nothing else", "Wrapping", "Done"] {
            assert!(
                !following_up.contains(ending),
                "and {ending} is Verkstead's rather than the session's to know about: \
                 {following_up}"
            );
        }
        assert!(
            !following_up.contains("gh pr ready") && !following_up.contains("gh pr merge"),
            "nor does it wrap anything up itself: {following_up}"
        );
    }

    /// A follow-up session is put inside the skill by its prompt, primed with the
    /// two documents, and told last what the human wants followed up.
    #[test]
    fn a_follow_up_session_is_started_on_the_documents_and_the_brief() {
        let prompt = following_up(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
            "Why is the window a minute? And add a header saying when it resets.\n",
            "",
        );

        assert!(
            prompt.contains(FOLLOWING_UP),
            "the skill is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            !prompt.contains(INSTRUCTION) && !prompt.contains(RESPONDING),
            "and no other skill is named: a follow-up is neither a one-shot \
             instruction nor a batch of comments — {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole, this being a session on the same work as \
             every other: {prompt:?}"
        );
        assert!(
            prompt.ends_with(
                "# What I want to follow up on\n\nWhy is the window a minute? And add a \
                 header saying when it resets.\n"
            ),
            "and the follow-up brief is the last thing said, under them: {prompt:?}"
        );
    }

    /// And a follow-up picked up again is the same session over: the same skill
    /// and the same brief, with the rounds it has already been through under
    /// them, so that it does not open by asking what was answered an hour ago.
    #[test]
    fn a_relaunched_follow_up_is_told_what_it_has_already_asked() {
        let prompt = following_up(
            "# Rate limiting\n\nThe API has none.\n",
            None,
            "Why is the window a minute?\n",
            "## About the window\n\n**Q9** Is a minute right?\n\nYes\n",
        );

        assert!(
            prompt.contains("# What I want to follow up on\n\nWhy is the window a minute?"),
            "the brief it was opened with is what it is still about: {prompt:?}"
        );
        assert!(
            prompt.ends_with(
                "# What you have already asked, and what I said\n\n## About the window\n\n\
                 **Q9** Is a minute right?\n\nYes\n"
            ),
            "and the rounds already answered come last, under it: {prompt:?}"
        );
    }

    /// A Conversation steered into Follow-up may have Deferred Answers nothing
    /// has been told about yet, and they fold in here as they do everywhere.
    #[test]
    fn deferred_answers_fold_into_a_following_up_prompt() {
        let prompt = folded(
            &following_up(
                "# Rate limiting\n\nThe API has none.\n",
                None,
                "Add a header saying when the window resets.\n",
                "",
            ),
            "## The wording\n\n**Q9** Which status?\n\n429 Too Many Requests\n",
        );

        assert!(
            prompt.contains("# What I want to follow up on"),
            "the brief the follow-up was steered with is still there: {prompt:?}"
        );
        assert!(
            prompt.ends_with("**Q9** Which status?\n\n429 Too Many Requests\n"),
            "and what the human has since decided comes last, as in every other \
             prompt: {prompt:?}"
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

    /// The Answers to a Deferred Ask go where the newest and least general
    /// thing said goes: under the documents the prompt is built from.
    #[test]
    fn deferred_answers_are_folded_under_the_documents() {
        let prompt = folded(
            &next_task("# Rate limiting\n\nThe API has none.\n", None),
            "## The wording\n\n**Q9** Which status?\n\n429 Too Many Requests\n",
        );

        assert!(
            prompt.contains("# The Brief this started from"),
            "the work is still what the session is being told about: {prompt:?}"
        );
        assert!(
            prompt.ends_with("**Q9** Which status?\n\n429 Too Many Requests\n"),
            "and what the human has since decided about it comes last: {prompt:?}"
        );
    }

    /// Which is every session of an ordinary run: a heading over an empty digest
    /// would tell one that something had been decided.
    #[test]
    fn a_session_with_nothing_to_fold_is_started_on_the_prompt_as_it_stands() {
        let prompt = next_task("# Rate limiting\n\nThe API has none.\n", None);

        assert_eq!(folded(&prompt, "  \n"), prompt);
    }

    /// A companion of a Conversation, made by hand: the store is where one comes
    /// from, and what the listing is written against is the shape rather than
    /// the query.
    fn companion(
        name: &str,
        mode: store::CompanionMode,
        branch: &str,
        worktree: &str,
    ) -> store::Companion {
        store::Companion {
            repo: store::Repo {
                id: 7,
                path: PathBuf::from(format!("/home/tobi/src/{name}")),
                name: name.to_owned(),
                default_branch: "main".to_owned(),
            },
            mode,
            base_ref: None,
            branch: branch.to_owned(),
            worktree: Some(PathBuf::from(worktree)),
            base_commit: Some(COMMIT.to_owned()),
        }
    }

    /// What a companion's base resolved to when it was checked out, which is
    /// what a detached one is named by.
    const COMMIT: &str = "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7";

    /// What a session is told about the companions: where each one is, what it
    /// holds and whether it may be written to, under one heading and under
    /// whatever the prompt already said.
    #[test]
    fn every_companion_is_named_with_its_path_its_branch_and_its_write_status() {
        let prompt = alongside(
            &next_task("# Rate limiting\n\nThe API has none.\n", None),
            "rate-limiting",
            &[
                companion(
                    "askance",
                    store::CompanionMode::ReadOnly,
                    "",
                    "/var/lib/verkstead/worktrees/askance-main",
                ),
                companion(
                    "tobico-skills",
                    store::CompanionMode::ReadWrite,
                    "",
                    "/var/lib/verkstead/worktrees/tobico-skills-rate-limiting",
                ),
            ],
        );

        assert!(
            prompt.contains("# The Brief this started from"),
            "the work is still what the session is being told about: {prompt:?}"
        );
        assert_eq!(
            prompt.matches("# Companion repositories").count(),
            1,
            "one listing, whatever the prompt was built by: {prompt:?}"
        );
        assert!(
            prompt.contains(&format!(
                "- `askance` at `/var/lib/verkstead/worktrees/askance-main`, \
                 detached at `{COMMIT}`, read-only."
            )),
            "a read-only companion is detached at the commit its base came to, \
             rather than at a branch name that has moved on since: {prompt:?}"
        );
        assert!(
            prompt.contains(
                "- `tobico-skills` at \
                 `/var/lib/verkstead/worktrees/tobico-skills-rate-limiting`, on branch \
                 `rate-limiting`, read-write."
            ),
            "and a read-write one is on the branch cut for it, mirroring the \
             Conversation's: {prompt:?}"
        );
    }

    /// The first session of a Conversation nobody has named is told to name the
    /// branch, under whatever it was already being told.
    #[test]
    fn the_first_session_of_an_unnamed_conversation_is_told_to_name_the_branch() {
        let prompt = naming(&grilling("# Rate limiting\n\nThe API has none.\n"), true);

        assert!(
            prompt.contains("# Rate limiting"),
            "the Brief is still what the session is being grilled about: {prompt:?}"
        );
        assert!(
            prompt.contains("# This branch has no name yet"),
            "and under it, the one thing to do first: {prompt:?}"
        );
        assert!(
            prompt.contains("`git branch -m <name>`"),
            "said as the command that does it, in this worktree: {prompt:?}"
        );
        assert!(
            prompt.contains("kebab-case"),
            "and the shape of the name to pick: {prompt:?}"
        );
    }

    /// And it is told to get on with it rather than to come back about it: a
    /// rename is read off the checkout, so there is nobody to ask.
    #[test]
    fn the_naming_instruction_asks_for_nothing_back() {
        let prompt = naming(&ungrilled("# Rate limiting\n"), true);

        assert!(
            prompt.contains("There is nobody to ask and nothing to report"),
            "the rename reports itself: {prompt:?}"
        );
        assert_eq!(
            prompt.matches("# This branch has no name yet").count(),
            1,
            "one instruction, whatever the prompt was built by: {prompt:?}"
        );
    }

    /// And a session on a branch nobody is waiting to see named is told nothing
    /// about it: the human typed the name, or the first session has been and
    /// gone.
    #[test]
    fn a_session_on_a_settled_branch_is_told_nothing_about_naming_it() {
        let built = implementing("# Rate limiting\n", Some("Use a token bucket.\n"));

        assert_eq!(
            naming(&built, false),
            built,
            "a heading over nothing would tell a session the name was in question",
        );
    }

    /// The listing says what is there and nothing about what to do with it. What
    /// the work is, is the Brief's to say — a prompt that told a session to go
    /// and use a repository would be Verkstead deciding the work off a
    /// configuration screen.
    #[test]
    fn the_listing_tells_a_session_nothing_about_what_to_do_with_them() {
        let prompt = alongside(
            "",
            "rate-limiting",
            &[companion(
                "askance",
                store::CompanionMode::ReadOnly,
                "",
                "/var/lib/verkstead/worktrees/askance-main",
            )],
        );

        for instructed in [
            "you should",
            "use it",
            "make sure",
            "read the",
            "nix develop",
        ] {
            assert!(
                !prompt.to_lowercase().contains(instructed),
                "the listing is neutral, and {instructed:?} is not: {prompt:?}"
            );
        }
    }

    /// A companion named with a branch of its own is on that branch rather than
    /// on the Conversation's — mirroring is what an empty name means, not what
    /// every name means.
    #[test]
    fn a_companion_with_a_branch_of_its_own_is_listed_on_it() {
        let prompt = alongside(
            "",
            "rate-limiting",
            &[companion(
                "askance",
                store::CompanionMode::ReadWrite,
                "the-typed-one",
                "/var/lib/verkstead/worktrees/askance-the-typed-one",
            )],
        );

        assert!(
            prompt.contains("on branch `the-typed-one`"),
            "a typed branch name stands on its own: {prompt:?}"
        );
    }

    /// Which is most Conversations: one repository is what most work needs, and
    /// a heading over an empty list would tell a session that something had been
    /// configured.
    #[test]
    fn a_conversation_with_no_companions_is_started_on_the_prompt_as_it_stands() {
        let prompt = next_task("# Rate limiting\n\nThe API has none.\n", None);

        assert_eq!(alongside(&prompt, "rate-limiting", &[]), prompt);
    }

    /// The workbench shows a commit's message body beside its diff, and nothing
    /// else tells a session to write one — so every skill that commits work has
    /// to say it. One wording across the seven, because seven wordings would be
    /// seven things to keep true and the human reads them as one convention.
    #[test]
    fn every_skill_that_commits_work_asks_for_the_commits_summary() {
        let block = summary_block("next-task/SKILL.md");

        for name in [
            "implementing/SKILL.md",
            "instruction/SKILL.md",
            "addressing/SKILL.md",
            "reviewing/SKILL.md",
            "responding/SKILL.md",
            "following-up/SKILL.md",
        ] {
            assert_eq!(
                summary_block(name),
                block,
                "{name} should carry the same block, word for word"
            );
        }
    }

    /// And what the block says, held once: which commits get a summary, when the
    /// diagram is required, what kind of diagram it is, and that it comes after
    /// the prose.
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
            block.contains("tick rides along with the code"),
            "a task's commit is still a delivering one, the list's tick and all: {block}"
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
            block.find("The prose first") < block.find("The diagram after it"),
            "the words come first and the picture is checked against them, in the block \
             as in the message: {block}"
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

    /// And the empty directory beside them, which is what covers the account's
    /// own skills now that the mount doing that has moved to a path of
    /// Verkstead's own.
    #[test]
    fn an_empty_directory_is_installed_for_the_accounts_own_to_be_hidden_behind() {
        let state = tempfile::tempdir().unwrap();

        let skills = Skills::installed(state.path()).expect("this binary carries skills");

        assert_eq!(skills.nothing(), state.path().join("nothing"));
        assert_eq!(
            std::fs::read_dir(skills.nothing()).unwrap().count(),
            0,
            "what is bound over an account's skills has to hold nothing of its own"
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
