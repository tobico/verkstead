//! What is in front of a terminal opened for a Preview: its run, its shell, or
//! something else ([ADR 0024](../../../../docs/adr/0024-a-session-suggests-a-preview.md),
//! *Typed into a terminal, as the human would have*).
//!
//! **The run is a process group.** Once the Preview's command has been typed,
//! the first group in the pseudo-terminal's foreground that is not the shell's
//! is recorded as the run — `npm run dev` has `node` in front, so the group is
//! what says it and a name would not. From then on what is in front is one of
//! three things: that group, which is the run; the shell, which is the run
//! having ended or been stopped; or any other group, which is the human having
//! started a command of their own. Where nothing can be read at all — Windows,
//! whose pseudoconsole has no foreground — it is none of them, and says so.
//!
//! **The moments either side of typing read as the run.** Before the shell has
//! come up the foreground is the sandbox wrapper, and nothing has been typed to
//! be running; just after, the shell is still in front until it has read the
//! line. Both are the run on its way, which is what the tab is called by and
//! what a press leaves alone — a second press then typing the command twice
//! would be the one thing worse than waiting. A command that has come and gone
//! before any reading saw it would hold that forever, so the shell is given
//! [`STARTING`] to be seen leaving the foreground, and after that it is the
//! shell in front.
//!
//! This is the bookkeeping alone: what the foreground *is* is [`super::busy`]'s
//! to read, and when it is read is the register's — see [`crate::terminals`].

use std::time::Duration;

use tokio::time::Instant;
use verkstead_render::InFront;

use super::busy::Front;

/// How long the shell is given, after a command is typed into it, to have
/// started running it before the shell in front is read as the run having
/// ended.
///
/// A shell reads a typed line at once on any ordinary machine, and a login
/// shell starting its first prompt has already been waited out before anything
/// was typed. What this is for is the command that was over before a reading
/// saw it — a build with nothing to do — and a few seconds of the row saying
/// *running* about one of those is the cheap side of the mistake: the other is
/// the command typed twice into a shell that had not got round to it.
pub(crate) const STARTING: Duration = Duration::from_secs(20);

/// One Preview terminal's run, as the register holds it.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    stage: Stage,
    in_front: InFront,
}

/// Where the run has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// The shell has not come up, and nothing has been typed.
    ComingUp,

    /// Typed at this moment, and no group other than the shell seen in front
    /// since.
    Typed(Instant),

    /// The run, by the id of the process leading its group. Kept after the
    /// shell comes back in front, so that a run stopped with Ctrl+Z and brought
    /// back with `fg` is the run again rather than something else.
    Recorded(u32),

    /// What was typed was never seen start, and there is no run to recognise.
    Unseen,
}

impl Run {
    /// A terminal just opened for a Preview, whose shell is still coming up.
    pub(crate) fn new() -> Run {
        Run {
            stage: Stage::ComingUp,
            in_front: InFront::Run,
        }
    }

    /// The command has been typed into it, now — the first time, or a press
    /// with the shell in front typing it again, which is a new run.
    pub(crate) fn typed(&mut self, now: Instant) {
        self.stage = Stage::Typed(now);
        self.in_front = InFront::Run;
    }

    /// Whether the run is waiting to be seen start, which is when the terminal
    /// is read quickly and whether or not anybody is looking — see
    /// [`crate::terminals`].
    pub(crate) fn starting(&self) -> bool {
        matches!(self.stage, Stage::ComingUp | Stage::Typed(_))
    }

    /// What is in front, as last read.
    pub(crate) fn in_front(&self) -> InFront {
        self.in_front
    }

    /// Take a reading of the foreground, and say whether what is in front
    /// changed with it.
    pub(crate) fn read(&mut self, front: Front, now: Instant) -> bool {
        let in_front = match (front, self.stage) {
            (Front::Unreadable, _) => InFront::Unknown,

            // The wrapper, or the shell itself before its first prompt: the
            // run on its way, nothing having been typed yet to be anything else.
            (_, Stage::ComingUp) => InFront::Run,

            (Front::Group(group), Stage::Recorded(run)) if group == run => InFront::Run,

            (Front::Group(group), Stage::Typed(_)) => {
                self.stage = Stage::Recorded(group);
                InFront::Run
            }

            (Front::Group(_), _) => InFront::Other,

            (Front::Shell, Stage::Typed(at)) if now.duration_since(at) < STARTING => InFront::Run,

            (Front::Shell, Stage::Typed(_)) => {
                self.stage = Stage::Unseen;
                InFront::Shell
            }

            (Front::Shell, Stage::Recorded(_) | Stage::Unseen) => InFront::Shell,
        };

        let changed = in_front != self.in_front;
        self.in_front = in_front;

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHELL: Front = Front::Shell;
    const WRAPPER: Front = Front::Group(10);
    const RUN: Front = Front::Group(42);
    const THEIRS: Front = Front::Group(77);

    /// A terminal coming up reads as the run, whatever is in front, until
    /// anything has been typed into it.
    #[test]
    fn a_terminal_coming_up_is_the_run_on_its_way() {
        let now = Instant::now();
        let mut run = Run::new();

        assert!(!run.read(WRAPPER, now));
        assert!(!run.read(SHELL, now));
        assert_eq!(run.in_front(), InFront::Run);
        assert!(run.starting());
    }

    /// The first group in front after typing is the run, and stays the run.
    #[test]
    fn the_first_group_after_typing_is_recorded_as_the_run() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);

        assert!(
            !run.read(SHELL, now),
            "the shell has a moment to read the line"
        );
        assert!(!run.read(RUN, now));
        assert!(!run.starting());
        assert_eq!(run.in_front(), InFront::Run);

        // Ended, with Ctrl+C: the shell is back in front.
        assert!(run.read(SHELL, now));
        assert_eq!(run.in_front(), InFront::Shell);

        // And the human starts something of their own.
        assert!(run.read(THEIRS, now));
        assert_eq!(run.in_front(), InFront::Other);
    }

    /// A run stopped and brought back is still the run.
    #[test]
    fn a_run_brought_back_to_the_front_is_the_run_again() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);
        run.read(RUN, now);
        run.read(SHELL, now);

        assert!(run.read(RUN, now));
        assert_eq!(run.in_front(), InFront::Run);
    }

    /// Typed again with the shell in front, it is a new run, recorded afresh.
    #[test]
    fn typing_again_starts_a_new_run() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);
        run.read(RUN, now);
        run.read(SHELL, now);

        run.typed(now);
        assert_eq!(run.in_front(), InFront::Run);

        assert!(!run.read(Front::Group(43), now));
        assert_eq!(run.in_front(), InFront::Run);

        assert!(run.read(RUN, now));
        assert_eq!(
            run.in_front(),
            InFront::Other,
            "the old run is not this one"
        );
    }

    /// A command that was over before any reading saw it is the shell in front
    /// once the shell has had its moment.
    #[test]
    fn a_run_never_seen_is_the_shell_after_starting() {
        let typed = Instant::now();
        let mut run = Run::new();
        run.typed(typed);

        assert!(!run.read(SHELL, typed + STARTING / 2));
        assert!(run.read(SHELL, typed + STARTING));
        assert_eq!(run.in_front(), InFront::Shell);
        assert!(!run.starting());
    }

    /// Where no foreground can be read, nothing is said.
    #[test]
    fn an_unreadable_foreground_is_unknown() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);

        assert!(run.read(Front::Unreadable, now));
        assert_eq!(run.in_front(), InFront::Unknown);
    }
}
