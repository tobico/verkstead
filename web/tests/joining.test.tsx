//! The question another machine asks: the modal a join raises, wherever the
//! human happens to be looking.
//!
//! **Mounted on its own rather than through a page**, because it belongs to no
//! page: it is drawn in the shell every page sits inside, beside the toast
//! layer, and what it is drawn over is whatever the human was reading. So what
//! is asked here is what the card says and what the two presses do, and the
//! shell's half — that it is there at all — is `App.tsx`'s own.
//!
//! The reading is the fixture the server's own tests wrote, so what the card is
//! drawn from is the shape the endpoint really answers with — see
//! `crates/server/tests/ui_content.rs`, and `tests/joining.rs` beside it, whose
//! subject that shape is.
//!
//! **The fingerprint is the point of the card.** The name and the OS say which
//! machine before anybody bothers with thirty-two bytes of hex; the hex is what
//! two people at two screens read to each other. So it is drawn whole and it is
//! asserted whole.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { describe, expect, it } from "vitest";

// The column every page sits in, which is where the card is drawn: beside the
// toast layer, and there once — see `src/App.tsx`.
import { Shell } from "../src/App";
import { Joining } from "../src/Joining";
import type { AskingDevice } from "../src/api/types";
import asking from "./fixtures/joins-asking.json" with { type: "json" };
import type { Answer } from "./serving";
import { hangs, json, serving, whenever } from "./serving";

/// Where the shell reads the question, and where the two presses go.
const ASKING = "/api/ui/devices/asking";

/// The device asking, off the fixture: a WSL, which is the case the OS word
/// exists for and the one somebody reading this card is deciding about.
const LAPTOP = (asking as AskingDevice[])[0]!;

/// A second device asking behind it, for the one test about what happens when
/// the first is answered. The same shape with another machine in it — one
/// fixture is what the endpoint answers with, and a second row is this file's
/// own arithmetic on it.
const DESK: AskingDevice = {
  request: "aabbccddeeff0011",
  identity: {
    device: "ffeeddccbbaa00998877665544332211",
    fingerprint: "11:22:33:44:55:66:77:88",
    name: "desk",
    os: "Windows",
    addresses: ["192.168.1.9"],
  },
};

/// The modal over whatever the stub is answering with.
///
/// No retries: a test that asked for a refusal should see it at once, rather
/// than after the three attempts a real page is right to make.
function mount(...answers: Array<Answer>) {
  const fetching = serving(...answers);

  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <Joining />
      </QueryClientProvider>
    )),
    fetching,
    // For the test about a Nudge: `invalidateQueries()` on this is exactly what
    // one does to the card — see `lookAgain` in `src/nudge.ts`.
    client,
  };
}

/// The card, once it is up.
function card(): Promise<HTMLElement> {
  return screen.findByRole("dialog");
}

/// One of the two presses on it, read afresh: the card is rebuilt over whichever
/// device is asking now, so a button held from before an answer landed is a node
/// no longer on the page.
function pressing(name: "Allow" | "Deny"): HTMLButtonElement {
  return screen.getByRole("button", { name }) as HTMLButtonElement;
}

/// Which paths a press was made against, in the order they went out.
function posted(fetching: ReturnType<typeof serving>): string[] {
  return fetching.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === "POST")
    .map(([path]) => String(path));
}

describe("a device asking to link", () => {
  /// The four things the card names a machine by. Three of them say which
  /// machine it is at a glance; the fourth is the one that is read aloud.
  it("says who is asking, on what, from where, and under which certificate", async () => {
    mount(whenever(ASKING, json([LAPTOP])));

    const drawn = await card();

    expect(drawn.textContent).toContain("laptop is asking to link");
    expect(drawn.textContent).toContain("Linux (WSL)");
    expect(drawn.textContent).toContain("laptop.tailnet-name.ts.net");
    expect(drawn.textContent).toContain(LAPTOP.identity.fingerprint);
  });

  /// The first address it advertised and not the rest: that is the one it would
  /// rather be reached on — the tailnet name before the LAN — and a card that
  /// listed every interface would be asking the human to read a network map.
  it("names the address it would rather be reached on, and not the others", async () => {
    mount(whenever(ASKING, json([LAPTOP])));

    const drawn = await card();

    expect(drawn.textContent).toContain("laptop.tailnet-name.ts.net");
    expect(drawn.textContent).not.toContain("192.168.1.31");
  });

  /// Nothing at all where nobody is asking, which is every machine on almost
  /// every day: the card is drawn over the page, and a page with an empty one
  /// over it is a page nobody can press anything on.
  it("draws nothing at all where nobody is asking", async () => {
    mount(whenever(ASKING, json([])));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  /// And nothing where the read itself failed. There is nobody being asked
  /// anything, and a refusal about a question nobody knows was asked is not
  /// something to put in front of somebody reading a Transcript.
  it("draws nothing where the read could not be made at all", async () => {
    mount(whenever(ASKING, json({ error: "no device identity" }, 503)));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  /// Allow goes to the route that records a member, and the card goes with the
  /// answer rather than after a second read.
  it("lets the device in, and the card goes on the answer", async () => {
    const { fetching } = mount(
      whenever(ASKING, json([LAPTOP])),
      whenever(`${ASKING}/${LAPTOP.request}/allow`, json([]), "POST"),
    );

    fireEvent.click(await screen.findByRole("button", { name: "Allow" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(posted(fetching)).toEqual([`${ASKING}/${LAPTOP.request}/allow`]);
  });

  /// And Deny goes to the one beside it, which records nothing.
  it("turns the device away, and the card goes on that answer too", async () => {
    const { fetching } = mount(
      whenever(ASKING, json([LAPTOP])),
      whenever(`${ASKING}/${LAPTOP.request}/deny`, json([]), "POST"),
    );

    fireEvent.click(await screen.findByRole("button", { name: "Deny" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(posted(fetching)).toEqual([`${ASKING}/${LAPTOP.request}/deny`]);
  });

  /// Neither press can be made twice, and neither can be made while the other is
  /// in flight: the two are one decision, and a Deny landing behind an Allow
  /// would be the human answering the same question both ways.
  it("puts both presses out while either is on its way", async () => {
    const { fetching } = mount(
      whenever(ASKING, json([LAPTOP])),
      whenever(`${ASKING}/${LAPTOP.request}/allow`, hangs(), "POST"),
    );

    fireEvent.click(await screen.findByRole("button", { name: "Allow" }));

    await waitFor(() => expect(pressing("Allow").disabled).toBe(true));
    expect(pressing("Deny").disabled).toBe(true);

    fireEvent.click(pressing("Allow"));
    fireEvent.click(pressing("Deny"));

    expect(posted(fetching)).toEqual([`${ASKING}/${LAPTOP.request}/allow`]);
  });

  /// And the card goes when the question does, whoever answered it: a press on
  /// another workbench and the ten minutes running out are both a read that
  /// comes back without the request in it, which is the one thing this reacts
  /// to.
  it("goes when the question stops being asked, with nothing pressed here", async () => {
    let held: AskingDevice[] = [LAPTOP];

    const { client } = mount(whenever(ASKING, () => json(held)()));

    await card();

    // Somebody pressed on the other workbench, or the ten minutes were up. Either
    // way what reaches this page is a Nudge, and what a Nudge comes to is this
    // read made again — see `src/nudge.ts`.
    held = [];
    await client.invalidateQueries();

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  /// Two devices asking at once is one card at a time, oldest first: answering
  /// the first raises the second out of the same answer. A stack of them would
  /// be two irreversible questions with one Escape between them.
  it("raises the next device asking once the first has been answered", async () => {
    mount(
      whenever(ASKING, json([LAPTOP, DESK])),
      whenever(`${ASKING}/${LAPTOP.request}/allow`, json([DESK]), "POST"),
    );

    const first = await card();
    expect(first.textContent).toContain("laptop is asking to link");
    expect(first.textContent).not.toContain("desk is asking to link");

    fireEvent.click(screen.getByRole("button", { name: "Allow" }));

    await waitFor(() =>
      expect(screen.getByRole("dialog").textContent).toContain(
        "desk is asking to link",
      ),
    );
  });

  /// And it is the shell that draws it, which is what makes it reach the human
  /// whatever page they are on: the card comes up over a page that knows nothing
  /// about devices and asked for nothing.
  ///
  /// Asked against the real `Shell` rather than against a page, because that is
  /// the claim — a card drawn by any page would be one the human only sees where
  /// they happen to be.
  it("comes up over whatever page the shell is carrying", async () => {
    serving(whenever(ASKING, json([LAPTOP])));

    render(() => (
      <QueryClientProvider
        client={
          new QueryClient({ defaultOptions: { queries: { retry: false } } })
        }
      >
        <Shell>
          <p>Any page at all.</p>
        </Shell>
      </QueryClientProvider>
    ));

    expect(await card()).toBeTruthy();
    expect(screen.getByText("Any page at all.")).toBeTruthy();
  });

  /// Escape is not a way out of this one. Nobody opened it, so there is nowhere
  /// for closing it to go back to — and a question pressed away from would run
  /// out ten minutes later with nothing on the page ever having said it was
  /// asked. See `insist` on the Modal, which exists for this card.
  it("stays up under Escape, there being no way out of it but the two presses", async () => {
    mount(whenever(ASKING, json([LAPTOP])));

    const drawn = await card();

    fireEvent.keyDown(document, { key: "Escape" });

    expect(screen.getByRole("dialog")).toBe(drawn);
    expect((drawn as HTMLDialogElement).open).toBe(true);
  });

  /// And nor is a press away from the card, for the same reason.
  it("stays up under a press on the backdrop", async () => {
    mount(whenever(ASKING, json([LAPTOP])));

    const drawn = await card();

    fireEvent.click(drawn);

    expect((drawn as HTMLDialogElement).open).toBe(true);
  });
});
