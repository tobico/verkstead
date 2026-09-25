//! The tray's menu as a value: what is on it, in what order, and what picking
//! View Logs comes to.
//!
//! Which is the whole of what a machine with no screen can check about a tray
//! — the icon itself is a thing on somebody's panel, and task 06 of this stage
//! is what puts one on a real desktop. The same split
//! `crates/desktop/src/tray.rs` made of its own menu, and the same three
//! questions asked of it.

import { afterEach, describe, expect, it, vi, type MockInstance } from "vitest";

import { type Chosen, label, MENU, viewing } from "../src/chosen.js";
import { keep, type Kept } from "../src/log.js";

/// Everything a spy on the stream was handed, which is where a run with no log
/// file puts its lines — including the one saying it has none.
const said = (stderr: MockInstance<typeof process.stderr.write>): string =>
  stderr.mock.calls.map(([line]) => String(line)).join("");

afterEach(() => vi.restoreAllMocks());

describe("the menu", () => {
  it("is Open, View Logs and then Quit", () => {
    expect(MENU.map(label)).toEqual(["Open", "View Logs", "Quit"]);
  });

  /// Because a panel that reports no click of its own opens the menu instead,
  /// and the first item is what a menu means by default. Asked on its own as
  /// well as in the order above: this one is a decision rather than a listing.
  it("has Open first, which is what the icon means by default", () => {
    expect(MENU[0]).toBe("open");
  });

  /// The id an item is made with is the id a pick is read back by, so two items
  /// sharing one would be two items that cannot be told apart.
  it("names each item once", () => {
    expect(new Set<Chosen>(MENU).size).toBe(MENU.length);
  });

  /// The Rust app's menu had Launch on Startup between the log and the way out;
  /// here that is a checkbox on the Desktop page (ADR-0020), and the tray is
  /// the three things a page cannot do.
  it("leaves Launch on Startup to the page", () => {
    expect(MENU.map(label)).not.toContain("Launch on Startup");
  });
});

describe("View Logs", () => {
  it("opens the file this run's logging went to", () => {
    const kept: Kept = { file: "/var/log/verkstead/verkstead.log" };

    expect(viewing(kept)).toEqual({ open: "/var/log/verkstead/verkstead.log" });
  });

  /// A machine with nowhere to keep a log file has only lost the log, so the
  /// item says where the logging went instead of opening nothing.
  it("says why there is none where there is none", () => {
    const kept: Kept = { nowhere: "Verkstead has nowhere to keep a log file on this machine" };

    expect(viewing(kept)).toEqual({
      note: "Verkstead has nowhere to keep a log file on this machine",
    });
  });

  /// And the words it says are the ones the logging itself worded, rather than
  /// anything this end made up: a run that was given nowhere to write hands its
  /// own reason over, and that is what reaches the human.
  it("says the words the logging handed over", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);

    const act = viewing(keep(undefined));

    expect(act).toHaveProperty("note");
    expect("note" in act ? act.note : "").toContain("standard error");
    expect(said(stderr)).toContain("nowhere to keep a log file");
  });
});
