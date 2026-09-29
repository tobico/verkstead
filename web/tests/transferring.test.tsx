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

import { cleanup, fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import dropdown from "../src/Menu.module.css";
import shell from "../src/Panes.module.css";
import sidebar from "../src/workbench/Conversations.module.css";
import type {
  ConversationView,
  DevicesView,
  Preflight,
} from "../src/api/types";
import { harnessAbsentOn, UNREACHABLE } from "../src/broken";
import chrome from "../src/picking.module.css";
import { rowKey } from "../src/reaching";
import actions from "../src/workbench/Actions.module.css";
import setup from "../src/workbench/Setup.module.css";
import transfer from "../src/workbench/Transfer.module.css";
import { OPEN, drawn, mount, theWorkbench } from "./bench";
import { offered, pick, rows } from "./pickers";
import { json, whenever } from "./serving";
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };
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

/// The Conversation the dialog is opened over: one the work has started in, its
/// grilling paired with an account on this device.
///
/// A Grilling rather than the Draft the fixtures open, because *Transfer to…* is
/// drawn from every state but Draft and Closed — a Draft is moved by the device
/// select on its own composer, and a Closed Conversation has no work left to
/// move.
const GRILLING = grilling as ConversationView;

/// Where the preflight of a device is read: on the device the Conversation is
/// on — which is this one, here — naming the machine it is asked about.
const preflightOf = (device: string) =>
  `/api/ui/conversations/${GRILLING.id}/preflight/${device}`;

/// What a device with nothing in the way answers.
///
/// Written here rather than read out of a fixture, for the reason the branch
/// lists in `bench.tsx` are: a golden file of this endpoint would need two
/// linked Verksteads behind it, and what keeps these honest is that they are
/// typed — `crates/server/tests/preflight.rs` is where the shape is proved
/// against real machines.
const READY = { device: "laptop", lacks: [] } satisfies Preflight;

/// And the three findings, each named against the machine it is about.
const NO_REPO = {
  device: "laptop",
  lacks: [{ Repo: { name: "verkstead", companion: false } }],
} satisfies Preflight;

const NO_HARNESS = {
  device: "laptop",
  lacks: [
    { Harness: { role: "Grilling", profile: "fable", agent_type: "Claude" } },
  ],
} satisfies Preflight;

const ASLEEP = { device: "laptop", lacks: ["Unreachable"] } satisfies Preflight;

/// Open the Conversation's action menu: press the trigger, and hand back what
/// it drops.
async function openActions(container: ParentNode): Promise<HTMLElement> {
  fireEvent.click(
    await drawn(
      container,
      `.${actions.conversationActions} > .${dropdown.trigger}`,
    ),
  );

  return drawn(container, `.${actions.conversationActions} > .${dropdown.drop}`);
}

/// The dialog *Transfer to…* opens, or `null` while nothing has opened one.
function dialog(): HTMLDialogElement | null {
  return document.body.querySelector<HTMLDialogElement>(
    `dialog.${transfer.transferring}`,
  );
}

/// The same, waited for: the press opens it a signal away rather than a request
/// away, but it is still not there on the tick the click was made on.
function opened(): Promise<HTMLDialogElement> {
  return waitFor(() => {
    const card = dialog();
    if (!card) throw new Error("no transfer dialog is open");
    return card;
  });
}

/// Press *Transfer to…* on the Conversation pane's own menu, and hand back the
/// dialog it opens — with its device select drawn, which waits on the
/// membership landing: the select is not there at all until this device has
/// read its own cluster.
async function transferring(container: ParentNode): Promise<HTMLDialogElement> {
  await openActions(container);
  fireEvent.click(await drawn(container, `.${actions.transfer}`));

  const card = await opened();
  await drawn(card, `.${setup.deviceSelect}`);

  return card;
}

/// The press that would move the work, which is drawn either enabled or
/// refused and carries nothing behind it yet.
function go(card: HTMLDialogElement): HTMLButtonElement {
  const found = card.querySelector<HTMLButtonElement>(`.${transfer.go}`);
  if (!found) throw new Error("the dialog has no Go");
  return found;
}

/// What the preflight found, as the dialog draws it.
function findings(card: HTMLDialogElement): string[] {
  return [...card.querySelectorAll(`.${transfer.lacks} li`)].map(
    (said) => said.textContent ?? "",
  );
}

describe("transferring a conversation to another device", () => {
  /// The row itself, on the menu the Conversation pane drops and on the same
  /// rows the sidebar's right-click drops — they are one set of rows, which is
  /// the whole reason `actions` is a factory rather than a component.
  it("is on both menus for a conversation past drafting", async () => {
    theCluster(whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)));

    const { container } = mount(`/conversations/${GRILLING.id}`);

    const menu = await openActions(container);
    expect(menu.querySelector(`.${actions.transfer}`)).toBeTruthy();

    // Away from that menu, so what is drawn next is the sidebar's own and not
    // the one still hanging over the pane.
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() =>
      expect(document.body.querySelector(`.${actions.transfer}`)).toBeNull(),
    );

    // And the same row under the pointer on the card in the sidebar, which is
    // very often about a Conversation no pane is showing. The card carries
    // seven fields and the rows need a good deal more, so the Conversation is
    // read again before there is anything to draw.
    fireEvent.contextMenu(
      await drawn(
        container,
        `[data-row="${rowKey(null, GRILLING.id)}"] .${sidebar.open}`,
      ),
      { clientX: 120, clientY: 200 },
    );

    const card = await drawn(
      container,
      `.${shell.conversationsPane} .${actions.conversationActions} > .${dropdown.drop}`,
    );

    await waitFor(() =>
      expect(card.querySelector(`.${actions.transfer}`)).toBeTruthy(),
    );
  });


  /// And on neither menu at all where this Verkstead is linked to nothing,
  /// which is nearly every one of them: a machine in no cluster has nowhere to
  /// move a Conversation to, and a row that opened a dialog with no device in
  /// it would be a press with nothing behind it.
  it("is on neither menu where there is no cluster", async () => {
    theWorkbench(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const menu = await openActions(container);

    // The rows around it are there, so this is the row being left out rather
    // than the menu not having been drawn.
    await drawn(menu, `.${actions.steer}`);
    expect(menu.querySelector(`.${actions.transfer}`)).toBeNull();
  });
  /// And it is not on either of the two states there is nothing to move from: a
  /// Draft, which is moved by its own composer's device select, and a Closed
  /// Conversation, whose work has ended.
  it("is on neither menu for a draft or a closed conversation", async () => {
    for (const state of ["Draft", "Closed"] as const) {
      theCluster(
        whenever(
          `/api/ui/conversations/${GRILLING.id}`,
          json({ ...GRILLING, state } satisfies ConversationView),
        ),
      );

      const { container } = mount(`/conversations/${GRILLING.id}`);
      const menu = await openActions(container);

      expect(
        menu.querySelector(`.${actions.transfer}`),
        `a ${state} conversation has nothing to transfer`,
      ).toBeNull();

      cleanup();
      vi.unstubAllGlobals();
    }
  });

  /// The select inside it is the compose page's own, with the one device it
  /// cannot offer left out: a Conversation cannot be moved onto the machine it
  /// is already on.
  it("offers every device of the cluster but the one the work is on", async () => {
    theCluster(whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)));

    const { container } = mount(`/conversations/${GRILLING.id}`);
    await transferring(container);

    // Every member, and one row fewer than the cluster has devices: this
    // device's own is the one machine the work cannot be moved onto, and the
    // names alone could not say so — two machines of one cluster are called
    // `workbench` here, which is exactly why the row that goes is decided by
    // the Device Id rather than by what anything reads as.
    expect(rows("Device")).toEqual(
      DEVICES.members.map((member) => member.identity.name),
    );
    expect(offered("Device")).toHaveLength(DEVICES.members.length);

    // And each row wears the mark for the machine's operating system, which is
    // what tells two devices of one name apart at a glance.
    for (const row of offered("Device")) {
      expect(row.querySelector(`.${chrome.mark}`)).toBeTruthy();
    }
  });

  /// And picking one sets the preflight going, which is what the dialog draws
  /// under the select. Nothing in the way is Go drawn enabled — the one state
  /// in which it is.
  it("draws the picked device's preflight and enables Go where nothing is in the way", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(READY)),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);

    // Nothing is asked of any device until one is picked, and Go is refused
    // while nothing has been.
    expect(go(card).disabled).toBe(true);

    pick("Device", "laptop");

    await waitFor(() => expect(go(card).disabled).toBe(false));
    expect(card.textContent).toContain("laptop has everything");
  });

  /// A Repo the far end has no match for is named, points at **Open repo** on
  /// that device, and leaves Go refused.
  it("names a repo the far end has no match for and refuses Go", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(NO_REPO)),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(findings(card)).toHaveLength(1));

    const said = findings(card)[0]!;
    expect(said).toContain("verkstead");
    expect(said).toContain("laptop");
    expect(said).toContain("Open repo");
    expect(go(card).disabled).toBe(true);
  });

  /// And a harness the far end has not got is named against the Pairing that
  /// wants it, in the words the onboarding probe says it in.
  it("names the pairing whose harness the far end has not got", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(NO_HARNESS)),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(findings(card)).toHaveLength(1));

    const said = findings(card)[0]!;
    expect(said).toContain("Grilling");
    expect(said).toContain("fable");
    expect(said).toContain(harnessAbsentOn("Claude", "laptop"));
    expect(go(card).disabled).toBe(true);
  });

  /// And a device that answered nothing is refused as unreachable, by its own
  /// name — never as *no match*, which would send the human to open a
  /// repository on a machine that may already have it.
  it("refuses a device that is not answering by its own name", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(ASLEEP)),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(findings(card)).toHaveLength(1));

    const said = findings(card)[0]!;
    expect(said).toContain("laptop");
    expect(said).toContain(UNREACHABLE);
    expect(said).not.toContain("Open repo");
    expect(go(card).disabled).toBe(true);
  });
});

describe("pressing Go", () => {
  /// Where the press goes: to the device the Conversation is on, naming the
  /// machine it is to be moved to — the same two devices the reading names,
  /// in the same two places.
  const transferTo = (device: string) =>
    `/api/ui/conversations/${GRILLING.id}/transfer/${device}`;

  /// The press, and the card shutting on it. Nothing here says the work has
  /// arrived — what comes back says it is *going*, the session's turn being
  /// what it waits for.
  it("writes the move down and shuts the card", async () => {
    const fetching = theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(READY)),
      whenever(transferTo(LAPTOP), json("Transferring"), "POST"),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(go(card).disabled).toBe(false));
    fireEvent.click(go(card));

    await waitFor(() => expect(dialog()).toBeNull());

    // And the press was made on the one path, which is where the two devices
    // are: a press put to the wrong end would be this device asking the laptop
    // to move work it does not have.
    expect(
      fetching.mock.calls.filter(
        ([path, init]) =>
          String(path) === transferTo(LAPTOP) && init?.method === "POST",
      ),
    ).toHaveLength(1);
  });

  /// And nothing is pressed for a device that was never picked: Go is refused
  /// while the reading holds anything, which is what the preflight is for.
  it("is refused while the far end is missing something", async () => {
    const fetching = theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(NO_REPO)),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(findings(card)).toHaveLength(1));

    fireEvent.click(go(card));

    expect(
      fetching.mock.calls.filter(
        ([path, init]) =>
          String(path) === transferTo(LAPTOP) && init?.method === "POST",
      ),
    ).toHaveLength(0);
    expect(dialog()).not.toBeNull();
  });

  /// The press asks the preflight again, because the reading here was drawn a
  /// moment ago and a machine can go to sleep in a moment. What it brings back
  /// is drawn in place of this card's own — the same sentences, about the world
  /// the press arrived in — and the card stays up.
  it("draws what the press found in the way, and stays open", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(READY)),
      whenever(transferTo(LAPTOP), json({ Lacking: ASLEEP }), "POST"),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(go(card).disabled).toBe(false));
    fireEvent.click(go(card));

    await waitFor(() => expect(findings(card)).toHaveLength(1));

    expect(findings(card)[0]!).toContain(UNREACHABLE);
    expect(go(card).disabled).toBe(true);
    expect(dialog()).not.toBeNull();
  });

  /// And a press refused in a word says so where the reading's own failure is
  /// said, rather than shutting on a move nobody made.
  it("says why a press that moved nothing was refused", async () => {
    theCluster(
      whenever(`/api/ui/conversations/${GRILLING.id}`, json(GRILLING)),
      whenever(preflightOf(LAPTOP), json(READY)),
      whenever(transferTo(LAPTOP), json("Elsewhere"), "POST"),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);
    const card = await transferring(container);
    pick("Device", "laptop");

    await waitFor(() => expect(go(card).disabled).toBe(false));
    fireEvent.click(go(card));

    await waitFor(() =>
      expect(card.textContent).toContain("already been moved"),
    );

    expect(dialog()).not.toBeNull();
  });

  /// And from the press until the work lands, the head of the timeline says
  /// where it is going — named, because the machine is the whole of what the
  /// human wants to read there.
  it("says on the conversation where the work is going", async () => {
    theCluster(
      whenever(
        `/api/ui/conversations/${GRILLING.id}`,
        json({
          ...GRILLING,
          transferring: "laptop",
        } satisfies ConversationView),
      ),
    );

    const { container } = mount(`/conversations/${GRILLING.id}`);

    await waitFor(() =>
      expect(container.textContent).toContain("Transferring to laptop"),
    );
  });
});
