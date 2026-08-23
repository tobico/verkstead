//! The three ways the work can be built, as everything that names one names it.
//!
//! Its own module because two choosers draw the same three: the one injected
//! onto a Set carrying a proposal, which is where the human picks, and the
//! workbench's own beside the Timeline. A second copy of these words would be
//! the same decision offered under two sets of names.

import type { Direction } from "./api/types";

/// The three, in the order the design names them — smallest first, which is
/// also the order to consider them in.
export const DIRECTIONS: Direction[] = ["inline", "task-list", "roadmap"];

/// What each direction is called, wherever one is named: on the chooser, and in
/// the line the Timeline gives the choice afterwards.
///
/// One record for all of them, so the thing the human picked and the thing they
/// read back cannot come to be called different things.
export const DIRECTION: Record<Direction, string> = {
  inline: "Implement inline",
  "task-list": "Break into a task list",
  roadmap: "Stage a roadmap",
};

/// What each one does, in the line under its name.
///
/// What the direction *is* and nothing about what pressing it sets off: each
/// chooser says that once, in its own words, rather than three times over.
export const DIRECTION_NOTE: Record<Direction, string> = {
  inline:
    "One fresh session under the implementation profile, primed with the handoff.",
  "task-list":
    "Broken into .tasks/ in the worktree by a session of its own, then one fresh session per task.",
  roadmap:
    "Staged under docs/roadmaps/ by a session of its own, a feature per stage.",
};
