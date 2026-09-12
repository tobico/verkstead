//! The wizard's three steps: what they are called, whether one stands met, and
//! which of them this device has open.
//!
//! The arithmetic of the frame and nothing of what a step contains — the same
//! separation the settings page keeps between `openings.ts` and the panes it
//! draws. Three things read this and all three have to agree: the page draws
//! the steps from [`STEPS`], reads their met-ness off the server's own
//! [`StepsView`], and remembers which is open under [`SETUP_STEP`].
//!
//! **The open step is a fact about the tab in front of you**, so it is kept on
//! the device (`src/device.ts`) and never sent anywhere. Two people setting the
//! same machine up from two browsers are on two different steps, and neither
//! has any business moving the other along; a phone put down mid-install picks
//! up where it was left. It is also why there is nothing on the wire for it:
//! the server's model of onboarding is the objective and the mode, and where
//! somebody has got to in reading it is not part of either.
//!
//! **And the first step's three screens are kept the same way** — what to
//! install, the run installing it, and what is left to do by hand. Which of
//! them is open is a fact about this device exactly as the open step is, so it
//! sits beside it under [`SETUP_SCREEN`] rather than on the wire.
//!
//! The order is the order a machine is set up in, and it is not a choice the
//! human has: a Profile is an account under a harness that has to be installed
//! first, and there is nothing to commit as until there is something to commit.

import { read, write } from "../device";
import type { StepsView } from "../api/types";

/// Where the wizard stands. Its own page rather than a pane of the settings —
/// see ADR-0016 — and the only page there is while onboarding mode is on.
export const SETUP = "/setup";

/// And where the wizard lets go: the compose page, which is where a Verkstead
/// with nothing in it yet has something to do. ADR-0016's own landing — stage
/// 04 makes that page's zero state, and until it lands the compose page is the
/// one that exists.
export const COMPOSE = "/compose";

/// The three steps, in the order they are worked through.
///
/// Named for the fields of [`StepsView`], which is what says whether one is met:
/// two lists that had to be kept in step would be two places for a step to go
/// missing from.
export const STEPS = ["dependencies", "accounts", "git"] as const;

/// One of them.
export type Step = (typeof STEPS)[number];

/// What each is called where the human reads it.
export const TITLES: Record<Step, string> = {
  dependencies: "What a session needs",
  accounts: "Agent Profiles",
  git: "Who the work is committed as",
};

/// Whether a step stands met, as the server last said.
///
/// A read that has not landed yet is *unmet*, which is what keeps the interval
/// running while the page is still working out where it stands: a step nobody
/// has heard about is not one to stop probing over.
export function met(steps: StepsView | undefined, step: Step): boolean {
  return steps?.[step] === true;
}

/// And the step after this one, or `null` at the end of the wizard.
export function after(step: Step): Step | null {
  return STEPS[STEPS.indexOf(step) + 1] ?? null;
}

/// Where this device's open step is kept. Namespaced like everything else this
/// app leaves in a browser.
export const SETUP_STEP = "verkstead.setup-step";

/// Which step this device has open — the first one where it has never had any,
/// which is where a wizard nobody has opened before starts.
///
/// Anything else under the key is a step this build does not have, from a
/// browser that was left on an older one: read as the first, rather than as a
/// page with nothing open on it.
export function openStep(): Step {
  const held = read(SETUP_STEP);
  return STEPS.find((step) => step === held) ?? STEPS[0];
}

/// Remember which step is open, so that reopening the wizard opens it.
export function keepStep(step: Step): void {
  write(SETUP_STEP, step);
}

/// How often the page asks the machine again while something is still to land,
/// in milliseconds.
///
/// ADR-0016's own number. The probes are a `PATH` walked and one trivial
/// `bwrap`, so this costs the machine nothing worth counting — and what it buys
/// is that `apt install bubblewrap` finishing in another window is a tick here
/// without anybody touching the page.
export const PROBE = 10_000;

/// And how often it asks while an install run of Verkstead's own is going.
///
/// Faster, because what is being watched is no longer somebody else's terminal:
/// the bar and the status line on the install screen are drawn from the
/// reading, so the interval is how often that screen moves at all. Two seconds
/// is short enough to read as a run going on rather than as a page updating.
export const RUNNING = 2_000;

/// The dependencies step's three screens, in the order a press moves through
/// them: what to install, the run installing it, and what is left to do by
/// hand.
export const SCREENS = ["choosing", "installing", "hints"] as const;

/// One of them.
export type Screen = (typeof SCREENS)[number];

/// Where this device's open screen is kept, beside the open step and for the
/// same reason: which screen somebody is on is a fact about the tab in front of
/// them, and the server's model of onboarding is the objective and the mode.
export const SETUP_SCREEN = "verkstead.setup-screen";

/// Which screen this device was left on — the first of the three where it has
/// never had one, and where what is under the key is not a screen this build
/// has.
export function openScreen(): Screen {
  const held = read(SETUP_SCREEN);
  return SCREENS.find((screen) => screen === held) ?? SCREENS[0];
}

/// Remember which screen is open, so that a reload opens it again.
export function keepScreen(screen: Screen): void {
  write(SETUP_SCREEN, screen);
}
