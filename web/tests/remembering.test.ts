//! Where Code's storage is kept, per Conversation *and* per device (ADR-0025).
//!
//! Every Verkstead issues a Conversation 7, so a key of the number alone was one
//! layout and one unsaved text for a member's Conversation 7 and this device's.
//! The keys follow the URL's own shape instead: this device's own bare, as they
//! always were, and a member's under its Device Id.

import { createRoot } from "solid-js";
import { describe, expect, it } from "vitest";

import { rowKey } from "../src/reaching";
import { keeping } from "../src/workbench/keeping";
import { holding } from "../src/workbench/remembering";

/// A member of the cluster, by the id its rows carry.
const MEMBER = "cc11dd22ee33ff4400112233aabbccdd";

/// A layout of one group holding one file, written as the keeping writes one.
function layoutOf(file: string): string {
  return JSON.stringify({
    layout: { group: { id: 1, tabs: [{ file }], chosen: file } },
    active: 1,
  });
}

/// The files the first group of a keeping is holding.
function filesIn(conversation: number, device: string | null): string[] {
  return createRoot((dispose) => {
    const kept = keeping().of(conversation, device);
    const layout = kept.layout();
    dispose();

    return "group" in layout
      ? layout.group.tabs().flatMap((tab) => ("file" in tab ? [tab.file] : []))
      : [];
  });
}

describe("code's storage", () => {
  /// The bare keys already written are this device's own, and lose nothing.
  it("reads this device's own conversation off the bare key", () => {
    localStorage.setItem("verkstead.code.7", layoutOf("/ours/Cargo.toml"));

    expect(filesIn(7, null)).toEqual(["/ours/Cargo.toml"]);
  });

  /// And a member's same-numbered Conversation is another piece of work: it
  /// reads its own key, and starts afresh where it has none.
  it("keeps a member's same-numbered conversation apart", () => {
    localStorage.setItem("verkstead.code.7", layoutOf("/ours/Cargo.toml"));

    expect(filesIn(7, MEMBER)).toEqual([]);

    localStorage.setItem(
      `verkstead.code.${MEMBER}.7`,
      layoutOf("/theirs/README.md"),
    );

    expect(filesIn(7, MEMBER)).toEqual(["/theirs/README.md"]);
    expect(filesIn(7, null)).toEqual(["/ours/Cargo.toml"]);
  });

  /// What the sidebar reads before any Code pane is opened: which rows the
  /// storage holds unsaved text for, under either shape of key.
  it("says which rows hold unsaved text, by device and number", () => {
    localStorage.setItem("verkstead.code-unsaved.3", "{}");
    localStorage.setItem(`verkstead.code-unsaved.${MEMBER}.7`, "{}");
    localStorage.setItem("verkstead.code.5", layoutOf("/ours/lib.rs"));
    localStorage.setItem("verkstead.code-unsaved.not-a-number", "{}");

    expect([...holding()].sort()).toEqual(
      [rowKey(null, 3), rowKey(MEMBER, 7)].sort(),
    );
  });
});
