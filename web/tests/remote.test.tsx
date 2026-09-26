//! What the Remote access section says of a machine, which is four different
//! things about four different machines.
//!
//! The point of the section is that its three unhappy states are told apart. A
//! machine with no Tailscale wants an install and nothing on the page can do
//! it; one whose daemon is not answering wants a command run in a terminal, and
//! the line it printed is what names the service; one that is up wants neither,
//! and the question about it is whether the workbench is served. A section that
//! collapsed any two of those into "not reachable" would leave the human
//! guessing which.
//!
//! Two halves mounted apart, because that is what they are: a card in the
//! middle pane saying how things stand, and the controls in the details pane it
//! opens — the box that serves the workbench, the way in, and the key.
//!
//! **The box will not lock the page out.** A browser that reached this pane over
//! the served address is here because the serve is on, so unticking it from
//! there would take away the connection carrying the press. Which page this is
//! is settled in the client, out of the address the reading already carries —
//! so the two tests for it are the same machine read from two hostnames.
//!
//! And the last of it is about the way in rather than about the machine: the
//! login link, drawn as a QR code and offered to copy, with **Reset key** under
//! it. The code is read back the way a camera reads one — see [`scanned`] —
//! because what is being asked about there is the drawing rather than the
//! encoding: a grid written out transposed encodes perfectly and scans as
//! nothing.
//!
//! **And Devices is the third section of the same pane**, which is a reading of
//! its own rather than another field of the machine: what device this is, and
//! every other device in its cluster, a row apiece. It is drawn on every state
//! of the pane —
//! a machine with no Tailscale at all still has an identity — and the clause
//! the card carries follows every one of the six sentences above it.
//!
//! The reads are fixtures the server's own tests wrote, so what the page is
//! drawn from is the shape the endpoint really answers with — see
//! `crates/server/tests/remote.rs` and `crates/server/tests/devices.rs`, whose
//! subject those shapes are.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import jsQR from "jsqr";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  faApple,
  faLinux,
  faWindows,
} from "@fortawesome/free-brands-svg-icons";
import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";

import type {
  DevicesView,
  DiscoveredDevice,
  Nudge,
  RemoteView,
  ServePress,
} from "../src/api/types";
import { listenForNudges } from "../src/nudge";
import { RemoteCard, RemotePane } from "../src/settings/Remote";
import devices from "./fixtures/devices.json" with { type: "json" };
import devicesDiscovered from "./fixtures/devices-discovered.json" with { type: "json" };
import devicesLinked from "./fixtures/devices-linked.json" with { type: "json" };
import devicesWaiting from "./fixtures/devices-waiting.json" with { type: "json" };
import devicesWsl from "./fixtures/devices-wsl.json" with { type: "json" };
import absent from "./fixtures/remote-absent.json" with { type: "json" };
import down from "./fixtures/remote-down.json" with { type: "json" };
import off from "./fixtures/remote-off.json" with { type: "json" };
import unreadableServe from "./fixtures/remote-serve-unreadable.json" with { type: "json" };
import serving from "./fixtures/remote-serving.json" with { type: "json" };
import done from "./fixtures/serve-done.json" with { type: "json" };
import ungranted from "./fixtures/serve-ungranted.json" with { type: "json" };
import { askedFor, json, whenever, serving as stubbing } from "./serving";
import { stream, streaming } from "./streaming";

/// The five machines: no Tailscale at all, one whose daemon is not answering,
/// one that is up and serving the workbench, one that is up and serving
/// nothing, and one that is up and whose serve state could not be read.
const ABSENT = absent as RemoteView;
const DOWN = down as RemoteView;
const SERVING = serving as RemoteView;
const OFF = off as RemoteView;
const UNREADABLE_SERVE = unreadableServe as RemoteView;

/// And the two answers a press comes back with that are not the machine read
/// again — the operator grant, and the reading a press that went through
/// carries.
const DONE = done as ServePress;
const UNGRANTED = ungranted as ServePress;

/// And what this Verkstead is: one device, on a Linux, with nothing linked to
/// it — and the same device on a WSL, which is the case the OS word exists for
/// and the one the row is told apart by.
const DEVICES = devices as DevicesView;
const WSL = devicesWsl as DevicesView;

/// And the same device in a cluster of three: a Mac on a tailnet, and a WSL
/// that answers to the same hostname this one does. Which is the case the whole
/// of cluster mode was written for — the two rows read *workbench*, and the OS
/// word and the mark beside it are the only things that tell them apart.
///
/// The WSL is the one the last dial found nothing at, which is the second way a
/// member's row is drawn: dimmed, reading *unreachable*, with everything about
/// it still on it.
const LINKED = devicesLinked as DevicesView;

/// And the same device with two joins asked for and neither answered: one still
/// inside its ten minutes, and one whose ten minutes ran out. Which are the two
/// ways a pending row is drawn, and neither of them is a member — nothing has
/// been agreed until somebody at the far end presses.
const WAITING = devicesWaiting as DevicesView;


/// And two devices this one has *heard* of and is not linked to at all, which is
/// what the Discovered list draws: a Mac on the LAN with two addresses, and a WSL
/// with one — the mark being the only thing that would tell the second from the
/// Windows it shares a hostname with.
const HEARD = devicesDiscovered as DiscoveredDevice[];

/// The login link the serving machine hands out, which is the address with the
/// key on the end of it.
const LINK =
  "https://workbench.tailnet-name.ts.net/?key=a-stated-workbench-key";

afterEach(() => {
  vi.unstubAllGlobals();
});

function mounting(what: () => JSX.Element) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

/// What this Verkstead is, held for its own path.
///
/// A second read beside the machine's, because the pane makes three: the serve
/// and the key are read off Tailscale, the Devices section is read off the
/// device, and the Discovered list under it off what a browse heard — and none
/// of them waits on another.
function theDevice(listed: DevicesView = DEVICES) {
  return whenever("/api/ui/devices", json(listed));
}

/// And what it has heard of the devices it is not linked to, held for its own
/// path too: the Discovered list is a third read rather than a field of the
/// second, so that a browse hearing something leaves the cluster's own rows
/// alone.
///
/// Nothing heard unless a test says otherwise, which is what a browse that has
/// just started answers however many machines are on the LAN.
function theHeard(heard: Heard = []) {
  return whenever(
    "/api/ui/devices/discovered",
    typeof heard === "function" ? heard : json(heard),
  );
}

/// What a test says the browse heard: a list, or an answer that changes under
/// the page — see [`thenHeard`].
type Heard = DiscoveredDevice[] | ((init?: RequestInit) => Promise<Response>);

/// A browse that heard nothing and then heard something, which is every browse:
/// the first read starts it and the rows arrive after it, so the answer a Nudge
/// asks for is not the answer the pane was drawn from.
function thenHeard(...answers: DiscoveredDevice[][]) {
  let taken = 0;

  return () => json(answers[Math.min(taken++, answers.length - 1)]!)();
}

/// A server answering the three reads this pane makes.
function theMachine(
  told: RemoteView,
  listed: DevicesView = DEVICES,
  heard: Heard = [],
) {
  return stubbing(
    whenever("/api/ui/remote", json(told)),
    theDevice(listed),
    theHeard(heard),
  );
}

function mountCard(told: RemoteView, listed: DevicesView = DEVICES) {
  theMachine(told, listed);
  return mounting(() => <RemoteCard open={false} press={vi.fn()} />);
}

function mountPane(
  told: RemoteView,
  listed: DevicesView = DEVICES,
  heard: Heard = [],
) {
  theMachine(told, listed, heard);
  return mounting(() => <RemotePane back={vi.fn()} />);
}

/// The serve box, which is a plain checkbox and the only one on the pane.
function theBox(): HTMLInputElement {
  return screen.getByRole("checkbox") as HTMLInputElement;
}

/// Where this page was opened, which is what decides whether the box may be
/// unticked. `localhost` is what jsdom answers with left alone, and that is the
/// machine Verkstead runs on.
function openedAt(hostname: string) {
  vi.stubGlobal("location", { hostname });
}

describe("the card", () => {
  /// A machine with nothing installed says so on the card, because that is the
  /// one somebody scanning the page has the most to do about.
  it("says a machine with no Tailscale is only reachable from itself", async () => {
    mountCard(ABSENT);

    await waitFor(() =>
      expect(screen.getByText(/No Tailscale on this machine/)).toBeTruthy(),
    );
  });

  /// And one whose daemon is not answering is its own answer rather than a
  /// serve that is off: nothing about it says whether this workbench would be
  /// served, and a box is not what fixes it.
  it("tells a Tailscale that is not up apart from a serve that is off", async () => {
    mountCard(DOWN);

    await waitFor(() =>
      expect(screen.getByText(/installed here and not up/)).toBeTruthy(),
    );

    expect(screen.queryByText(/not served to it/)).toBeNull();
  });

  /// A machine that is up and not serving names itself, and says the workbench
  /// is not on it.
  it("names the node of a machine that is up and serving nothing", async () => {
    mountCard(OFF);

    await waitFor(() =>
      expect(screen.getByText("workbench.tailnet-name.ts.net")).toBeTruthy(),
    );

    expect(screen.getByText(/not served to it/)).toBeTruthy();
  });

  /// And one that is serving says where, which is the address a phone is
  /// pointed at.
  it("says where a served workbench answers", async () => {
    mountCard(SERVING);

    await waitFor(() =>
      expect(
        screen.getByText("https://workbench.tailnet-name.ts.net"),
      ).toBeTruthy(),
    );
  });
});

describe("the pane", () => {
  /// The install pointer, which is the one thing this section has to say that
  /// nothing on the page can do — so it is a link out of Verkstead.
  it("points a machine with no Tailscale at where to get one", async () => {
    mountPane(ABSENT);

    const pointer = await waitFor(() => screen.getByText("Install Tailscale"));

    expect(pointer.getAttribute("href")).toBe("https://tailscale.com/download");
  });

  /// What the machine printed, shown as it came: the line names the service to
  /// start, and a word of it reworded is a command that does not work.
  it("shows what a Tailscale that is not up said, in its own words", async () => {
    mountPane(DOWN);

    await waitFor(() =>
      expect(screen.getByText(/sudo systemctl start tailscaled/)).toBeTruthy(),
    );

    // And what to do about it, which is the one command that is the human's.
    expect(screen.getByText("tailscale up")).toBeTruthy();
  });

  /// What a serving machine's pane is, whole: the box with the one line that is
  /// its own, the code and the link, and the key with its own one line.
  ///
  /// The other half of the claim is what is *not* there. The readings of node
  /// and served address, the two subheadings, and every note about what a
  /// tailnet is and what a login link is worth have gone: a pane read on a phone
  /// is the controls and nothing around them.
  it("is the box, its line, the way in and the key, and nothing else", async () => {
    mountPane(SERVING);

    await waitFor(() => expect(theBox().checked).toBe(true));

    expect(screen.getByText("Allow remote access via Tailscale")).toBeTruthy();
    expect(
      screen.getByText(
        "Allows secure remote access from other machines over the internet.",
      ),
    ).toBeTruthy();

    expect(
      screen.getByRole("img", { name: "The login link for this workbench" }),
    ).toBeTruthy();
    expect(screen.getByText(LINK)).toBeTruthy();
    expect(screen.getByText("Copy")).toBeTruthy();

    expect(screen.getByText("Reset key")).toBeTruthy();
    expect(
      screen.getByText(
        "Resets the secret token used to access the UI. This will " +
          "disconnect all other devices.",
      ),
    ).toBeTruthy();

    // The readings and the headings that stood over them.
    expect(screen.queryByText("This machine on the tailnet")).toBeNull();
    expect(screen.queryByText("The workbench is served at")).toBeNull();
    expect(screen.queryByRole("heading", { level: 3 })).toBeNull();

    // And the sentence that summed the machine up, which is the card's alone.
    expect(screen.queryByText(/Served to the tailnet at/)).toBeNull();
  });

  /// And one that is up with nothing served has no address — there is none
  /// until something is served, so there is nothing to scan and nothing to
  /// copy.
  it("gives a machine serving nothing no address", async () => {
    mountPane(OFF);

    await waitFor(() => expect(theBox().checked).toBe(false));

    expect(screen.queryByText(/^https:\/\//)).toBeNull();
  });
});

describe("the serve checkbox", () => {
  /// A server answering the read and the presses both.
  ///
  /// The read is held for its path, because the pane makes it whenever it likes
  /// and a test about a press should not have to say when. The presses go in the
  /// order given, which is what a re-try is made of: a refusal, and then the same
  /// press again once the grant has been run.
  function theMachinePressed(told: RemoteView, ...answers: Array<ServePress>) {
    return stubbing(
      whenever("/api/ui/remote", json(told)),
      theDevice(),
      theHeard(),
      ...answers.map((answer) => json(answer)),
    );
  }

  /// What the page put on the wire for the press it made.
  function pressed(fetching: ReturnType<typeof stubbing>): unknown {
    const put = fetching.mock.calls.find(
      ([path, init]) =>
        String(path) === "/api/ui/remote/serve" && init?.method === "POST",
    );

    expect(put, "expected the page to have pressed").toBeTruthy();
    return JSON.parse(String(put![1]?.body));
  }

  /// The position is the reading rather than anything the page remembers, which
  /// is what makes a serve set up in a terminal one this box unticks.
  it("stands where the machine reads, not where it was last pressed", async () => {
    mountPane(SERVING);

    await waitFor(() => expect(theBox().checked).toBe(true));
  });

  it("stands off on a machine serving nothing", async () => {
    mountPane(OFF);

    await waitFor(() => expect(theBox().checked).toBe(false));
  });

  /// And a serve state this build could not read gets no box at all: *cannot
  /// tell* under a control offering to turn *off* on would be the one sentence
  /// this section must never say. What Tailscale said stands in its place.
  it("is not drawn where the serve state could not be read", async () => {
    mountPane(UNREADABLE_SERVE);

    await waitFor(() =>
      expect(screen.getByText(/could not be read/)).toBeTruthy(),
    );

    expect(
      screen.getByText("the serve configuration named no Web section"),
    ).toBeTruthy();
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  /// A press says which way it is going, and the link it comes back with is
  /// what the pane then draws — the answer *is* the read, so nothing asks
  /// again.
  it("presses the serve on and draws the link off the answer", async () => {
    const fetching = theMachinePressed(OFF, DONE);
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theBox().checked).toBe(false));
    fireEvent.click(theBox());

    await waitFor(() => expect(screen.getByText(LINK)).toBeTruthy());

    expect(pressed(fetching)).toEqual({ on: true });
    expect(theBox().checked).toBe(true);
  });

  /// And off the other way, which is the same press with the other value: the
  /// box unticks and the way in goes with the serve that made it.
  it("presses the serve off", async () => {
    const fetching = theMachinePressed(SERVING, {
      press: "Done",
      reading: OFF,
    });
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theBox().checked).toBe(true));
    fireEvent.click(theBox());

    await waitFor(() => expect(theBox().checked).toBe(false));

    expect(pressed(fetching)).toEqual({ on: false });
    expect(screen.queryByText(LINK)).toBeNull();
  });

  /// A serve Tailscale would not take draws the line that makes it take one —
  /// exactly as it is to be typed, because a word of it reworded is a command
  /// that does not work.
  it("draws the operator grant when a serve is refused", async () => {
    theMachinePressed(OFF, UNGRANTED);
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theBox().checked).toBe(false));
    fireEvent.click(theBox());

    await waitFor(() =>
      expect(
        screen.getByText("sudo tailscale set --operator=ada"),
      ).toBeTruthy(),
    );

    // With what Tailscale said under it, because the grant is this build's
    // reading of a refusal and the refusal is the machine's own.
    expect(screen.getByText("Access denied: serve config denied")).toBeTruthy();

    // And nothing moved: the box is still where the machine reads.
    expect(theBox().checked).toBe(false);
  });

  /// The press after the grant has been run is the same press again, and it
  /// serves — which is the whole of what a re-try is here.
  it("serves on the press after the grant", async () => {
    theMachinePressed(OFF, UNGRANTED, DONE);
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theBox().checked).toBe(false));
    fireEvent.click(theBox());

    await waitFor(() =>
      expect(
        screen.getByText("sudo tailscale set --operator=ada"),
      ).toBeTruthy(),
    );

    fireEvent.click(theBox());

    await waitFor(() => expect(theBox().checked).toBe(true));

    // And the grant goes with the refusal it belonged to: there is nothing left
    // for anybody to run.
    expect(screen.queryByText("sudo tailscale set --operator=ada")).toBeNull();
  });

  /// The page that arrived over the serve cannot take it away: unticking from
  /// there would cut the connection carrying the press, and what would be left
  /// is a phone on a tailnet with nothing answering on it.
  ///
  /// Settled in the client, off the address the reading already carries: the
  /// server sees the tailnet and the loopback arrive on one port and could not
  /// tell them apart.
  it("cannot be unticked from a page opened over the served address", async () => {
    openedAt("workbench.tailnet-name.ts.net");
    const fetching = theMachine(SERVING);
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theBox().checked).toBe(true));
    expect(theBox().disabled).toBe(true);

    // With the reason on it, because a box that will not be pressed and says
    // nothing about why is one somebody goes looking for the bug in.
    expect(theBox().closest("label")?.getAttribute("title")).toMatch(
      /opened over the tailnet/,
    );

    fireEvent.click(theBox());

    expect(
      fetching.mock.calls.some(
        ([path]) => String(path) === "/api/ui/remote/serve",
      ),
    ).toBe(false);
    expect(theBox().checked).toBe(true);
  });

  /// And the same machine read from the machine itself unticks as it always
  /// did: nothing is being cut off, and the desktop app is this case too.
  it("unticks from a page opened on localhost", async () => {
    const fetching = theMachinePressed(SERVING, {
      press: "Done",
      reading: OFF,
    });
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theBox().checked).toBe(true));
    expect(theBox().disabled).toBe(false);
    expect(theBox().closest("label")?.getAttribute("title")).toBeNull();

    fireEvent.click(theBox());

    await waitFor(() => expect(theBox().checked).toBe(false));
    expect(pressed(fetching)).toEqual({ on: false });
  });
});

describe("the login link", () => {
  /// The address by itself lets nobody in — the workbench answers 401 without
  /// the key — so what the code carries is the link, which is the address with
  /// the key on it.
  it("draws the login link as a code and beside it as text", async () => {
    mountPane(SERVING);

    const code = await waitFor(() =>
      screen.getByRole("img", { name: "The login link for this workbench" }),
    );

    // Drawn here rather than fetched: a workbench standing behind a secret has
    // no business asking a third party to render it, and an install on a
    // tailnet may have nowhere to fetch from. So it is an inline SVG with the
    // modules in it, and nothing on the wire.
    expect(code.tagName.toLowerCase()).toBe("svg");
    expect(code.querySelector("path")?.getAttribute("d")).toMatch(/^M\d/);
    expect(code.querySelector("image")).toBeNull();

    expect(screen.getByText(LINK)).toBeTruthy();
  });

  /// And what a phone reading it gets is that link, which is the whole of the
  /// claim: scanned, it opens the workbench and the key on the end of it is
  /// what the handshake lets the phone in with.
  ///
  /// Read back the way a camera reads it rather than compared against the
  /// encoder, because what is being asked about is the drawing: a grid written
  /// out transposed or mirrored encodes perfectly and scans as nothing.
  it("reads back as the login link when it is scanned", async () => {
    mountPane(SERVING);

    const code = await waitFor(() =>
      screen.getByRole("img", { name: "The login link for this workbench" }),
    );

    expect(scanned(code)).toBe(LINK);
  });

  /// And the link beside it copies, for every way in that is not a camera: a
  /// laptop on the same tailnet, a link pasted into a note.
  it("copies the link", async () => {
    const written = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText: written } });

    mountPane(SERVING);

    fireEvent.click(await waitFor(() => screen.getByText("Copy")));

    await waitFor(() => expect(screen.getByText("Copied")).toBeTruthy());

    expect(written).toHaveBeenCalledWith(LINK);
  });

  /// A machine serving nothing has no address to be let in at, so there is
  /// nothing to point a camera at — and the key is still there to be taken
  /// back, because it is not Tailscale's.
  it("draws nothing to scan on a machine serving nothing", async () => {
    mountPane(OFF);

    await waitFor(() => expect(theBox().checked).toBe(false));

    // Named rather than counted, because the pane draws another image: the mark
    // for this device's operating system, which stands whatever Tailscale is
    // doing.
    expect(
      screen.queryByRole("img", { name: "The login link for this workbench" }),
    ).toBeNull();
    expect(screen.getByText("Reset key")).toBeTruthy();
  });

  /// And so has a machine with no Tailscale on it, one whose daemon is not
  /// answering, and one whose serve could not be read.
  ///
  /// The key gates every install, and the daemon prints it in the startup line
  /// wherever it is running — so a key that has gone somewhere it should not
  /// have has to be re-issuable whatever the tailnet is doing. Turning the
  /// serve off is one of the moments somebody would want it back, and a Reset
  /// that went away with the address would be the press removing itself.
  it("keeps Reset key on every state of the pane", async () => {
    for (const [named, told] of [
      ["with no Tailscale", ABSENT],
      ["with the daemon down", DOWN],
      ["whose serve could not be read", UNREADABLE_SERVE],
    ] as const) {
      const { unmount } = mountPane(told);

      expect(
        await waitFor(() => screen.getByText("Reset key")),
        `the ${named} machine`,
      ).toBeTruthy();

      unmount();
    }
  });

  /// **Reset key** re-issues the secret, and the QR and the link redraw on the
  /// new one — out of the press's own answer, which is the machine read again.
  it("redraws the code and the link on a key that was reset", async () => {
    const fresh = {
      ...SERVING,
      link: "https://workbench.tailnet-name.ts.net/?key=the-key-it-was-reset-to",
    } as RemoteView;

    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(),
      theHeard(),
      whenever("/api/ui/remote/key", json(fresh), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    const drawn = () =>
      screen
        .getByRole("img", { name: "The login link for this workbench" })
        .querySelector("path")
        ?.getAttribute("d");

    await waitFor(() => expect(screen.getByText(LINK)).toBeTruthy());
    const before = drawn();

    fireEvent.click(screen.getByText("Reset key"));

    await waitFor(() =>
      expect(
        screen.getByText(
          "https://workbench.tailnet-name.ts.net/?key=the-key-it-was-reset-to",
        ),
      ).toBeTruthy(),
    );

    // The code with it, because the code is the link: one that redrew only the
    // text would leave a camera being pointed at the key that was just taken
    // away.
    expect(drawn()).not.toBe(before);

    // The old one is gone rather than standing beside the new one.
    expect(screen.queryByText(LINK)).toBeNull();

    // And the answer is the read, so nothing asked again.
    expect(askedFor(fetching, "/api/ui/remote")).toBe(1);
  });

  /// A reset the server would not make is said where it was pressed, and
  /// nothing on the page moves: the key that was there is still the key.
  it("says so when a reset was refused", async () => {
    stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(),
      theHeard(),
      whenever(
        "/api/ui/remote/key",
        () =>
          Promise.resolve(
            new Response(
              JSON.stringify({
                error: "the workbench key could not be re-issued",
              }),
              { status: 500, headers: { "content-type": "application/json" } },
            ),
          ),
        "POST",
      ),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    fireEvent.click(await waitFor(() => screen.getByText("Reset key")));

    await waitFor(() =>
      expect(screen.getByText(/could not be re-issued/)).toBeTruthy(),
    );

    // And nothing on the page moved: the key that was there is still the key,
    // and the code above it still opens the workbench.
    expect(screen.getByText(LINK)).toBeTruthy();
  });
});

describe("the devices section", () => {
  /// The mark standing beside a device's name, found by the word it is labelled
  /// with — which is the OS word itself, so a screen reader hears what the
  /// drawing says.
  function theMark(os: string): Element {
    return screen.getByRole("img", { name: os });
  }

  /// And which icon that mark turned out to be, compared against the shape
  /// rather than against a class name: the icons are drawn as the paths Font
  /// Awesome ships, so the path *is* the identity of the drawing.
  function drawnAs(icon: IconDefinition): string {
    return [icon.icon[4]].flat().filter((path) => typeof path === "string")[0]!;
  }

  /// The one row the list holds, whole: the mark, the name, *this device* and
  /// the addresses a peer could reach it on.
  it("holds this device, with its mark, its name and its addresses", async () => {
    mountPane(SERVING);

    await waitFor(() => expect(screen.getByText("workbench")).toBeTruthy());

    expect(theMark("Linux")).toBeTruthy();
    expect(screen.getByText("this device")).toBeTruthy();
    expect(screen.getByText("192.168.1.24, 10.0.0.7")).toBeTruthy();
  });

  /// And nothing to press on it. There is nothing yet to unlink this device
  /// from, and a device could not be unlinked from itself in any case — *this
  /// device* is what stands where another row will carry the press.
  it("offers no Unlink", async () => {
    mountPane(SERVING);

    await waitFor(() => expect(screen.getByText("workbench")).toBeTruthy());

    expect(screen.queryByText("Unlink")).toBeNull();
  });

  /// A device linked to this one is a row beside it, drawn the same way: the
  /// mark for its OS, the name it is shown under, and the addresses a peer
  /// could reach it on.
  it("draws a row for every device linked to this one", async () => {
    mountPane(SERVING, LINKED);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    expect(theMark("macOS")).toBeTruthy();
    expect(
      screen.getByText("laptop.tailnet-name.ts.net, 100.64.0.2"),
    ).toBeTruthy();

    // And the third row, which answers to the same hostname this device does:
    // a Windows machine and the WSL on it share one, and the OS word is what
    // tells the two apart.
    expect(theMark("Linux (WSL)")).toBeTruthy();
    expect(screen.getByText("172.29.0.14")).toBeTruthy();

    expect(
      screen.getAllByText("workbench").length,
      "this device and the WSL linked to it both answer to that hostname",
    ).toBe(2);
  });

  /// And only one of them is this machine, whichever else is on the list.
  it("marks this device and no other", async () => {
    mountPane(SERVING, LINKED);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    expect(screen.getAllByText("this device").length).toBe(1);
  });

  /// And a press on every member's row and on no other: two members on the
  /// fixture, two Unlinks, with this device's own row carrying none.
  it("offers an Unlink on every member's row and on no other", async () => {
    mountPane(SERVING, LINKED);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    const presses = screen.getAllByText("Unlink");

    expect(presses.length, "one apiece for the two members").toBe(2);
    expect(
      presses.some((press) =>
        press.closest("li")?.textContent?.includes("this device"),
      ),
      "and none of them on the row that is this machine",
    ).toBe(false);
  });

  /// A member the last dial found nothing at reads *unreachable*, and only that
  /// one does: the fixture's Mac is answering and its WSL is not.
  it("says which member is not answering", async () => {
    mountPane(SERVING, LINKED);

    const away = await waitFor(() => screen.getByText("unreachable"));

    expect(
      away.closest("li")?.textContent,
      "the word is on the row of the member that is not answering, which is the \
one on the LAN",
    ).toContain("172.29.0.14");
    expect(
      screen.getAllByText("unreachable").length,
      "and the member that is answering says nothing of the kind",
    ).toBe(1);
  });

  /// And it is still the whole row: the mark, the name, the addresses and the
  /// press, because none of them has stopped being true — a machine that is
  /// never coming back is most of what an Unlink is for, so the one row
  /// somebody needs the press on is the one row it must not be missing from.
  it("leaves everything on an unreachable member's row", async () => {
    mountPane(SERVING, LINKED);

    const away = await waitFor(() => screen.getByText("unreachable"));

    expect(theMark("Linux (WSL)")).toBeTruthy();
    expect(screen.getByText("172.29.0.14")).toBeTruthy();

    const press = away.closest("li")?.querySelector("button");

    expect(press?.textContent, "and the Unlink is drawn on it").toBe("Unlink");
    expect(press?.disabled, "and works exactly as any other's").toBe(false);
  });

  /// The case the whole of cluster mode was written for: a Windows machine and
  /// the WSL on it share a hostname, so the word and the mark together are what
  /// tell the two rows apart — the Linux mark, and *Linux (WSL)* beside it.
  it("draws a WSL with the Linux mark and Linux (WSL) beside it", async () => {
    mountPane(SERVING, WSL);

    const mark = await waitFor(() => theMark("Linux (WSL)"));

    expect(mark.querySelector("path")?.getAttribute("d")).toBe(
      drawnAs(faLinux),
    );
  });

  /// And the other two platforms wear their own marks, off the same word.
  it("draws a Mac and a Windows with their own marks", async () => {
    for (const [os, icon] of [
      ["macOS", faApple],
      ["Windows", faWindows],
    ] as const) {
      const { unmount } = mountPane(SERVING, {
        ...DEVICES,
        this: { ...DEVICES.this, os },
      });

      const mark = await waitFor(() => theMark(os));

      expect(mark.querySelector("path")?.getAttribute("d"), os).toBe(
        drawnAs(icon),
      );

      unmount();
    }
  });

  /// The section stands on a machine with no Tailscale at all, which is the
  /// whole reason it is outside the choice the serve and the code are inside: a
  /// device identity is not a tailnet's, and a list that vanished on such a
  /// machine would be a cluster feature that appeared to need one.
  it("draws on every state of the pane, Tailscale or none", async () => {
    for (const [named, told] of [
      ["with no Tailscale", ABSENT],
      ["with the daemon down", DOWN],
      ["whose serve could not be read", UNREADABLE_SERVE],
      ["serving nothing", OFF],
    ] as const) {
      const { unmount } = mountPane(told);

      expect(
        await waitFor(() => screen.getByText("this device")),
        `the machine ${named}`,
      ).toBeTruthy();

      unmount();
    }
  });

  /// And it reads its own answer rather than anything of the settings query
  /// the rest of the page shares — for the reason the two sections beside it
  /// read nothing of it either: a device is not configured, so there is nothing
  /// of it in either settings file to read.
  it("reads the device rather than the settings", async () => {
    const fetching = theMachine(SERVING);
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("this device")).toBeTruthy());

    expect(askedFor(fetching, "/api/ui/devices")).toBeGreaterThan(0);
    expect(
      fetching.mock.calls.filter(([path]) =>
        String(path).startsWith("/api/ui/settings"),
      ),
    ).toEqual([]);
  });
});

describe("the discovered list", () => {
  /// The row of the device named, which is the `li` its name is drawn in: what a
  /// test about one row asks is what that row says, and the list beside it is
  /// somebody else's row.
  function theRow(name: string): Element {
    const row = screen.getAllByText(name).at(0)?.closest("li");

    if (!row) {
      throw new Error(`no row for ${name}`);
    }

    return row;
  }

  /// A device heard of is a row: the mark for its OS, the name it advertised,
  /// where it was found and that the LAN is where — and a press that needs
  /// nothing typed.
  it("draws a row for every device it has heard of", async () => {
    mountPane(SERVING, DEVICES, HEARD);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    const heard = theRow("laptop");

    expect(
      heard.textContent,
      "where it was found, with the port that device's listener landed on — which \
is the only thing that tells two Verksteads on one machine apart",
    ).toContain("192.168.1.31:8423, 10.0.0.31:8423");
    expect(heard.textContent, "and that the LAN is where").toContain("LAN");
    expect(
      heard.querySelector("button")?.textContent,
      "with one press to link it, and nothing to type",
    ).toBe("Add");

    // And the second, which is the case the whole of cluster mode was written
    // for: it answers to the same hostname this device does, and the mark beside
    // the name is what tells the two apart.
    expect(screen.getByRole("img", { name: "Linux (WSL)" })).toBeTruthy();
    expect(theRow("172.29.0.14:9423").textContent).toContain("LAN");
  });

  /// And a press apiece, on every discovered row and on none of the cluster's
  /// own: a member is linked already, and this device is not linked to itself.
  ///
  /// Counted among the rows rather than on the page, because the box below the
  /// list says Add as well — one press per row and one for the address somebody
  /// types, which is two ways of asking the same question.
  it("offers Add on every discovered row and on no other", async () => {
    mountPane(SERVING, LINKED, HEARD);

    await waitFor(() =>
      expect(screen.getByText("192.168.1.31:8423, 10.0.0.31:8423")).toBeTruthy(),
    );

    expect(
      screen.getAllByText("Add").filter((press) => press.closest("li")).length,
      "one apiece for the two devices heard of; the members carry Unlink instead",
    ).toBe(2);
    expect(
      screen.getAllByText("Unlink").length,
      "and the two members' rows are untouched by any of it",
    ).toBe(2);
  });

  /// Nothing heard yet reads as a browse that has just started rather than as a
  /// network with nothing on it.
  ///
  /// **Which is what every first read of this list is.** The browse starts when
  /// the list is first asked for, so the answer the pane is drawn from is empty
  /// however many machines are out there — a line saying none was found would be
  /// wrong for the first second of every visit to this pane.
  it("says devices appear as they are heard rather than that none was found", async () => {
    mountPane(SERVING);

    await waitFor(() => expect(screen.getByText("Discovered")).toBeTruthy());

    expect(
      screen.getByText(/appear here as they are heard/),
      "and the typed box below is what is left for the ones a browse cannot reach",
    ).toBeTruthy();
  });

  /// The whole of how a device arrives: a browse that has heard one machine, a
  /// Nudge down the stream, and the second row appearing with nothing reloaded.
  ///
  /// **Which is what a browse is rather than a way of testing one.** It finds
  /// things after the fact, so the answer the pane was drawn from is never the
  /// last word — and the row already on the page stays exactly where it was while
  /// the new one arrives beside it.
  ///
  /// **And the cluster's own rows are not re-read for it**, which is why the two
  /// lists are two readings: a browse hears something every few seconds, and a
  /// membership re-read each time would be the rows the human is looking at
  /// replaced by whatever the LAN happened to say.
  it("draws a device the browse found without re-reading the cluster", async () => {
    const fetching = theMachine(SERVING, DEVICES, thenHeard([HEARD[1]!], HEARD));
    streaming();

    const queries = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

    render(() => (
      <QueryClientProvider client={queries}>
        <RemotePane back={vi.fn()} />
      </QueryClientProvider>
    ));

    const listening = listenForNudges(queries);
    stream().opens();

    // The one device this browse had heard when the pane was drawn, and the read
    // of the membership beside it.
    await waitFor(() =>
      expect(screen.getByText("172.29.0.14:9423")).toBeTruthy(),
    );

    const read = askedFor(fetching, "/api/ui/devices");

    stream().nudges({ kind: "discovered" } satisfies Nudge);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    expect(
      screen.getByText("172.29.0.14:9423"),
      "and the row that was already drawn is still the same row",
    ).toBeTruthy();
    expect(
      askedFor(fetching, "/api/ui/devices"),
      "while the membership is not read again: nothing about the cluster changed, \
and the rows drawn of it are not the browse's to move",
    ).toBe(read);

    listening();
  });
});

describe("adding a device", () => {
  /// The box and the press, which are the one thing on this pane that is
  /// configured rather than read.
  function theAddress(): HTMLInputElement {
    return screen.getByLabelText("Link another device") as HTMLInputElement;
  }

  /// The box stands on every state of the pane, for the reason the list does:
  /// a machine that has never heard of a tailnet has a device of its own, and
  /// linking two of them is not Tailscale's.
  it("offers Add on every state of the pane", async () => {
    for (const [named, told] of [
      ["with no Tailscale", ABSENT],
      ["with the daemon down", DOWN],
      ["serving nothing", OFF],
    ] as const) {
      const { unmount } = mountPane(told);

      expect(
        await waitFor(() => theAddress()),
        `the machine ${named}`,
      ).toBeTruthy();

      unmount();
    }
  });

  /// And it will not be pressed with nothing typed in it: an empty address is
  /// nowhere to knock.
  it("will not be pressed on an empty address", async () => {
    mountPane(SERVING);

    const press = await waitFor(
      () => screen.getByText("Add") as HTMLButtonElement,
    );

    expect(press.disabled).toBe(true);

    fireEvent.input(theAddress(), { target: { value: "192.168.1.31" } });

    expect(press.disabled).toBe(false);
  });

  /// A press sends the address as typed, and the section redraws out of the
  /// answer rather than out of a second read: the pending row the press left
  /// behind is in what came back.
  it("sends the address and redraws on what came back", async () => {
    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(),
      theHeard(),
      whenever("/api/ui/devices/joins", json(WAITING), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theAddress()).toBeTruthy());

    fireEvent.input(theAddress(), {
      target: { value: "laptop.tailnet-name.ts.net" },
    });
    fireEvent.click(screen.getByText("Add"));

    await waitFor(() =>
      expect(
        screen.getByText("Waiting for confirmation on laptop."),
      ).toBeTruthy(),
    );

    const [, sent] = fetching.mock.calls.find(
      ([path, init]) =>
        String(path) === "/api/ui/devices/joins" && init?.method === "POST",
    )!;

    expect(JSON.parse(String(sent?.body))).toEqual({
      address: "laptop.tailnet-name.ts.net",
    });
  });

  /// And the box empties on a press that went through, so the next address is
  /// typed into an empty one rather than over the last.
  it("empties the box on a press that went through", async () => {
    stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(),
      theHeard(),
      whenever("/api/ui/devices/joins", json(WAITING), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theAddress()).toBeTruthy());

    fireEvent.input(theAddress(), { target: { value: "192.168.1.31" } });
    fireEvent.click(screen.getByText("Add"));

    await waitFor(() => expect(theAddress().value).toBe(""));
  });

  /// A press that did not get through says what the far end said, in the far
  /// end's own words: a machine that is off, an address nobody is at and a
  /// Verkstead that would not have it are three different things to do
  /// something about.
  it("says what went wrong in the words it came back in", async () => {
    stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(),
      theHeard(),
      whenever(
        "/api/ui/devices/joins",
        json({ error: "192.168.1.31 answered nothing" }, 502),
        "POST",
      ),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(theAddress()).toBeTruthy());

    fireEvent.input(theAddress(), { target: { value: "192.168.1.31" } });
    fireEvent.click(screen.getByText("Add"));

    await waitFor(() =>
      expect(screen.getByText(/192\.168\.1\.31 answered nothing/)).toBeTruthy(),
    );
  });
});

describe("a join waiting to be confirmed", () => {
  /// The row says which device is being waited on and where this one knocked —
  /// and it is not a member, because nothing has been agreed.
  it("says which device is being waited on", async () => {
    mountPane(SERVING, WAITING);

    await waitFor(() =>
      expect(
        screen.getByText("Waiting for confirmation on laptop."),
      ).toBeTruthy(),
    );

    expect(screen.getByText("laptop.tailnet-name.ts.net")).toBeTruthy();
  });

  /// And it counts for nothing on the card: a request is not a membership until
  /// somebody at the far end presses, so a Verkstead waiting on two of them is
  /// still a Verkstead nothing is linked to.
  it("counts for nothing on the card", async () => {
    mountCard(SERVING, WAITING);

    await waitFor(() =>
      expect(screen.getByText(/No other devices are linked\./)).toBeTruthy(),
    );
  });

  /// And it draws **this device's** fingerprint, which is the whole point of
  /// drawing one: the modal on the other machine shows the same string, and the
  /// pair exists for two people at two screens to compare by eye.
  it("draws this device's own fingerprint", async () => {
    mountPane(SERVING, WAITING);

    const drawn = await waitFor(() =>
      screen.getByText(WAITING.this.fingerprint),
    );

    expect(
      drawn.closest("li")?.textContent,
      "on the row that is waiting rather than on this device's own",
    ).toContain("Waiting for confirmation on laptop.");
  });

  /// A request whose ten minutes ran out says so rather than going on reading
  /// *waiting* for ever, and offers Dismiss where the live one offers Cancel.
  it("says when a request expired, and offers Dismiss", async () => {
    mountPane(SERVING, WAITING);

    await waitFor(() =>
      expect(screen.getByText("The request to desk expired.")).toBeTruthy(),
    );

    expect(screen.getByText("Cancel")).toBeTruthy();
    expect(screen.getAllByText("Dismiss").length).toBe(2);
  });

  /// And one the far end came back and refused says *that* rather than reading
  /// expired: a human said no, which is a different thing from nobody being
  /// there, and the row is owed the difference.
  it("says when the far end refused, and offers Dismiss", async () => {
    mountPane(SERVING, WAITING);

    const said = await waitFor(() =>
      screen.getByText("studio refused the request."),
    );

    expect(
      said.closest("li")?.textContent,
      "a refused row is over, so it carries Dismiss rather than Cancel",
    ).toContain("Dismiss");

    expect(
      said.closest("li")?.textContent,
      "and nobody is comparing a fingerprint on a request that is over",
    ).not.toContain(WAITING.this.fingerprint);
  });

  /// And Cancel takes it back, with the section redrawn out of the press's own
  /// answer.
  it("takes a request back on Cancel", async () => {
    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(WAITING),
      theHeard(),
      whenever(
        "/api/ui/devices/joins/1122334455667788/cancel",
        json(DEVICES),
        "POST",
      ),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() =>
      expect(
        screen.getByText("Waiting for confirmation on laptop."),
      ).toBeTruthy(),
    );

    fireEvent.click(screen.getByText("Cancel"));

    await waitFor(() =>
      expect(
        screen.queryByText("Waiting for confirmation on laptop."),
      ).toBeNull(),
    );

    expect(
      askedFor(fetching, "/api/ui/devices/joins/1122334455667788/cancel"),
    ).toBe(1);
  });
});

/// **Unlink**: a device taken out of the cluster for everybody, asked once
/// before it happens.
///
/// The second of the two departures this pane makes from *nothing is confirmed
/// twice* — Add is the first — and it departs for the reason Remove on a Repo
/// does: it cannot be taken back. So what is asked here is the asking as much
/// as the press: that nothing goes out until the card has been answered, that
/// the card names the device rather than saying *this one*, and that every way
/// out of it but the one button leaves the cluster alone.
describe("unlinking a device", () => {
  /// The member the fixture's presses are about: the Mac, which is the first
  /// of its two members and the one that is answering.
  const LAPTOP = LINKED.members[0]!.identity.device;

  /// What the section reads once that one has gone, which is what the press
  /// answers with — the list the server read again, already a row shorter.
  const AFTER: DevicesView = {
    ...LINKED,
    members: LINKED.members.slice(1),
  };

  /// The press on a row asks rather than acts, and the card names the device it
  /// would take away: the list it was pressed in is behind it, so a card
  /// reading *Unlink this device?* would be asking about whichever row the
  /// human remembers pressing.
  it("asks before anything goes out, naming the device", async () => {
    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(LINKED),
      theHeard(),
      whenever(`/api/ui/devices/members/${LAPTOP}/unlink`, json(AFTER), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    fireEvent.click(screen.getAllByText("Unlink")[0]!);

    await waitFor(() =>
      expect(screen.getByText("Unlink laptop?")).toBeTruthy(),
    );

    expect(
      askedFor(fetching, `/api/ui/devices/members/${LAPTOP}/unlink`),
      "and nothing has gone out while the card is up",
    ).toBe(0);
  });

  /// And the card says what the press really does, which is the one thing about
  /// it somebody could be wrong about: it is not this device's own half of a
  /// link being cut.
  it("says that the device leaves the cluster for everybody", async () => {
    mountPane(SERVING, LINKED);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    fireEvent.click(screen.getAllByText("Unlink")[0]!);

    const said = await waitFor(() =>
      screen.getByText(/leaves the cluster for every device in it/),
    );

    expect(said.textContent).toContain("told to forget the rest");
  });

  /// The press on the card is what acts, and the section is redrawn out of its
  /// own answer rather than out of a second request.
  it("unlinks the device the card named, and redraws the list", async () => {
    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(LINKED),
      theHeard(),
      whenever(`/api/ui/devices/members/${LAPTOP}/unlink`, json(AFTER), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    fireEvent.click(screen.getAllByText("Unlink")[0]!);

    await waitFor(() =>
      expect(screen.getByText("Unlink laptop?")).toBeTruthy(),
    );

    fireEvent.click(
      screen.getAllByText("Unlink").find((press) => press.closest("dialog"))!,
    );

    await waitFor(() => expect(screen.queryByText("laptop")).toBeNull());

    expect(askedFor(fetching, `/api/ui/devices/members/${LAPTOP}/unlink`)).toBe(
      1,
    );
  });

  /// And the way back leaves the cluster alone, which is the whole of what
  /// asking is for.
  it("leaves the cluster alone on Keep it", async () => {
    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(LINKED),
      theHeard(),
      whenever(`/api/ui/devices/members/${LAPTOP}/unlink`, json(AFTER), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    fireEvent.click(screen.getAllByText("Unlink")[0]!);

    await waitFor(() =>
      expect(screen.getByText("Unlink laptop?")).toBeTruthy(),
    );

    fireEvent.click(screen.getByText("Keep it"));

    await waitFor(() =>
      expect(screen.queryByText("Unlink laptop?")).toBeNull(),
    );

    expect(screen.getByText("laptop"), "the row is where it was").toBeTruthy();
    expect(askedFor(fetching, `/api/ui/devices/members/${LAPTOP}/unlink`)).toBe(
      0,
    );
  });

  /// And after the last member goes, the list is this device's row alone and
  /// the section reads as it did before anything was linked.
  it("leaves this device alone on the list once the last member goes", async () => {
    const last = LINKED.members[1]!.identity.device;

    const fetching = stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice({ ...LINKED, members: LINKED.members.slice(1) }),
      theHeard(),
      whenever(`/api/ui/devices/members/${last}/unlink`, json(DEVICES), "POST"),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("unreachable")).toBeTruthy());

    fireEvent.click(screen.getAllByText("Unlink")[0]!);
    fireEvent.click(
      await waitFor(() =>
        screen.getAllByText("Unlink").find((press) => press.closest("dialog"))!,
      ),
    );

    await waitFor(() => expect(screen.queryByText("unreachable")).toBeNull());

    expect(screen.getByText("this device")).toBeTruthy();
    expect(
      screen.queryByText("Unlink"),
      "and nothing left to press, which is the list before anything was linked",
    ).toBeNull();
    expect(askedFor(fetching, `/api/ui/devices/members/${last}/unlink`)).toBe(
      1,
    );
  });

  /// A server that could not answer at all is said under the list, in its own
  /// words: what the human can do about a workbench that is down is different
  /// from what they can do about anything else.
  it("says what went wrong where the press could not be made", async () => {
    stubbing(
      whenever("/api/ui/remote", json(SERVING)),
      theDevice(LINKED),
      theHeard(),
      whenever(
        `/api/ui/devices/members/${LAPTOP}/unlink`,
        json(
          { error: "this server holds no device identity to answer for" },
          503,
        ),
        "POST",
      ),
    );
    mounting(() => <RemotePane back={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("laptop")).toBeTruthy());

    fireEvent.click(screen.getAllByText("Unlink")[0]!);
    fireEvent.click(
      await waitFor(() =>
        screen.getAllByText("Unlink").find((press) => press.closest("dialog"))!,
      ),
    );

    await waitFor(() =>
      expect(screen.getByText(/The device could not be unlinked/)).toBeTruthy(),
    );

    expect(screen.getByText("laptop"), "and the row stays drawn").toBeTruthy();
  });
});

describe("the devices clause on the card", () => {
  /// It reads right after every one of the six sentences the card says about
  /// Tailscale, rather than being written into one of them: what it says is
  /// true of a machine in any of those states.
  it("follows whatever the card said about Tailscale", async () => {
    for (const [named, told] of [
      ["with no Tailscale", ABSENT],
      ["with the daemon down", DOWN],
      ["whose Tailscale could not be read", UNREADABLE_SERVE],
      ["serving nothing", OFF],
      ["serving the workbench", SERVING],
    ] as const) {
      const { unmount } = mountCard(told);

      const standing = await waitFor(() =>
        screen.getByText(/No other devices are linked\./),
      );

      // In the one line rather than under it, so that the card reads as one
      // sentence about this machine.
      expect(standing.textContent, `the machine ${named}`).toMatch(
        /\. No other devices are linked\.$/,
      );

      unmount();
    }
  });

  /// And the number is the devices linked to this one rather than the rows the
  /// list holds: this device is a row and is not linked to itself, so a
  /// Verkstead that has never met another says none.
  it("counts the devices linked rather than the rows drawn", async () => {
    mountCard(SERVING);

    await waitFor(() =>
      expect(screen.getByText(/No other devices are linked/)).toBeTruthy(),
    );

    expect(screen.queryByText(/1 other device/)).toBeNull();
  });

  /// One reads as a sentence rather than as a figure, and more than one counts.
  it("says one and says many", async () => {
    const { unmount } = mountCard(SERVING, {
      ...LINKED,
      members: LINKED.members.slice(0, 1),
    });

    await waitFor(() =>
      expect(screen.getByText(/One other device is linked\./)).toBeTruthy(),
    );

    unmount();

    mountCard(SERVING, {
      ...LINKED,
      members: [...LINKED.members, ...LINKED.members, ...LINKED.members],
    });

    await waitFor(() =>
      expect(screen.getByText(/6 other devices are linked\./)).toBeTruthy(),
    );
  });

  /// And the number is the rows the list draws rather than a figure answered
  /// beside them: one membership, one answer about it, so the sentence cannot
  /// come to disagree with what the pane is showing.
  it("counts the rows the list draws", async () => {
    mountCard(SERVING, LINKED);

    await waitFor(() =>
      expect(screen.getByText(/2 other devices are linked\./)).toBeTruthy(),
    );
  });
});

/// A drawn QR code, read the way a camera reads one.
///
/// The page draws an SVG of exact squares on a grid, so what a photograph of it
/// would hold can be built from the same two things a scanner works from: how
/// wide the grid is, and which of its cells are dark. The modules are blown up
/// so the decoder has more than one pixel of each to work with, the way a phone
/// held over a screen has.
///
/// Nothing of the encoder is consulted. What is asked here is whether the
/// picture on the page carries the link, which a comparison against the encoder
/// that drew it could not answer.
function scanned(code: Element): string | null {
  const size = Number(code.getAttribute("viewBox")?.split(" ")[2]);
  const dark = new Set(
    [
      ...(code.querySelector("path")?.getAttribute("d") ?? "").matchAll(MODULE),
    ].map(([, x, y]) => `${x},${y}`),
  );

  // Four pixels a module, which is more than jsQR's own minimum and less than
  // anything a test should spend on a bitmap.
  const scale = 4;
  const width = size * scale;
  const pixels = new Uint8ClampedArray(width * width * 4);

  for (let y = 0; y < width; y++) {
    for (let x = 0; x < width; x++) {
      const on = dark.has(`${Math.floor(x / scale)},${Math.floor(y / scale)}`);
      const at = (y * width + x) * 4;

      pixels[at] = pixels[at + 1] = pixels[at + 2] = on ? 0 : 255;
      pixels[at + 3] = 255;
    }
  }

  return jsQR(pixels, width, width)?.data ?? null;
}

/// One dark module of the path the page draws — see `Qr.tsx`, which writes each
/// as a one-unit square at its own place on the grid.
const MODULE = /M(\d+) (\d+)h1v1h-1z/g;
