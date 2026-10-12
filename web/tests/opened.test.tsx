//! A click on one of the app's own notifications, as the page follows it: told
//! the path over the bridge, and routed there in place.
//!
//! **The stub bridge is the app telling**, which is the one thing about this no
//! test on this side could otherwise stand in for. What it keeps is the listener
//! the page handed it and whether the page has stopped listening, and a click is
//! that listener called with a path — exactly what the preload does with the
//! channel's message, the event taken off. The app's side of the same channel is
//! `desktop/src/window.ts`, and the literal both sides meet at is pinned here and
//! in `desktop/tests/bridge.test.ts`.
//!
//! **Mounted through the real gate**, so the router the path is followed in is
//! the table the app builds, and the shell the listener is held by is the one
//! every page sits in: a path routed to is a page drawn, and a page loaded again
//! would be the shell drawn again — which is what the last half of the first case
//! reads.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Gate } from "../src/App";
import { follow } from "../src/opened";
import { NAME, OPEN as CHANNEL, type Bridge } from "../src/settings/bridge";
import { OPEN, SET_UP, theWorkbench } from "./bench";
import { json, whenever } from "./serving";

afterEach(() => {
  vi.unstubAllGlobals();
});

/// The app's window, stood in for: what the page handed it to be told on, and
/// how many times the page has stopped listening.
type Telling = {
  click(path: string): void;
  readonly listening: number;
  readonly stopped: number;
};

function standingIn(): Telling {
  let heard: ((path: string) => void) | undefined;
  let listening = 0;
  let stopped = 0;

  vi.stubGlobal(NAME, {
    platform: "linux",
    settings: () => Promise.resolve({ whenClosed: "tray", trayIcon: true }),
    set: () => Promise.resolve({ whenClosed: "tray", trayIcon: true }),
    logs: () => Promise.resolve(),
    startup: () => Promise.resolve({ possible: false, on: false }),
    register: () => Promise.resolve({ possible: false, on: false }),
    head: () => {},
    opening: (listener) => {
      heard = listener;
      listening += 1;

      return () => {
        heard = undefined;
        stopped += 1;
      };
    },
  } satisfies Bridge);

  return {
    click(path) {
      if (heard === undefined) {
        throw new Error("nothing on the page is listening");
      }
      heard(path);
    },
    get listening() {
      return listening;
    },
    get stopped() {
      return stopped;
    },
  };
}

/// The app at `path`, set up, over the workbench's own readings.
function at(path: string) {
  window.history.pushState({}, "", path);
  theWorkbench(whenever("/api/ui/onboarding", json(SET_UP)));

  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={client}>
      <Gate />
    </QueryClientProvider>
  ));
}

describe("a clicked notification", () => {
  /// A Set's notice opens the Timeline Event it is read on, and a stop's the
  /// Conversation — both routed to in the page that was already there.
  it("is routed to in place", async () => {
    const app = standingIn();
    const { container } = at("/settings");

    await waitFor(() => expect(app.listening).toBe(1));
    const shell = container.querySelector("main");
    expect(shell).not.toBeNull();

    const event = `/conversations/${OPEN.id}/events/12`;
    app.click(event);
    await waitFor(() => expect(window.location.pathname).toBe(event));

    // A stop's notice names the Conversation, which the workbench then opens at
    // the end of its Timeline the way it opens any Conversation it lands on — so
    // what is read is that the Event has gone and the Conversation is the one.
    // So the page is taken somewhere else first, for the second click to be seen
    // to move it.
    app.click("/settings");
    await waitFor(() => expect(window.location.pathname).toBe("/settings"));
    app.click(`/conversations/${OPEN.id}`);
    await waitFor(() =>
      expect(window.location.pathname).toMatch(
        new RegExp(`^/conversations/${OPEN.id}(/|$)`),
      ),
    );

    // The same shell, which a load of the path would have drawn again — and the
    // same one listener, which it would have handed the app a second time.
    expect(container.querySelector("main")).toBe(shell);
    expect(app.listening).toBe(1);
  });

  /// The router reads two slashes as another host, as a browser does, and the
  /// window never leaves the workbench.
  it("is not followed off the workbench", async () => {
    const app = standingIn();
    at("/settings");

    await waitFor(() => expect(app.listening).toBe(1));
    app.click("//elsewhere.example/x");
    app.click("elsewhere");

    await screen.findAllByText(/./);
    expect(window.location.pathname).toBe("/settings");
  });

  /// Stopping is what tells the app there is no page to tell, so that a click
  /// after it loads the window on the path instead.
  it("stops being listened for when the page goes", async () => {
    const app = standingIn();
    const { unmount } = at("/settings");

    await waitFor(() => expect(app.listening).toBe(1));
    unmount();

    expect(app.stopped).toBe(1);
  });
});

describe("outside the app", () => {
  /// A browser and a phone have no bridge, and nothing to listen to.
  it("listens to nothing", () => {
    const go = vi.fn();
    const stop = follow(go);

    expect(go).not.toHaveBeenCalled();
    expect(stop).toBeTypeOf("function");
    stop();
  });
});

describe("the channel it is told on", () => {
  /// The literal the preload listens on for the page — pinned on this side as
  /// `desktop/tests/bridge.test.ts` pins it on that one.
  it("is the app's own word for opening", () => {
    expect(CHANNEL).toBe("verkstead:open");
  });
});
