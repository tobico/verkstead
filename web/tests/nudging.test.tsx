//! The Nudge: the open page being told what moved, and looking again because of
//! it — over the server's stream, and relayed by the service worker off a push
//! (ADR-0009).
//!
//! What a page does about a Nudge is the same whatever it says, and that is the
//! point of the tests below rather than an omission in them: reading a kind
//! narrowly is the next thing to be written, and until it is, every kind takes
//! the path an unrecognised kind will always take.
//!
//! Driven through `App` for the same reason `resuming` is — what the Nudge acts
//! on is the app's own query client, and a test that built a client of its own
//! would be asserting its own arrangement rather than the app's.
//!
//! The clock is held still throughout, except where the poll is the thing being
//! asked about: anything a page learns here it learned from a Nudge, because the
//! fallback underneath never ran.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type {
  ConversationEntry,
  ConversationView,
  Nudge,
  ProfileEntry,
  QuestionSetEvent,
  RepoEntry,
  SetView,
  TimelineEvent,
} from "../src/api/types";
import { askedFor, json, serving, whenever } from "./serving";
import { worker } from "./worker";
import kinds from "./fixtures/nudges.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

/// The renderer is a page's own doing and neither Set fixture has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

/// The Conversation the human is looking at, with a session's Question Sets on
/// its Timeline — which is where a Set arrives now that there is no list of
/// them.
const CONVERSATION = grilling as ConversationView;

/// The read a Nudge is meant to cause, and the only one worth counting.
const OPENED = `/api/ui/conversations/${CONVERSATION.id}`;

/// What the app fetches alongside the Conversation it is about — the sidebar,
/// the Repos the picker on it needs, the Agent Profiles the details pane picks
/// from, the roadmaps nothing is driving, and the release check. Held to their own paths and out of the counting:
/// a Nudge is never about a release, and which test pays for the rest is not
/// something any of them is about.
const BESIDE = [
  whenever("/api/ui/conversations", json(conversations as ConversationEntry[])),
  whenever("/api/ui/repos", json(repos as RepoEntry[])),
  whenever("/api/ui/profiles", json(profiles as ProfileEntry[])),
  whenever("/api/ui/abandoned-roadmaps", json([])),
  whenever("/api/ui/update", json("Current")),
];

/// One Set twice over: waiting when the page was drawn, answered from another
/// device by the time a Nudge says to look again.
const WAITING = answering as SetView;
const ANSWERED = answered as SetView;

/// The Set on the fixture's Timeline that is still waiting, to build an arrival
/// out of: an Event of the shape the server really writes.
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

/// What the Timeline already carried when the page was drawn.
const ALREADY_THERE = ASKED.title;

/// The Set the stream is there to be immediate about — put to the human while
/// they are looking straight at the Timeline it lands on.
const ARRIVAL: QuestionSetEvent = {
  ...ASKED,
  id: ASKED.id + 100,
  set_id: ASKED.set_id + 100,
  title: "Whether to keep the outbound queue at all",
  standing: { Waiting: "waiting" },
};

/// The same Conversation a moment later, with the arrival at the foot of its
/// Timeline.
const MOVED_ON: ConversationView = {
  ...CONVERSATION,
  timeline: [...CONVERSATION.timeline, { QuestionSet: ARRIVAL } as TimelineEvent],
};

/// Every kind of Nudge the server writes, off the fixture the server's own
/// tests leave behind — so what these drive the stream with is what really goes
/// down it rather than a guess at it.
const KINDS = kinds as Nudge[];

/// What a Nudge says by default here: a Question Set moved in a Conversation,
/// which is what nearly every test in this file makes happen.
const SET_ARRIVED: Nudge = {
  kind: "set",
  conversation: CONVERSATION.id,
};

/// A stand-in for the browser's `EventSource`, which jsdom has none of — and
/// which a test would want its own of anyway, having no other way to put a
/// Nudge on the wire or to sever the connection carrying it.
class Streaming {
  /// Every stream the app has opened, newest last.
  static opened: Streaming[] = [];

  private readonly listeners = new Map<string, Array<(event: Event) => void>>();
  closed = false;

  constructor(readonly url: string) {
    Streaming.opened.push(this);
  }

  addEventListener(name: string, listener: (event: Event) => void): void {
    this.listeners.set(name, [...(this.listeners.get(name) ?? []), listener]);
  }

  close(): void {
    this.closed = true;
  }

  /// What the browser does when the connection is established — on the first
  /// one and on every reconnect after it, which is the whole of how a page
  /// finds out it was away.
  opens(): void {
    this.fire("open");
  }

  /// One Nudge, as the server writes it: a named event whose data says what
  /// moved. `said` is passed through untouched, so a test may put something
  /// down the wire that no page could read.
  nudges(said: unknown = SET_ARRIVED): void {
    const data = typeof said === "string" ? said : JSON.stringify(said);

    for (const listener of this.listeners.get("nudge") ?? []) {
      listener(new MessageEvent("nudge", { data }));
    }
  }

  private fire(name: string): void {
    for (const listener of this.listeners.get(name) ?? []) {
      listener(new Event(name));
    }
  }
}

/// The stream the app opened, which there is always exactly one of: the app
/// opens it on mount and holds it for as long as it is running.
function stream(): Streaming {
  const opened = Streaming.opened.at(-1);
  if (!opened) {
    throw new Error("the app opened no stream");
  }
  return opened;
}

/// A stand-in for `navigator.serviceWorker`, the page's end of the relay, which
/// jsdom has none of either.
class Container {
  private readonly listeners = new Set<(event: MessageEvent) => void>();

  addEventListener(
    _name: "message",
    listener: (event: MessageEvent) => void,
  ): void {
    this.listeners.add(listener);
  }

  removeEventListener(
    _name: "message",
    listener: (event: MessageEvent) => void,
  ): void {
    this.listeners.delete(listener);
  }

  /// Whether anything on the page is still listening for a relayed Nudge.
  get listening(): boolean {
    return this.listeners.size > 0;
  }

  /// One message from the worker, as the browser hands it over.
  delivers(data: unknown): void {
    for (const listener of [...this.listeners]) {
      listener(new MessageEvent("message", { data }));
    }
  }
}

/// A browser whose worker can reach this page. Defined on the real navigator
/// rather than stubbed over it, because everything else the app reads there is
/// still wanted — and taken away again after the test.
function attaches(): Container {
  const container = new Container();
  Object.defineProperty(navigator, "serviceWorker", {
    configurable: true,
    value: container,
  });
  return container;
}

/// A push arriving at the service worker and relayed on, as the worker relays
/// it: the message delivered is the worker's own — `assets/sw.js` driven with a
/// real push — so what the page is asked about is what it will really be sent
/// rather than a guess at it.
async function pushed(container: Container): Promise<void> {
  const sw = worker();
  sw.opens();
  await sw.pushes({
    id: ARRIVAL.set_id,
    title: ARRIVAL.title,
    project: "verkstead",
  });

  for (const message of sw.relayed) {
    container.delivers(message);
  }
}

beforeEach(() => {
  vi.useFakeTimers();
  Streaming.opened = [];
  vi.stubGlobal("EventSource", Streaming);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  // The property is this test's own, over a navigator every other test shares.
  delete (navigator as { serviceWorker?: unknown }).serviceWorker;
});

describe("the Nudge stream", () => {
  it("listens on the server's stream for as long as the app is running", () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    serving(...BESIDE, json(CONVERSATION));
    const { unmount } = render(() => <App />);

    expect(stream().url).toBe("/api/ui/nudges");

    unmount();

    // Nothing is left holding a connection open behind a page that is gone.
    expect(stream().closed).toBe(true);
  });

  it("shows the Set a Nudge is about without waiting on the poll", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    stream().nudges();

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // The clock never moved, so the ten-second poll never ran: the second read
    // is the Nudge's doing and nothing else's.
    expect(askedFor(fetching, OPENED)).toBe(2);
  });

  it("reads everything back when a dropped stream reconnects", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    // Answered from another device while the stream was dead — so what the page
    // has to catch up on is a Set leaving the list, which no Nudge arrived to
    // say. The reconnect is the whole of the news.
    serving(...BESIDE, json(MOVED_ON), json(CONVERSATION));
    render(() => <App />);
    stream().opens();
    await waitFor(() => screen.getByText(ARRIVAL.title));

    stream().opens();

    await waitFor(() => expect(screen.queryByText(ARRIVAL.title)).toBeNull());
  });

  it("asks for nothing when the stream first opens", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));

    stream().opens();
    await vi.advanceTimersByTimeAsync(0);

    // The page has just read the world; opening the stream it reads the world
    // over is not news that the world moved.
    expect(askedFor(fetching, OPENED)).toBe(1);
  });

  it.each(KINDS.map((moved) => [moved.kind, moved] as const))(
    "reads everything back for a Nudge of kind %s",
    async (_kind, moved) => {
      window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
      const fetching = serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
      render(() => <App />);
      await waitFor(() => screen.getByText(ALREADY_THERE));
      stream().opens();

      stream().nudges(moved);

      // Every kind, whatever it names and wherever it points: what a page does
      // about one is read back everything it is showing (ADR-0009). Reading
      // back less is what the kinds are for and is not written yet, and this is
      // the test that will say so when it is.
      await waitFor(() => screen.getByText(ARRIVAL.title));
      expect(askedFor(fetching, OPENED)).toBe(2);
    },
  );

  it.each([
    ["a kind this page has never heard of", { kind: "liveness", conversation: 1 }],
    ["a Nudge that is not JSON at all", "nudge"],
    ["a Nudge with nothing in it", ""],
  ])("reads everything back for %s", async (_what, said) => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    stream().nudges(said);

    // A page against a server newer than itself, which is every page between a
    // deploy and a reload: what it cannot read it treats as everything having
    // moved, which is exactly the reaction it gives what it can.
    await waitFor(() => screen.getByText(ARRIVAL.title));
    expect(askedFor(fetching, OPENED)).toBe(2);
  });

  it("leaves the poll running underneath it", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    await vi.advanceTimersByTimeAsync(10_000);

    // The stream is the fast path, never the only one: a page that cannot have
    // one at all still keeps up, ten seconds at a time.
    expect(askedFor(fetching, OPENED)).toBe(2);
  });
});

describe("a Nudge relayed by the worker", () => {
  it("shows the Set the push was about, with no stream to hear it on", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    // The Conversation as the server would answer for it now, which the arrival
    // changes under the open page exactly as it does in the world.
    let standing = CONVERSATION;
    const fetching = serving(...BESIDE, () => json(standing)());
    const container = attaches();
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    const read = askedFor(fetching, OPENED);

    standing = MOVED_ON;
    await pushed(container);

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // One read more than the page had already done, and not one beyond it: the
    // stream never opened — which is what a suspended PWA leaves behind — and
    // the clock never moved, so the poll never ran either. The push is the whole
    // of how this page found out.
    expect(askedFor(fetching, OPENED)).toBe(read + 1);
  });

  it("reads everything back, exactly as a Nudge on the stream does", async () => {
    window.history.pushState({}, "", `/sets/${WAITING.id}`);
    serving(...BESIDE, json(WAITING), json(ANSWERED));
    const container = attaches();
    const { container: page } = render(() => <App />);
    // The badge and the menu under it belong to a Set still waiting: the page
    // as it was drawn.
    await waitFor(() => expect(page.querySelector(".standing")).toBeTruthy());

    await pushed(container);

    // A relayed Nudge says nothing at all where a streamed one says a kind, and
    // gets the reaction a kind nobody recognises gets: everything this page is
    // showing, which here is a Set answered elsewhere in the meantime.
    await waitFor(() => expect(page.querySelector(".answered-at")).toBeTruthy());
  });

  it("ignores a message that is not a Nudge", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION));
    const container = attaches();
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    const read = askedFor(fetching, OPENED);

    container.delivers({ verkstead: "something else" });
    container.delivers("a message from somewhere else entirely");
    container.delivers(null);
    await vi.advanceTimersByTimeAsync(0);

    // Whatever else may one day be posted to a page, by this worker or another,
    // is not a Nudge until it says so.
    expect(askedFor(fetching, OPENED)).toBe(read);
  });

  it("stops listening when the app goes", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    serving(...BESIDE, json(CONVERSATION));
    const container = attaches();
    const { unmount } = render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    expect(container.listening).toBe(true);

    unmount();

    // Nothing is left holding a query client that has no page to refresh.
    expect(container.listening).toBe(false);
  });

  it("shrugs where the browser has no worker at all", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    serving(...BESIDE, json(CONVERSATION));

    // No `attaches()`: jsdom has no `navigator.serviceWorker`, which is the same
    // absence a browser without service workers presents. The page loses the
    // relay and nothing else.
    expect(() => render(() => <App />)).not.toThrow();
    await waitFor(() => screen.getByText(ALREADY_THERE));
  });
});
