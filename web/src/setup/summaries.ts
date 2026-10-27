//! What the rail says under a step that stands met: one line naming what
//! meeting it came to.
//!
//! Worked out here rather than sent, because both lines are a reading the page
//! already has said again in fewer words — the rows the onboarding reading
//! carries, and the Profiles the settings already list. A step 3 that is met is
//! the wizard over, so there is no line for it.

import { AGENT_NAME } from "../agents";
import type { DependencyView, ProfileEntry } from "../api/types";
import { NAMES } from "./Dependencies";

/// A list of names as a sentence reads one, with the serial comma `docs/copy.md`
/// asks for: *A sandbox, git, and Claude Code*.
export function listed(names: string[]): string {
  if (names.length <= 2) {
    return names.join(" and ");
  }

  return `${names.slice(0, -1).join(", ")}, and ${names.at(-1)}`;
}

/// The first step's line: the rows the machine has, in their row names and in
/// the order the step draws them.
export function present(rows: DependencyView[]): string {
  return listed(
    rows
      .filter((row) => row.state.state === "Present")
      .map((row) => NAMES[row.dependency]),
  );
}

/// The second's: the harnesses of the Profiles saved, each named once however
/// many accounts it has, in the order the app lists the harnesses in.
export function harnesses(profiles: ProfileEntry[]): string {
  const kept = new Set(profiles.map((profile) => profile.account.agent_type));

  return listed(
    (Object.keys(AGENT_NAME) as Array<keyof typeof AGENT_NAME>)
      .filter((agent) => kept.has(agent))
      .map((agent) => AGENT_NAME[agent]),
  );
}
