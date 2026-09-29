//! A Conversation whose work has moved to another device, opened by a URL that
//! names the copy left behind (ADR-0020, *Transfer*).
//!
//! **A transferred Conversation has a row in more than one database.** The device
//! it came from keeps its copy so that every link anybody kept still leads to the
//! work, and that copy is a tombstone: nothing writes to it, the merged list does
//! not draw it, and its own URL leads to wherever the record is now. Which is
//! what is asked here — that the page never draws one, and that where it goes
//! instead is the device and the id the mark names.
//!
//! **And that it goes there whether or not that device is answering.** Which copy
//! is the record is not a question about who can be reached: a tombstone drawn
//! because the far end was asleep would be a read-only copy of the work presented
//! as the work. So the far end refusing is a page drawn on the live copy's own
//! path with the error every unreachable member draws, rather than a page drawn
//! on the copy that is not the record.
//!
//! The golden fixtures are the workbench's: `cargo test` renders the real
//! endpoints and writes them, and the mark is a field on the very same record.

import { cleanup, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ConversationView, DevicesView } from "../src/api/types";
import { OPEN, mount, theWorkbench } from "./bench";
import { json, whenever } from "./serving";
import linked from "./fixtures/devices-linked.json" with { type: "json" };

/// The cluster this all happens in: this device, and the laptop the work went
/// to.
const DEVICES = linked as DevicesView;
const THIS_DEVICE = DEVICES.this.device;
const LAPTOP = DEVICES.members[0]!.identity.device;

/// Where a call for the laptop stands: the prefix takes the place of `/api/ui`,
/// so the far end sees the path the browser would have written locally.
const at = (path: string) => `/api/ui/members/${LAPTOP}${path}`;

/// And where a page about one of its Conversations stands.
const on = (path: string) => `/devices/${LAPTOP}${path}`;

/// The id the laptop numbered its copy, which is its own and nothing to do with
/// the id here: every Verkstead issues its own, and the two collide by
/// construction.
const THERE = 9;

/// The copy this device kept: the record the fixtures open, marked as having been
/// handed on to the laptop.
const TOMBSTONE: ConversationView = {
  ...OPEN,
  branch: "the-copy-left-behind",
  transferred: { device: LAPTOP, id: THERE },
};

/// And the live record over there, which is the same work under the laptop's own
/// id.
const LIVE: ConversationView = {
  ...OPEN,
  id: THERE,
  branch: "where-the-work-is",
  transferred: null,
};

/// The workbench with that cluster under it, and whatever the test serves for
/// the two ends of the transfer.
///
/// Anything neither end was asked for is a refusal rather than a throw, so a pane
/// this file is not about draws its error and says nothing here.
function theCluster(...answers: Parameters<typeof theWorkbench>) {
  return theWorkbench(
    whenever("/api/ui/devices", json(DEVICES)),
    ...answers,
    json({ error: "nothing serves that" }, 404),
  );
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("a conversation whose work has moved to another device", () => {
  it("lands on the live copy's device and id", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${OPEN.id}`, json(TOMBSTONE)),
      whenever(at(`/conversations/${THERE}`), json(LIVE)),
    );

    const { history, container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() =>
      expect(history.get().startsWith(on(`/conversations/${THERE}`))).toBe(true),
    );

    // And what is drawn is the record over there rather than the copy kept here,
    // which is the other half of the same sentence.
    await waitFor(() =>
      expect(container.textContent).toContain(LIVE.branch),
    );
    expect(container.textContent).not.toContain(TOMBSTONE.branch);
  });

  it("lands there whether or not that device is answering", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${OPEN.id}`, json(TOMBSTONE)),
      // The laptop's lid is shut: the relay has nowhere to put the call, which
      // is the refusal every unreachable member answers with.
      whenever(
        at(`/conversations/${THERE}`),
        json({ error: "the device is not answering" }, 502),
      ),
    );

    const { history, container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() =>
      expect(history.get()).toBe(on(`/conversations/${THERE}`)),
    );

    // Nothing of the copy is drawn on the way past it, which is the whole point
    // of the redirect being about the record rather than about reachability.
    expect(container.textContent).not.toContain(TOMBSTONE.branch);
  });

  /// And the same thing the other way round, which is what a transfer back
  /// leaves behind: the copy on the laptop is the tombstone now, and the record
  /// is this device's own — so the path it lands on is a local one, with no
  /// `/devices/` in front of it.
  it("lands on a local path where the work has come home", async () => {
    const back: ConversationView = {
      ...LIVE,
      transferred: { device: THIS_DEVICE, id: OPEN.id },
    };

    theCluster(
      whenever(at(`/conversations/${THERE}`), json(back)),
      whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)),
    );

    const { history } = mount(on(`/conversations/${THERE}`));

    await waitFor(() =>
      expect(
        history.get().startsWith(`/conversations/${OPEN.id}`),
      ).toBe(true),
    );
  });

  /// And a Conversation that is the record is drawn where it stands, which is
  /// every Conversation there has never been a transfer of.
  it("draws a conversation that is the record where it stands", async () => {
    theCluster(whenever(`/api/ui/conversations/${OPEN.id}`, json(OPEN)));

    const { history, container } = mount(`/conversations/${OPEN.id}`);

    await waitFor(() => expect(container.textContent).toContain(OPEN.branch));

    expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true);
  });
});
