//! What the open Conversation can go to, as the page pushes it: the shape checked
//! rather than trusted, before the edge holds it for the Go menu.
//!
//! The edge's own half — dropping what this refuses and saying so — is
//! `main.ts`'s, a channel being nothing this suite can open; what it acts on is
//! this module's answer, which is what is asked about here.

import { describe, expect, it } from "vitest";

import { enabled, NOWHERE, PANES, pathFor, type Where, where } from "../src/where.js";

/// What the page says of a Conversation on a member with every pane to go to.
const AT = "/devices/8f2a/conversations/3";
const ALL: Where = {
  panes: [
    { pane: "timeline", path: `${AT}/timeline` },
    { pane: "code", path: `${AT}/code` },
    { pane: "backlog", path: `${AT}/backlog` },
    { pane: "roadmap", path: `${AT}/roadmaps/mvp` },
    { pane: "share", path: `${AT}/share` },
  ],
};

describe("a push of the right shape", () => {
  it("is held as it was said", () => {
    expect(where(ALL)).toEqual(ALL);
  });

  it("is held with nothing in it, which is a page with nothing open", () => {
    expect(where({ panes: [] })).toEqual(NOWHERE);
  });

  it("is held with some panes left out", () => {
    const some = { panes: [ALL.panes[0], ALL.panes[1], ALL.panes[4]] };

    expect(where(some)).toEqual(some);
  });

  /// Only what the shape names: a field nobody reads is not carried on to the
  /// menu.
  it("keeps nothing it was not asked for", () => {
    const extra = { panes: [{ pane: "code", path: "/conversations/3/code", label: "x" }], more: 1 };

    expect(where(extra)).toEqual({ panes: [{ pane: "code", path: "/conversations/3/code" }] });
  });
});

describe("a push of the wrong shape", () => {
  it.each([
    ["nothing", undefined],
    ["null", null],
    ["a string", "/conversations/3"],
    ["no list", {}],
    ["a list that is not one", { panes: "timeline" }],
    ["a pane that is not an object", { panes: ["timeline"] }],
    ["a pane the menu has not got", { panes: [{ pane: "steer", path: `${AT}/steer` }] }],
    ["a pane said twice", { panes: [ALL.panes[1], ALL.panes[1]] }],
    ["a pane with no path", { panes: [{ pane: "code" }] }],
    ["a path that is not the workbench's", { panes: [{ pane: "code", path: "code" }] }],
    ["a path to another host", { panes: [{ pane: "code", path: "//elsewhere.example/x" }] }],
    ["a path with a backslash", { panes: [{ pane: "code", path: "/\\elsewhere.example" }] }],
  ])("is dropped: %s", (_case, pushed) => {
    expect(where(pushed)).toBeUndefined();
  });
});

describe("the panes", () => {
  /// The Go menu's own order, which is the workbench's.
  it("are the five the Go menu draws, in its order", () => {
    expect(PANES).toEqual(["timeline", "code", "backlog", "roadmap", "share"]);
  });
});

describe("which of the Go menu's items can be pressed", () => {
  /// Where the window's history stands at the very start of a run.
  const START = { back: false, forward: false };

  it("is Conversations alone at the start of a run, with nowhere said", () => {
    expect(enabled(NOWHERE, START)).toStrictEqual({
      back: false,
      forward: false,
      conversations: true,
      timeline: false,
      code: false,
      backlog: false,
      roadmap: false,
      share: false,
    });
  });

  /// The settings page and the compose page say nowhere, and have a history
  /// behind them.
  it("greys the five panes on a page with no Conversation open", () => {
    expect(enabled(NOWHERE, { back: true, forward: false })).toStrictEqual({
      back: true,
      forward: false,
      conversations: true,
      timeline: false,
      code: false,
      backlog: false,
      roadmap: false,
      share: false,
    });
  });

  it("enables every pane on a record with a backlog and a roadmap", () => {
    expect(enabled(ALL, { back: true, forward: true })).toStrictEqual({
      back: true,
      forward: true,
      conversations: true,
      timeline: true,
      code: true,
      backlog: true,
      roadmap: true,
      share: true,
    });
  });

  it("greys Backlog and Roadmap on a record with neither", () => {
    const three = {
      panes: ALL.panes.filter(({ pane }) => pane !== "backlog" && pane !== "roadmap"),
    };

    expect(enabled(three, START)).toMatchObject({
      timeline: true,
      code: true,
      backlog: false,
      roadmap: false,
      share: true,
    });
  });
});

describe("where a pane item goes", () => {
  it("is the path the page gave for it, device and all", () => {
    expect(pathFor(ALL, "roadmap")).toBe(`${AT}/roadmaps/mvp`);
    expect(pathFor(ALL, "timeline")).toBe(`${AT}/timeline`);
  });

  it("is nowhere for a pane the page did not name", () => {
    expect(pathFor(NOWHERE, "code")).toBeUndefined();
  });
});
