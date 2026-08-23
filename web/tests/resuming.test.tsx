//! Coming back to the app after it was away — the PWA reopened, the phone
//! unlocked, the tab refocused. The document becoming visible again is the one
//! signal that reliably fires on an iOS PWA resume, and every open page reads
//! itself afresh off it.
//!
//! Driven through `App` rather than through a page on its own, because what is
//! being asked about is the app's own query client: the refetch is that
//! client's doing, and a test that built a client of its own would only be
//! asking about the client it built. Both pages are reached on their real
//! routes, for the same reason.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type {
  ConversationEntry,
  ConversationView,
  ProfileEntry,
  QuestionSetEvent,
  RepoEntry,
  SetView,
  TimelineEvent,
} from "../src/api/types";
import { askedFor, json, serving, whenever } from "./serving";
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

/// The renderer is a page's own doing and neither Set fixture has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

/// The Conversation the human left open, with a session's Question Sets on its
/// Timeline — which is where a Set is now reached.
const CONVERSATION = grilling as ConversationView;

/// The read coming back is meant to cause, and the only one worth counting.
const OPENED = `/api/ui/conversations/${CONVERSATION.id}`;

/// What the app fetches alongside the Conversation it is about — the sidebar,
/// the Repos the picker on it needs, the Agent Profiles the details pane picks
/// from, and the release check. Held to their own paths rather than left in the
/// order the answers are handed out: a page with several things to fetch has no
/// fixed order between them, and which test pays for one of these is not
/// something any of them is about.
const BESIDE = [
  whenever("/api/ui/conversations", json(conversations as ConversationEntry[])),
  whenever("/api/ui/repos", json(repos as RepoEntry[])),
  whenever("/api/ui/profiles", json(profiles as ProfileEntry[])),
  whenever("/api/ui/update", json("Current")),
];

/// One Set twice over: waiting when the app went away, answered from another
/// device by the time it came back.
const WAITING = answering as SetView;
const ANSWERED = answered as SetView;

/// The Set on the fixture's Timeline that is still waiting, to build an arrival
/// out of: an Event of the shape the server really writes, rather than one made
/// up here.
///
/// The waiting one rather than the answered one because its title is the
/// Timeline's alone — the answered Set is titled after the Brief above it, and a
/// title drawn twice on one page is not one a test can look for.
const ASKED: QuestionSetEvent = (() => {
  const found = CONVERSATION.timeline.find(
    (event): event is { QuestionSet: QuestionSetEvent } =>
      "QuestionSet" in event && "Waiting" in event.QuestionSet.standing,
  );
  if (!found) {
    throw new Error("the fixture's Timeline should carry a waiting Question Set");
  }
  return found.QuestionSet;
})();

/// What arrived while nobody was looking — the Set the human must not have to
/// wait ten seconds to be told about.
const ARRIVAL: TimelineEvent = {
  QuestionSet: {
    ...ASKED,
    id: ASKED.id + 100,
    set_id: ASKED.set_id + 100,
    title: "Whether to keep the outbound queue at all",
    standing: { Waiting: "waiting" },
  },
};

/// The same Conversation a moment later, with the arrival at the foot of its
/// Timeline.
const MOVED_ON: ConversationView = {
  ...CONVERSATION,
  timeline: [...CONVERSATION.timeline, ARRIVAL],
};

/// What the Timeline calls the Set that was already there when the page was
/// drawn.
const ALREADY_THERE = ASKED.title;

beforeEach(() => {
  // The poll is the fallback these tests are here to get ahead of, so the clock
  // is held still: anything a page learns here, it learned from coming back.
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  // The state is the instance's own, over the one every other test reads off
  // the prototype — so it goes when the test does.
  delete (document as { visibilityState?: DocumentVisibilityState })
    .visibilityState;
});

/// The app going where the human put it, as the browser says so: the state
/// first, because a listener reads it rather than the event.
///
/// Dispatched on the document and left to bubble, which is where the browser
/// fires it and how it reaches a listener on the window.
function showing(state: DocumentVisibilityState): void {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => state,
  });
  document.dispatchEvent(new Event("visibilitychange", { bubbles: true }));
}

/// Away and back again — the whole of a resume, which is two transitions and
/// not one.
function reopened(): void {
  showing("hidden");
  showing("visible");
}

describe("coming back to the app", () => {
  it("shows the Set that arrived while it was away", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(
      ...BESIDE,
      json(CONVERSATION),
      json(MOVED_ON),
    );
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));

    reopened();

    await waitFor(() => screen.getByText(ARRIVAL.QuestionSet.title));
    // The clock never moved, and nothing here runs on one: the second read is
    // the one coming back asked for.
    expect(askedFor(fetching, OPENED)).toBe(2);
  });

  it("catches up the Set whose page was open when it went away", async () => {
    window.history.pushState({}, "", `/sets/${WAITING.id}`);
    serving(...BESIDE, json(WAITING), json(ANSWERED));
    const { container } = render(() => <App />);
    // The badge and the menu under it belong to a Set still waiting: this is
    // the page as it was left.
    await waitFor(() => expect(container.querySelector(".standing")).toBeTruthy());

    reopened();

    // Answered from another device in the meantime, so the page the human comes
    // back to is the record rather than the form they left.
    await waitFor(() =>
      expect(container.querySelector(".answered-at")).toBeTruthy(),
    );
  });

  it("asks for nothing while the app is away", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));

    showing("hidden");
    await vi.advanceTimersByTimeAsync(0);

    // Going away is not coming back: the phone pays for a fetch, and there is
    // nobody there to read what it brings.
    expect(askedFor(fetching, OPENED)).toBe(1);
  });
});
