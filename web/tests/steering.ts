//! The pending steer a conversation carries while its form is being written,
//! for the suites that draw one.
//!
//! One place rather than a literal per site: the form is every field the pane
//! has, and a test that only cares *that* a steer is pending should not have to
//! spell all of them out to say so.

import type { PendingSteerView, SteerForm } from "../src/api/types";

/// The form a press opens on: no target picked, nothing written, nothing
/// ticked, and no companion row touched.
export const UNANSWERED: SteerForm = {
  target: null,
  brief: null,
  digest: false,
  instruction: null,
  follow_up: null,
  investigation: null,
  pairing: null,
  interrupt: false,
  added: [],
  upgraded: [],
};

/// A pending steer, with whatever the test is about written into its form.
export function pending(form: Partial<SteerForm> = {}): PendingSteerView {
  return { at: "2026-09-23T09:14:00Z", form: { ...UNANSWERED, ...form } };
}
