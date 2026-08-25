//! The Nudge: the open page being told what moved, and looking again because of
//! it — over the server's stream, and relayed by the service worker off a push
//! (ADR-0009).
//!
//! What a page does about a Nudge is what its kind stands for, which is one
//! table in `src/nudge.ts` — so most of what is below is about which reads a
//! kind causes and, at least as much, which it does not.
//!
//! Driven through `App` for the same reason `resuming` is — what the Nudge acts
//! on is the app's own query client, and a test that built a client of its own
//! would be asserting its own arrangement rather than the app's.
//!
//! The clock is held still throughout, and there is no longer anything for it to
//! run: the ten-second poll is gone (ADR-0009), so every read a page makes here
//! is one something told it to make.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type {
  AgentOutputEvent,
  ConversationEntry,
  ConversationView,
  Nudge,
  ProfileEntry,
  PullRequestDetails,
  QuestionSetEvent,
  RepoEntry,
  SetView,
  TimelineEvent,
  TranscriptView,
} from "../src/api/types";
// The badge on a waiting Set, and the date a settled one carries.
import sheet from "../src/set/Sheet.module.css";
import standing from "../src/set/Standing.module.css";
// The mark on a session's row, and the pinned card the timeline holds
// above the record.
import marks from "../src/workbench/Mark.module.css";
import outputPane from "../src/workbench/Output.module.css";
import prPane from "../src/workbench/PullRequest.module.css";
import timeline from "../src/workbench/Timeline.module.css";
import { drawn } from "./bench";
import {
  askedFor,
  json,
  readable,
  reads as readingOf,
  serving,
  whenever,
} from "./serving";
import { worker } from "./worker";
import kinds from "./fixtures/nudges.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
import wrapping from "./fixtures/conversation-wrapping.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import said from "./fixtures/transcript.json" with { type: "json" };
import saidSince from "./fixtures/transcript-more.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

/// The renderer is a page's own doing and neither Set fixture has a Diagram;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

/// The Conversation the human is looking at, with a session's Question Sets on
/// its Timeline — which is where a Set arrives now that there is no list of
/// them.
const CONVERSATION = grilling as ConversationView;

/// The Conversation the workbench reads, which is what most of this file
/// counts.
const OPENED = `/api/ui/conversations/${CONVERSATION.id}`;

/// The four lists the workbench draws its panes over: the sidebar, the Repos the
/// picker on it needs, the roadmaps nothing is driving beside them, and the
/// Agent Profiles the details pane picks from.
///
/// Named because they are counted. Which of them a Nudge moves is the whole
/// question of what a kind stands for, and the reason the tests below can tell
/// a narrow reaction from the widest one.
const SIDEBAR = "/api/ui/conversations";
const REPOS = "/api/ui/repos";
const ROADMAPS = "/api/ui/abandoned-roadmaps";
const PROFILES = "/api/ui/profiles";

/// What the app fetches alongside the Conversation it is about, held to their
/// own paths so no test has to say what order a page asks in. The release check
/// is among them and out of every count: a Nudge is never about a release, and
/// nothing on the workbench asks for one.
const BESIDE = [
  whenever(SIDEBAR, json(conversations as ConversationEntry[])),
  whenever(REPOS, json(repos as RepoEntry[])),
  whenever(PROFILES, json(profiles as ProfileEntry[])),
  whenever(ROADMAPS, json([])),
  whenever("/api/ui/update", json("Current")),
];

/// One Set twice over: waiting when the page was drawn, answered from another
/// device by the time a Nudge says to look again.
const WAITING = readable(answering);
const ANSWERED = readable(answered);

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

/// The stream the app opened, newest first — which is the one it is listening
/// on. There is one at a time and not one per app: the connection is given back
/// whenever the page is hidden and taken again when it is looked at, so a page
/// that has been away has opened more than one over its life.
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

/// The document going away and coming back, which is what an iOS PWA being
/// suspended and reopened looks like from inside the page.
///
/// `visibilityState` is read-only here as it is in a browser, so it is redefined
/// for as long as the test needs it — and taken away again after, like the
/// worker container above.
function away(state: "visible" | "hidden"): void {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    value: state,
  });
  document.dispatchEvent(new Event("visibilitychange"));
}

beforeEach(() => {
  vi.useFakeTimers();
  Streaming.opened = [];
  vi.stubGlobal("EventSource", Streaming);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  // Back to the state jsdom's own document reports.
  delete (document as { visibilityState?: unknown }).visibilityState;
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

  it("shows the Set a Nudge is about, the moment it is told", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    stream().nudges();

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // The clock never moved and nothing here runs on one: the second read is
    // the Nudge's doing and nothing else's.
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

  it.each([
    ["a kind this page has never heard of", { kind: "weather", conversation: 1 }],
    ["a Nudge that is not JSON at all", "nudge"],
    ["a Nudge with nothing in it", ""],
  ])("reads everything back for %s", async (_what, wire) => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    const lists = [SIDEBAR, REPOS, ROADMAPS, PROFILES].map((path) =>
      askedFor(fetching, path),
    );
    stream().opens();

    stream().nudges(wire);

    // A page against a server newer than itself, which is every page between a
    // deploy and a reload: what it cannot read it treats as everything having
    // moved, which is what it used to do about everything.
    await waitFor(() => screen.getByText(ARRIVAL.title));
    expect(askedFor(fetching, OPENED)).toBe(2);
    [SIDEBAR, REPOS, ROADMAPS, PROFILES].forEach((path, at) => {
      expect(askedFor(fetching, path), `${path} was not read again`).toBe(
        lists[at]! + 1,
      );
    });
  });

  it("asks for nothing on a timer", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, json(CONVERSATION));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    await vi.advanceTimersByTimeAsync(10 * 60 * 1000);

    // The poll this page used to keep up on is gone (ADR-0009). Ten minutes of
    // it: what stands behind a Nudge that never arrived is the catch-up on
    // coming back, not a clock.
    expect(askedFor(fetching, OPENED)).toBe(1);
  });
});

/// The session's output on the Conversation the fixtures open.
const OUTPUT: AgentOutputEvent = (() => {
  const found = CONVERSATION.timeline.find((event) => "AgentOutput" in event);
  if (!found || !("AgentOutput" in found)) {
    throw new Error("the fixture's Timeline should carry a session's output");
  }
  return found.AgentOutput;
})();

/// The same Conversation with that session still talking — which is the moment
/// this whole file is about, and one no fixture holds: a fixture is a payload
/// rather than a moment.
const TALKING: ConversationView = {
  ...CONVERSATION,
  timeline: CONVERSATION.timeline.map((event) =>
    "AgentOutput" in event
      ? { AgentOutput: { ...event.AgentOutput, running: true } }
      : event,
  ),
};

/// Where the open pane reads what that session has been saying.
const TRANSCRIPT_OF_IT = `/api/ui/conversations/${CONVERSATION.id}/transcript/${OUTPUT.id}`;

/// The record itself, and what the session said after it.
const SAID = said as TranscriptView;
const SAID_SINCE = saidSince as TranscriptView;

/// And where the pane asks for the second of those: the cursor the first
/// reading ended at, handed back as the server wrote it.
const REST_OF_IT = `${TRANSCRIPT_OF_IT}?after=${encodeURIComponent(
  SAID.cursor,
)}`;

/// The workbench with that session's output open, its Transcript to hand — and
/// the rest of it for the reading the next Nudge sets off.
function theTalking() {
  return serving(
    ...BESIDE,
    whenever(OPENED, json(TALKING)),
    whenever(TRANSCRIPT_OF_IT, json(SAID)),
    whenever(REST_OF_IT, json(SAID_SINCE)),
  );
}

/// The Conversation that has a pull request, and what is on it.
const WRAPPED = wrapping as ConversationView;
const WRAPPED_UP = `/api/ui/conversations/${WRAPPED.id}`;

const PULL_REQUEST = (() => {
  const found = WRAPPED.pinned.find((event) => "PullRequest" in event);
  if (!found || !("PullRequest" in found)) {
    throw new Error("the fixture should carry a pull request");
  }
  return found.PullRequest;
})();

/// Where the open pane asks GitHub, through the server, what is on it.
const WHAT_IS_ON_IT = `/api/ui/conversations/${WRAPPED.id}/pull-request/${PULL_REQUEST.id}`;

const CARRIED: PullRequestDetails = {
  commits: [{ sha: "d41f8a3b6c2e91750f4a8c3d5b7e2f10a9c6d4b8", subject: "chore: finish" }],
  comments: [],
};

/// The workbench with that pull request to hand.
function theWrapping() {
  return serving(
    ...BESIDE,
    whenever(WRAPPED_UP, json(WRAPPED)),
    whenever(WHAT_IS_ON_IT, json(CARRIED)),
  );
}

/// The five reads every kind is judged against below.
const COUNTED = [OPENED, SIDEBAR, REPOS, ROADMAPS, PROFILES];

/// How many times each of them has been made.
function reads(fetching: ReturnType<typeof serving>): Record<string, number> {
  return Object.fromEntries(
    COUNTED.map((path) => [path, askedFor(fetching, path)]),
  );
}

/// Which of them each kind of Nudge is about: the viewer's side of the
/// vocabulary, written out where a reader can see the whole of it at once.
///
/// A kind the server writes and this leaves out fails the sweep below rather
/// than quietly reading everything, which is what an unrecognised kind does at
/// runtime and is not something to discover from a fixture.
const ABOUT: Record<string, readonly string[]> = {
  // The kind that arrives twice a second while a session talks: it moves the
  // Transcript alone, and the Transcript is not among the five.
  transcript: [],
  screen: [OPENED],
  commit: [OPENED],
  set: [OPENED, SIDEBAR],
  liveness: [OPENED],
  conversation: [OPENED, SIDEBAR],
  conversations: [SIDEBAR],
  repos: [REPOS, ROADMAPS],
  profiles: [PROFILES],
};

describe("what a Nudge is about", () => {
  it.each(KINDS.map((moved) => [moved.kind, moved] as const))(
    "reads back what a Nudge of kind %s names, and nothing else",
    async (kind, moved) => {
      const about = ABOUT[kind];
      expect(
        about,
        `the server writes a Nudge of kind ${kind} and this page has no reaction for it`,
      ).toBeDefined();

      // Pointed at the Conversation this page has open. The fixture scopes its
      // Nudges to a Conversation of its own, which is what the sweep after this
      // one is about.
      const here =
        "conversation" in moved
          ? { ...moved, conversation: CONVERSATION.id }
          : moved;

      window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
      const fetching = serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
      render(() => <App />);
      await waitFor(() => screen.getByText(ALREADY_THERE));
      stream().opens();
      const before = reads(fetching);

      stream().nudges(here);
      await vi.advanceTimersByTimeAsync(0);

      await waitFor(() => {
        for (const path of COUNTED) {
          expect(askedFor(fetching, path), path).toBe(
            before[path]! + (about!.includes(path) ? 1 : 0),
          );
        }
      });
    },
  );

  it.each(
    KINDS.filter((moved) => "conversation" in moved).map(
      (moved) => [moved.kind, moved] as const,
    ),
  )("leaves another Conversation alone on a Nudge of kind %s", async (_kind, moved) => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();
    const before = askedFor(fetching, OPENED);

    // The fixture's own scope, which is a Conversation nobody here is looking
    // at: a page open on one Conversation hears about every other one, and a
    // scoped kind is what keeps that from costing it a read.
    stream().nudges(moved);
    await vi.advanceTimersByTimeAsync(0);

    expect(askedFor(fetching, OPENED)).toBe(before);
  });

  it("reads a talking session's Transcript back, and nothing beside it", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = theTalking();
    const { container } = render(() => <App />);
    // Opened once the row says the session is still going, which is what the
    // pane reads to decide whether the record can still move: a Transcript
    // opened over a session that had already stopped is read once and never
    // again, whatever any Nudge says.
    await drawn(container, `.${timeline.agentOutput} .${marks.mark}`);

    fireEvent.click(await drawn(container, `.${timeline.agentOutput}`));
    await drawn(container, `.details-pane .${outputPane.turn}`);
    stream().opens();
    const before = { ...reads(fetching), [REST_OF_IT]: askedFor(fetching, REST_OF_IT) };

    stream().nudges({ kind: "transcript", conversation: CONVERSATION.id });

    // The one read a batch of lines is worth, and it asks for the batch rather
    // than for the hour before it. This is the Nudge that arrives twice a
    // second while a session talks, and what it used to cost was the whole
    // record plus every one of the five below — the Repos and the Profiles
    // among them, which nothing a session says can move.
    await waitFor(() =>
      expect(askedFor(fetching, REST_OF_IT)).toBe(before[REST_OF_IT]! + 1),
    );
    for (const path of COUNTED) {
      expect(askedFor(fetching, path), path).toBe(before[path]);
    }
  });

  it("asks GitHub nothing while a session talks", async () => {
    window.history.pushState({}, "", `/conversations/${WRAPPED.id}`);
    const fetching = theWrapping();
    const { container } = render(() => <App />);
    const pinned = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(pinned.querySelector(`.${timeline.openPullRequest}`)!);
    await drawn(container, `.details-pane .${prPane.commits}`);
    stream().opens();
    const before = askedFor(fetching, WHAT_IS_ON_IT);

    for (const kind of ["transcript", "screen", "set", "conversation"]) {
      stream().nudges({ kind, conversation: WRAPPED.id });
    }
    await vi.advanceTimersByTimeAsync(0);

    // This is the read the server answers by asking GitHub through the host's
    // `gh`, and it used to be made on every Nudge of any kind — which, while a
    // session talked, was an API call about twice a second.
    expect(askedFor(fetching, WHAT_IS_ON_IT)).toBe(before);
  });

  it("asks GitHub again when a commit lands on its Conversation", async () => {
    window.history.pushState({}, "", `/conversations/${WRAPPED.id}`);
    const fetching = theWrapping();
    const { container } = render(() => <App />);
    const pinned = await drawn(container, `.${timeline.pinned} .${timeline.pullRequest}`);
    fireEvent.click(pinned.querySelector(`.${timeline.openPullRequest}`)!);
    await drawn(container, `.details-pane .${prPane.commits}`);
    stream().opens();
    const before = askedFor(fetching, WHAT_IS_ON_IT);

    stream().nudges({ kind: "commit", conversation: WRAPPED.id });

    // A commit landing is what puts a commit on a pull request, so this is the
    // one thing that moves what the pane is showing.
    await waitFor(() =>
      expect(askedFor(fetching, WHAT_IS_ON_IT)).toBe(before + 1),
    );
  });

  it("moves the badge on a waiting Set when its agent goes", async () => {
    const DISCONNECTED: SetView = {
      ...WAITING,
      standing: { Waiting: "disconnected" },
    };

    window.history.pushState({}, "", `/sets/${WAITING.id}`);
    const fetching = serving(...BESIDE, json(readingOf(WAITING)), json(readingOf(DISCONNECTED)));
    const { container } = render(() => <App />);
    await drawn(container, `.${standing.liveness}.${standing.waiting}`);
    stream().opens();

    // Scoped to a Conversation, and the Set is keyed by its own id: what a page
    // showing one Set does about a Set moving somewhere is read the Set it is
    // showing, which is one read at most.
    stream().nudges({ kind: "liveness", conversation: WAITING.conversation });

    // The verdict used to cycle with the ten-second poll. The poll is gone, so
    // the agent letting go of its wait is a Nudge of its own (ADR-0009).
    await drawn(container, `.${standing.liveness}.${standing.disconnected}`);
    expect(askedFor(fetching, `/api/ui/sets/${WAITING.id}`)).toBe(2);
  });
});

describe("the connection the stream holds", () => {
  /// A browser gives one origin six connections over HTTP/1.1, and a stream held
  /// for as long as a page lives pins one of them for as long as the page is
  /// open. Six Verksteads left open in tabs is every connection pinned, and from
  /// there the page in front of the human waits for one that nothing is going to
  /// give back — which is a workbench that loads nothing at all.
  it("lets the connection go while the page is not being looked at", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    away("hidden");

    expect(stream().closed).toBe(true);
  });

  /// And takes one again when it is. A page that was not listening is a page
  /// that has to be told everything, which is exactly what coming back already
  /// does — so what it hears from here on is the news, and what it missed is the
  /// read it is making anyway.
  it("listens again when the page is looked at", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();
    away("hidden");

    away("visible");

    expect(stream().closed).toBe(false);

    // And it is a stream that hears Nudges, rather than one nothing is listening
    // on: what arrives down this one is drawn like anything else.
    stream().opens();
    stream().nudges();
    await waitFor(() => screen.getByText(ARRIVAL.title));
  });

  /// The stream opening for a page that has come back is not a reconnect, and
  /// must not be read as one: coming back is already a read of everything, and a
  /// second one over the top of it would be the whole world fetched twice every
  /// time somebody changed tabs.
  it("reads the world back once when the page comes back", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();
    const before = askedFor(fetching, OPENED);

    away("hidden");
    away("visible");
    // The connection is made again before the page reads, so the browser's own
    // open lands after the catch-up rather than instead of it.
    stream().opens();
    await vi.advanceTimersByTimeAsync(0);

    expect(askedFor(fetching, OPENED)).toBe(before + 1);
  });

  /// A page nobody has looked at yet holds nothing either — every browser starts
  /// a background tab hidden, and a tab opened behind the one being read is
  /// exactly the case this is all about.
  it("opens no stream at all for a page that starts hidden", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "hidden",
    });

    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));

    expect(Streaming.opened).toEqual([]);
  });
});

describe("coming back to a page that was away", () => {
  it("reads everything back when the document becomes visible again", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    // Moved on while the page was away, with no Nudge to say so: a suspended
    // PWA hears nothing at all, which is the whole reason this is here.
    serving(...BESIDE, json(CONVERSATION), json(MOVED_ON));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();

    away("visible");

    await waitFor(() => screen.getByText(ARRIVAL.title));
  });

  it("asks for nothing when the document goes away", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
    render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    stream().opens();
    const before = reads(fetching);

    away("hidden");
    await vi.advanceTimersByTimeAsync(0);

    // The same event fires both ways round. A page on its way out has nothing
    // to catch up on.
    for (const path of COUNTED) {
      expect(askedFor(fetching, path), path).toBe(before[path]);
    }
  });

  it("stops watching for it when the app goes", async () => {
    window.history.pushState({}, "", `/conversations/${CONVERSATION.id}`);
    const fetching = serving(...BESIDE, whenever(OPENED, json(CONVERSATION)));
    const { unmount } = render(() => <App />);
    await waitFor(() => screen.getByText(ALREADY_THERE));
    const before = reads(fetching);

    unmount();
    away("visible");
    await vi.advanceTimersByTimeAsync(0);

    // Nothing is left holding a query client that has no page to refresh.
    for (const path of COUNTED) {
      expect(askedFor(fetching, path), path).toBe(before[path]);
    }
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
    // the page never came back into view either. The push is the whole of how
    // this page found out.
    expect(askedFor(fetching, OPENED)).toBe(read + 1);
  });

  it("reads everything back, exactly as a Nudge on the stream does", async () => {
    window.history.pushState({}, "", `/sets/${WAITING.id}`);
    serving(...BESIDE, json(readingOf(WAITING)), json(readingOf(ANSWERED)));
    const container = attaches();
    const { container: page } = render(() => <App />);
    // The badge and the menu under it belong to a Set still waiting: the page
    // as it was drawn.
    await waitFor(() => expect(page.querySelector(`.${standing.standing}`)).toBeTruthy());

    await pushed(container);

    // A relayed Nudge says nothing at all where a streamed one says a kind, and
    // gets the reaction a kind nobody recognises gets: everything this page is
    // showing, which here is a Set answered elsewhere in the meantime.
    await waitFor(() => expect(page.querySelector(`.${sheet.answeredAt}`)).toBeTruthy());
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
