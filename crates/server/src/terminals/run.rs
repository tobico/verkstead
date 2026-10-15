//! What is in front of a terminal opened for a Preview: its run, its shell, or
//! something else ([ADR 0024](../../../../docs/adr/0024-a-session-suggests-a-preview.md),
//! *Typed into a terminal, as the human would have*).
//!
//! **The run is every process group in front from the typing to the shell's
//! return.** Once the Preview's command has been typed, whatever takes the
//! pseudo-terminal's foreground that is not the shell is the run — `npm run
//! dev` has `node` in front, so a group is what says it and a name would not —
//! and that is *every* group until the shell is back, because a shell gives
//! each step of `cargo build && cargo run -- serve` a group of its own, and the
//! server that step starts is as much the run as the build before it was. From
//! the shell's return what is in front is one of three things: one of the run's
//! groups again, which is the run brought back with `fg`; the shell, which is
//! the run having ended or been stopped; or any other group, which is the human
//! having started a command of their own. Where nothing can be read at all —
//! Windows and macOS, where no foreground can be named — it is none of them,
//! and says so.
//!
//! **The shell is back once it holds the front.** Between the steps of a chain
//! the shell is in front for a moment, deciding whether to run the next, and a
//! reading that happened on that moment is not the run having ended. So the
//! shell seen in front during a run is the run still, until it is seen there
//! again on a reading at least [`SETTLING`] later with no group in between;
//! the run is read quickly while that is unsettled, as it is while it is still
//! to be seen start.
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

/// How long the shell has to hold the front during a run before the run is
/// read as over.
///
/// What it has to outlast is a shell between the steps of a chain, which
/// starts the next one as soon as the last has been reaped — a moment far
/// under this. And what it costs is a press in that window only refocusing,
/// the run being read every tenth of a second until it is settled.
pub(crate) const SETTLING: Duration = Duration::from_millis(300);

/// One Preview terminal's run, as the register holds it.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    stage: Stage,
    in_front: InFront,
}

/// Where the run has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Stage {
    /// The shell has not come up, and nothing has been typed.
    ComingUp,

    /// Typed at this moment, and no group other than the shell seen in front
    /// since.
    Typed(Instant),

    /// The run, by the id of the process leading each group seen in front
    /// since the typing, and when the shell was seen back in front where it
    /// has been since the last of them.
    Running {
        groups: Vec<u32>,
        back: Option<Instant>,
    },

    /// The shell back in front for good. The run's groups are kept, so that a
    /// run stopped with Ctrl+Z and brought back with `fg` is the run again
    /// rather than something else — and are none where what was typed was
    /// never seen start.
    Ended(Vec<u32>),
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

    /// Whether the run is to be read quickly, and whether or not anybody is
    /// looking — see [`crate::terminals`]: while it is still to be seen start,
    /// and while the shell is back in front and not yet settled there.
    pub(crate) fn starting(&self) -> bool {
        matches!(
            self.stage,
            Stage::ComingUp | Stage::Typed(_) | Stage::Running { back: Some(_), .. }
        )
    }

    /// What is in front, as last read.
    pub(crate) fn in_front(&self) -> InFront {
        self.in_front
    }

    /// Take a reading of the foreground, and say whether what is in front
    /// changed with it.
    pub(crate) fn read(&mut self, front: Front, now: Instant) -> bool {
        let in_front = match (front, &mut self.stage) {
            (Front::Unreadable, _) => InFront::Unknown,

            // The wrapper, or the shell itself before its first prompt: the
            // run on its way, nothing having been typed yet to be anything else.
            (_, Stage::ComingUp) => InFront::Run,

            (Front::Group(group), Stage::Typed(_)) => {
                self.stage = Stage::Running {
                    groups: vec![group],
                    back: None,
                };
                InFront::Run
            }

            // Any group at all until the shell is back: the next step of a
            // chain is the run as much as the first was.
            (Front::Group(group), Stage::Running { groups, back }) => {
                if !groups.contains(&group) {
                    groups.push(group);
                }
                *back = None;
                InFront::Run
            }

            (Front::Group(group), Stage::Ended(groups)) if groups.contains(&group) => {
                self.stage = Stage::Running {
                    groups: std::mem::take(groups),
                    back: None,
                };
                InFront::Run
            }

            (Front::Group(_), Stage::Ended(_)) => InFront::Other,

            (Front::Shell, Stage::Typed(at)) if now.duration_since(*at) < STARTING => InFront::Run,

            (Front::Shell, Stage::Typed(_)) => {
                self.stage = Stage::Ended(Vec::new());
                InFront::Shell
            }

            (
                Front::Shell,
                Stage::Running {
                    back: back @ None, ..
                },
            ) => {
                *back = Some(now);
                InFront::Run
            }

            (
                Front::Shell,
                Stage::Running {
                    groups,
                    back: Some(since),
                },
            ) if now.duration_since(*since) >= SETTLING => {
                self.stage = Stage::Ended(std::mem::take(groups));
                InFront::Shell
            }

            (Front::Shell, Stage::Running { .. }) => InFront::Run,

            (Front::Shell, Stage::Ended(_)) => InFront::Shell,
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
    const NEXT: Front = Front::Group(43);
    const THEIRS: Front = Front::Group(77);

    /// A moment between two readings, well under [`SETTLING`].
    const MOMENT: Duration = Duration::from_millis(50);

    /// The shell seen back in front, and seen there again once it has held it:
    /// the run over.
    fn settled(run: &mut Run, now: Instant) -> bool {
        run.read(SHELL, now);
        run.read(SHELL, now + SETTLING)
    }

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

    /// The first group in front after typing is the run, and stays the run
    /// until the shell has held the front.
    #[test]
    fn the_first_group_after_typing_is_the_run() {
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

        // Ended, with Ctrl+C: the shell is back in front, and once it has held
        // it the run is over.
        assert!(!run.read(SHELL, now), "not on one reading");
        assert!(run.starting(), "read quickly until it settles");
        assert!(run.read(SHELL, now + SETTLING));
        assert_eq!(run.in_front(), InFront::Shell);
        assert!(!run.starting());

        // And the human starts something of their own.
        assert!(run.read(THEIRS, now + SETTLING));
        assert_eq!(run.in_front(), InFront::Other);
    }

    /// Each step of a chain is a group of its own, and every one of them is
    /// the run — the shell in front for a moment between two of them is not
    /// the run having ended.
    #[test]
    fn every_step_of_a_chain_is_the_run() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);

        assert!(!run.read(RUN, now));
        assert!(!run.read(SHELL, now + MOMENT), "between the steps");
        assert!(!run.read(NEXT, now + MOMENT * 2), "the next step");
        assert!(
            !run.read(SHELL, now + SETTLING * 2),
            "the shell between steps again: the last one reset the clock"
        );
        assert!(!run.read(NEXT, now + SETTLING * 2 + MOMENT));
        assert_eq!(run.in_front(), InFront::Run);

        // Over once the shell holds the front, and then any step brought back
        // with `fg` is the run again, and another group is not.
        assert!(settled(&mut run, now + SETTLING * 3));
        assert_eq!(run.in_front(), InFront::Shell);
        assert!(run.read(THEIRS, now + SETTLING * 5));
        assert_eq!(run.in_front(), InFront::Other);
    }

    /// A run stopped and brought back is still the run.
    #[test]
    fn a_run_brought_back_to_the_front_is_the_run_again() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);
        run.read(RUN, now);
        settled(&mut run, now);

        assert!(run.read(RUN, now + SETTLING));
        assert_eq!(run.in_front(), InFront::Run);
    }

    /// Typed again with the shell in front, it is a new run: the old one's
    /// group, brought back once the new one has ended, is not it.
    #[test]
    fn typing_again_starts_a_new_run() {
        let now = Instant::now();
        let mut run = Run::new();
        run.typed(now);
        run.read(RUN, now);
        settled(&mut run, now);

        run.typed(now);
        assert_eq!(run.in_front(), InFront::Run);

        assert!(!run.read(Front::Group(44), now));
        assert_eq!(run.in_front(), InFront::Run);

        assert!(settled(&mut run, now));
        assert!(run.read(RUN, now + SETTLING));
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
