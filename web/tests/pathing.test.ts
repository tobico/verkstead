//! Where a details pane stands: the path it is reached at, and what a path says
//! is open.
//!
//! And which Events have a pane behind them at all, which is the same question
//! asked of an Event rather than of a path — so the end of a record is here too:
//! the last thing on a Timeline that can be opened, which is where opening a
//! Conversation lands.
//!
//! The arithmetic alone, which is `src/workbench/openings.ts` and knows nothing
//! of the page — what the workbench does with it is `workbench.test.tsx`, under
//! *the path a details pane stands at* and *landing on the end of the record*.
//! Worth its own file for the same reason the widths have one: a path is a
//! value, and a value is cheaper to hold true here than through a mounted page.

import { describe, expect, it } from "vitest";

import type { ConversationView } from "../src/api/types";
import {
  lastOpening,
  openingAt,
  openingOf,
  opensRoadmap,
  pathOf,
  pathTo,
  roadmapOpened,
  type Opening,
} from "../src/workbench/openings";
import draft from "./fixtures/conversation.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import roadmapped from "./fixtures/conversation-roadmap.json" with {
  type: "json",
};
import secondRound from "./fixtures/conversation-second-round.json" with {
  type: "json",
};
import tasked from "./fixtures/conversation-tasks.json" with { type: "json" };
import wrapping from "./fixtures/conversation-wrapping.json" with {
  type: "json",
};

/// The records these are read off, which are the golden fixtures `cargo test`
/// writes from the real endpoints — so what is held true here is what the server
/// actually said a Timeline is.
const DRAFT = draft as ConversationView;
const GRILLING = grilling as ConversationView;
const ROADMAPPED = roadmapped as ConversationView;
const SECOND_ROUND = secondRound as ConversationView;
const TASKED = tasked as ConversationView;
const WRAPPING = wrapping as ConversationView;

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

describe("the end of a record", () => {
  /// Which is what opening a Conversation lands on: the last thing that has a
  /// pane behind it, so the human arrives at where the work got to.
  it("is the last event with a pane behind it", () => {
    const last = GRILLING.timeline.at(-1)!;
    expect("UnreadableSet" in last).toBe(true);
    expect(lastOpening(GRILLING.timeline)).toBe(openingOf(last));
  });

  /// And it is the last *openable* one rather than the last one. A record very
  /// often ends on a move — every step of the ladder writes one — and a wrap-up
  /// ends on a manual task, neither of which has anything to show.
  it("skips past the events with nothing to open", () => {
    expect(WRAPPING.timeline.slice(-2).map(openingOf)).toEqual([null, null]);

    const opened = WRAPPING.timeline.find((event) => "PullRequest" in event)!;
    expect(lastOpening(WRAPPING.timeline)).toBe(openingOf(opened));
  });

  /// A steer that carried no document is a third: it says the state and nothing
  /// else, which is why the Timeline draws one as a line rather than a card.
  it("skips a steer that carried no document", () => {
    const steer = SECOND_ROUND.timeline.find((event) => "Steer" in event)!;
    expect("Steer" in steer && steer.Steer.html).toBeNull();
    expect(openingOf(steer)).toBeNull();
  });

  /// And the Brief while it is still being written, which is a field with the
  /// conversation's setup under it rather than a card to press. A Draft that has
  /// nothing else on its record has nothing openable at all, and the pane stays
  /// bare paper.
  it("says nothing of a brief that has not frozen", () => {
    const brief = DRAFT.timeline[0]!;
    expect("Brief" in brief && brief.Brief.frozen).toBe(false);
    expect(openingOf(brief)).toBeNull();
    expect(lastOpening(DRAFT.timeline)).toBeNull();
  });

  /// The two lists open by their word rather than by the id of the row they
  /// landed at, being read off the worktree rather than off the record.
  it("opens the two lists by the word their cards are named by", () => {
    expect(lastOpening(TASKED.timeline)).toBe("backlog");

    const stages = ROADMAPPED.timeline.find((event) => "StageList" in event)!;
    expect(openingOf(stages)).toBe(opensRoadmap("mvp"));
  });

  /// And a backlog whose worktree has gone leaves the row with nothing to read,
  /// so there is nothing to open either — which is the same answer the Timeline
  /// gives by drawing no card at that row.
  it("says nothing of a list there is nothing left to read", () => {
    const landed = TASKED.timeline.find((event) => "TaskList" in event);
    if (landed === undefined || !("TaskList" in landed)) {
      throw new Error("the fixture should carry a backlog on its record");
    }

    expect(openingOf(landed)).toBe("backlog");
    expect(openingOf({ TaskList: { ...landed.TaskList, list: null } })).toBeNull();
  });

  /// Nothing openable anywhere on a record selects nothing, whatever is on it.
  it("says nothing of a record made only of moves", () => {
    expect(
      lastOpening(WRAPPING.timeline.filter((event) => "Moved" in event)),
    ).toBeNull();
    expect(lastOpening([])).toBeNull();
  });
});
