//! The skills Verkstead ships, and how a session comes to be running one.
//!
//! A session's behaviour should be the product's rather than whatever happens
//! to be installed on the machine it runs on. So Verkstead carries its own
//! skills, embedded in the binary as the viewer is (ADR-0004), writes them out
//! under the State Directory at startup, and every sandbox binds that directory
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

/// And the fork of next-task, which every session the runner launches is put
/// inside — the task sessions and the finish one alike, because which of them it
/// is, is read off `.tasks/` rather than told.
const NEXT_TASK: &str = "~/.claude/skills/next-task/SKILL.md";

/// The bundled skills, installed on the host, ready for a sandbox to bind.
#[derive(Debug, Clone)]
pub struct Skills {
    path: PathBuf,
}

impl Skills {
    /// Write them out under `state_dir`, replacing whatever is already there.
    ///
    /// Replaced rather than written over, so that what a session finds is what
    /// this binary ships and not the union of that with every binary that ran
    /// here before: a skill withdrawn from the product should stop being a skill
    /// sessions are run under.
    ///
    /// Under the State Directory because that is the one place Verkstead is
    /// given to write, and beside the worktrees for the same reason they are
    /// there: this is something Verkstead made rather than something the human
    /// pointed it at.
    ///
    /// Refused where the binary carries none. A grilling session with no
    /// grilling skill is a session that has been told to read a file that is not
    /// there, and a server that starts anyway would be one whose every
    /// Conversation fails at the far end of the button.
    pub fn installed(state_dir: &Path) -> Result<Skills> {
        let path = state_dir.join("skills");

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

/// What a task-list session is started on: the same two documents, under the
/// line that sends the agent into the breakdown skill instead.
///
/// The same documents because it is the same work, read for a different purpose:
/// this session is not building it but deciding how it splits, and a breakdown
/// drawn from anything less than what the grilling settled would be a backlog for
/// some other piece of work.
pub(crate) fn breaking_down(brief: &str, handoff: Option<&str>) -> String {
    on_the_documents(
        &format!(
            "Read {BREAKING_DOWN} and break the work described below into tasks, the way it says."
        ),
        brief,
        handoff,
    )
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

    /// The same two documents, into the other skill. What differs is the line
    /// above them, which is the whole of what sends a session one way or the
    /// other.
    #[test]
    fn a_breakdown_session_is_started_on_the_same_documents_inside_the_fork() {
        let prompt = breaking_down(
            "# Rate limiting\n\nThe API has none.\n",
            Some("# What we settled\n\nIn-process counter.\n"),
        );

        assert!(
            prompt.contains(BREAKING_DOWN),
            "the fork is named by the path it is mounted at: {prompt:?}"
        );
        assert!(
            !prompt.contains(IMPLEMENTING),
            "and nothing sends this session to build the work instead: {prompt:?}"
        );
        assert!(
            prompt.contains("The API has none.") && prompt.contains("In-process counter."),
            "both documents go in whole: {prompt:?}"
        );
    }

    /// Installing is what puts them where a sandbox can bind them.
    #[test]
    fn the_skills_are_written_out_under_the_state_directory() {
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
