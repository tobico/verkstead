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
