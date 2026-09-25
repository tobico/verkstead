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
//! how many others are linked to it. It is drawn on every state of the pane —
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

import type { DevicesView, RemoteView, ServePress } from "../src/api/types";
import { RemoteCard, RemotePane } from "../src/settings/Remote";
import devices from "./fixtures/devices.json" with { type: "json" };
import devicesWsl from "./fixtures/devices-wsl.json" with { type: "json" };
import absent from "./fixtures/remote-absent.json" with { type: "json" };
import down from "./fixtures/remote-down.json" with { type: "json" };
import off from "./fixtures/remote-off.json" with { type: "json" };
import unreadableServe from "./fixtures/remote-serve-unreadable.json" with { type: "json" };
import serving from "./fixtures/remote-serving.json" with { type: "json" };
import done from "./fixtures/serve-done.json" with { type: "json" };
import ungranted from "./fixtures/serve-ungranted.json" with { type: "json" };
import { askedFor, json, whenever, serving as stubbing } from "./serving";

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

/// The login link the serving machine hands out, which is the address with the
/// key on the end of it.
const LINK = "https://workbench.tailnet-name.ts.net/?key=a-stated-workbench-key";

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
/// A second read beside the machine's, because the pane makes two: the serve
/// and the key are read off Tailscale and the Devices section is read off the
/// device, and neither of them waits on the other.
function theDevice(listed: DevicesView = DEVICES) {
  return whenever("/api/ui/devices", json(listed));
}

/// A server answering the two reads this section makes.
function theMachine(told: RemoteView, listed: DevicesView = DEVICES) {
  return stubbing(whenever("/api/ui/remote", json(told)), theDevice(listed));
}

function mountCard(told: RemoteView, listed: DevicesView = DEVICES) {
  theMachine(told, listed);
  return mounting(() => <RemoteCard open={false} press={vi.fn()} />);
}

function mountPane(told: RemoteView, listed: DevicesView = DEVICES) {
  theMachine(told, listed);
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
      expect(
        screen.getByText(/sudo systemctl start tailscaled/),
      ).toBeTruthy(),
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
    expect(
      screen.queryByText("sudo tailscale set --operator=ada"),
    ).toBeNull();
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
    expect(
      theBox().closest("label")?.getAttribute("title"),
    ).toMatch(/opened over the tailnet/);

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
    const fetching = theMachinePressed(SERVING, { press: "Done", reading: OFF });
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
    const { unmount } = mountCard(SERVING, { ...DEVICES, linked: 1 });

    await waitFor(() =>
      expect(screen.getByText(/One other device is linked\./)).toBeTruthy(),
    );

    unmount();

    mountCard(SERVING, { ...DEVICES, linked: 3 });

    await waitFor(() =>
      expect(screen.getByText(/3 other devices are linked\./)).toBeTruthy(),
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
    [...(code.querySelector("path")?.getAttribute("d") ?? "").matchAll(MODULE)]
      .map(([, x, y]) => `${x},${y}`),
  );

  // Four pixels a module, which is more than jsQR's own minimum and less than
  // anything a test should spend on a bitmap.
  const scale = 4;
  const width = size * scale;
  const pixels = new Uint8ClampedArray(width * width * 4);

  for (let y = 0; y < width; y++) {
    for (let x = 0; x < width; x++) {
      const on = dark.has(
        `${Math.floor(x / scale)},${Math.floor(y / scale)}`,
      );
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
