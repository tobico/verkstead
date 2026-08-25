//! Where a notification goes when it is tapped.
//!
//! Every push the server sends names the page it is about — see `Notice` in
//! `crates/server/src/push.rs` — because what a push is about is the server's to
//! know. A Question Set names its own page; a stop names the Conversation it
//! stopped, so that a phone woken by one lands on the work that stopped rather
//! than on a Set it is not about.

import { describe, expect, it } from "vitest";

import { ORIGIN, UNREADABLE, worker } from "./worker";

/// A Question Set arriving, as the server sends it.
const SET = {
  path: "/sets/7",
  title: "Whether to keep the outbound queue at all",
  project: "verkstead",
};

/// And a run that stopped on something Verkstead decided to stop for.
const STOP = {
  path: "/conversations/3",
  title: "Implementing the work stopped on rate-limiting",
  project: "verkstead",
};

describe("tapping a notification", () => {
  it("opens the Set a Question Set's push was about", async () => {
    const sw = worker();
    const open = sw.opens();

    await sw.pushes(SET);
    await sw.taps(sw.shown[0]!);

    // The window that was already there, sent to the Set: a second Verkstead
    // over the top of the one the human left open is the thing to avoid.
    expect(open.navigated).toEqual([`${ORIGIN}/sets/7`]);
    expect(open.focused).toBe(1);
    expect(sw.opened).toEqual([]);
  });

  it("opens the Conversation a stop's push was about", async () => {
    const sw = worker();
    const open = sw.opens();

    await sw.pushes(STOP);
    await sw.taps(sw.shown[0]!);

    expect(open.navigated).toEqual([`${ORIGIN}/conversations/3`]);
    expect(open.focused).toBe(1);
  });

  it("opens a window where the human left none", async () => {
    const sw = worker();

    await sw.pushes(STOP);
    await sw.taps(sw.shown[0]!);

    // The ordinary case for a phone: the app is closed, and the notification is
    // the whole of the way in.
    expect(sw.opened.map((pane) => pane.url)).toEqual([
      `${ORIGIN}/conversations/3`,
    ]);
  });

  it("opens the workbench where the push could not say", async () => {
    const sw = worker();
    const open = sw.opens(`${ORIGIN}/repos`);

    await sw.pushes(UNREADABLE);
    await sw.taps(sw.shown[0]!);

    // Every Conversation is there, and every Set through the one it was asked
    // from: a notification that lands somewhere is worth more than one that
    // does nothing.
    expect(open.navigated).toEqual([`${ORIGIN}/`]);
  });

  it("closes the notification it was", async () => {
    const sw = worker();
    sw.opens();

    await sw.pushes(SET);
    await sw.taps(sw.shown[0]!);

    // Left standing it would be tapped again, on a decision already made.
    expect(sw.closed).toEqual(sw.shown);
  });
});

describe("the notification a push shows", () => {
  it("is tagged by the page it opens, so a repeat replaces it", async () => {
    const sw = worker();

    await sw.pushes(SET);
    await sw.pushes(SET);
    await sw.pushes(STOP);

    const tags = sw.shown.map((notification) => notification.options.tag);

    expect(tags[0]).toBe(tags[1]);
    expect(tags[2]).not.toBe(tags[0]);
  });
});
