//! Where a details pane stands: the path it is reached at, and what a path says
//! is open.
//!
//! The arithmetic alone, which is `src/workbench/openings.ts` and knows nothing
//! of the page — what the workbench does with it is `workbench.test.tsx`, under
//! *the path a details pane stands at*. Worth its own file for the same reason
//! the widths have one: a path is a value, and a value is cheaper to hold true
//! here than through a mounted page.

import { describe, expect, it } from "vitest";

import {
  openingAt,
  opensRoadmap,
  pathOf,
  pathTo,
  roadmapOpened,
  type Opening,
} from "../src/workbench/openings";

/// Every kind of thing the pane opens, and the path each stands at.
const PATHS: Array<[Opening, string]> = [
  [7, "/conversations/3/events/7"],
  ["backlog", "/conversations/3/backlog"],
  [opensRoadmap("mvp"), "/conversations/3/roadmaps/mvp"],
  [opensRoadmap("companion-repos"), "/conversations/3/roadmaps/companion-repos"],
];

describe("where a details pane stands", () => {
  it("puts every kind of pane at a path under its conversation", () => {
    expect(PATHS.map(([opening]) => pathTo("3", opening))).toEqual(
      PATHS.map(([, path]) => path),
    );
  });

  /// The conversation's own path is what those are nested under, and what the
  /// sidebar navigates to. Its id is a number on the wire and a segment in the
  /// URL, so it is taken either way.
  it("nests them all under the conversation's own path", () => {
    expect(pathOf(3)).toBe("/conversations/3");
    expect(pathOf("3")).toBe("/conversations/3");
    PATHS.forEach(([, path]) => expect(path.startsWith(`${pathOf(3)}/`)).toBe(true));
  });

  /// Which is the whole point of the `events/` segment: an id can never be read
  /// as one of the words beside it, however the words grow.
  it("keeps the ids behind a segment of their own", () => {
    expect(pathTo("3", 7)).toContain("/events/");
    expect(openingAt("/conversations/3/backlog")).toBe("backlog");
    expect(openingAt("/conversations/3/events/7")).toBe(7);
  });
});

describe("what a path says is open", () => {
  it("reads back every path it writes", () => {
    expect(PATHS.map(([, path]) => openingAt(path))).toEqual(
      PATHS.map(([opening]) => opening),
    );
  });

  /// A roadmap is named by a directory, so the name goes into the path escaped
  /// and comes back out of it whole.
  it("carries a roadmap's name through the path unharmed", () => {
    const named = opensRoadmap("steer and stages");
    expect(pathTo("3", named)).toBe("/conversations/3/roadmaps/steer%20and%20stages");
    expect(roadmapOpened(openingAt(pathTo("3", named)))).toBe("steer and stages");
  });

  /// Nothing is open at the conversation's own path, on the bare workbench, or
  /// on any other page: the pane is bare paper at all three.
  it("says nothing is open where the path names no pane", () => {
    expect(openingAt("/conversations/3")).toBeNull();
    expect(openingAt("/")).toBeNull();
    expect(openingAt("/settings")).toBeNull();
    expect(openingAt("/sets/7")).toBeNull();
  });

  /// And a path that names a pane there is no pane for leaves it empty, which
  /// is what a stale selection does: the URL is a record of what was picked
  /// rather than a promise that it is still there.
  it("says nothing is open where the path names nonsense", () => {
    expect(openingAt("/conversations/3/events/nowhere")).toBeNull();
    expect(openingAt("/conversations/3/nowhere")).toBeNull();
    expect(openingAt("/conversations/3/events")).toBeNull();
    expect(openingAt("/conversations/3/backlog/nowhere")).toBeNull();
    expect(openingAt("/conversations/3/events/1e3")).toBeNull();
    expect(openingAt("/conversations/3/roadmaps/mvp/1")).toBeNull();
  });
});
