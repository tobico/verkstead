//! What the viewer says about a Windows machine's Smart App Control, said once
//! for the two places that say it.
//!
//! **Two surfaces, one condition, and the human sees both.** The onboarding
//! wizard's machine step says it while somebody is standing at the box; the
//! settings page's Sandbox section says it long after, which is where a machine
//! that has started enforcing since onboarding is read. They were two constants
//! of the same name in two files, word for word the same but joined
//! differently, so a correction to the sentence would have reached one of them
//! and neither the other nor the test pinning it.
//!
//! **Worded apart because the two states are different news.** Enforcing is a
//! session that cannot run what it builds here, today. Evaluation blocks
//! nothing today and is Windows deciding for itself whether to start, which is
//! a machine that can cross into the first state with nobody told — so the line
//! says that rather than saying everything is fine.
//!
//! **Two halves rather than one sentence**, because the wizard draws a callout
//! with a line over a line: what the state is, and what it means for a session.
//! Whoever draws them runs them together where one line is what they have room
//! for.
//!
//! The wording is the viewer's, like `BARRED` and `SEEN` in the wizard's step:
//! what the server sends is which state the machine is in — see `AppControl` in
//! [`./api/types.ts`](./api/types.ts) — and what to say about it is the same two
//! pairs of sentences on every Verkstead. Verkstead reads this setting and never
//! writes it; turning it off is the human's one-way decision in Windows
//! Security.

import type { AppControl } from "./api/types";

/// What the state is, and what it means for a session.
export type AppControlSaid = {
  /// Which state the machine is in, as one sentence naming the setting.
  said: string;
  /// And what that means for a session here, with what Verkstead does and does
  /// not do about it.
  means: string;
};

/// The two states, each said and then explained.
export const APP_CONTROL: Record<AppControl, AppControlSaid> = {
  Enforcing: {
    said: "Smart App Control is enforcing on this machine.",
    means:
      "A session can't run an executable it builds here. Verkstead never " +
      "changes this setting: turning it off is in Windows Security, and " +
      "Windows can't turn it back on without a reinstall.",
  },
  Evaluation: {
    said: "Smart App Control is evaluating on this machine.",
    means:
      "It blocks nothing today. Windows decides on its own whether to start " +
      "enforcing it, and a session then can't run what it builds here. " +
      "Verkstead never changes this setting.",
  },
};
