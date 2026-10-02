//! What this device is forwarding, at the foot of the sidebar: the item beside
//! *Show archived*, and the popup it opens — see `src/workbench/Forwarding.tsx`.
//!
//! The reading is written here rather than read out of a fixture: a Forward is
//! a listener this machine holds for a member's terminal, and the golden
//! fixtures are written by a Verkstead in no cluster, which holds none.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ForwardView, ForwardsView } from "../src/api/types";
import dropdown from "../src/Menu.module.css";
import shell from "../src/Panes.module.css";
import archived from "../src/workbench/Archived.module.css";
import forwarding from "../src/workbench/Forwarding.module.css";
import {
  HIDING_SOMETHING,
  drawn,
  mount,
  mountSidebar,
  theWorkbench,
} from "./bench";
import { json, whenever } from "./serving";

afterEach(() => {
  vi.unstubAllGlobals();
});

/// A member of the cluster, by its Device Id, and what its row is called.
const MEMBER = "8f2a1c0b4d6e7f902b13c4d5e6f70819";

/// One Forward, forwarding unless the test says otherwise.
function forward(port: number, over: Partial<ForwardView> = {}): ForwardView {
  return {
    port,
    device: MEMBER,
    name: "build-box",
    os: "Linux",
    conversation: 7,
    title: "Port the parser",
    terminal: 1,
    standing: { kind: "forwarding" },
    ...over,
  };
}

/// The sidebar, over whatever this device is forwarding.
function forwardingThese(forwards: () => ForwardView[]) {
  return theWorkbench(
    whenever("/api/ui/forwards", () =>
      json({ forwards: forwards() } satisfies ForwardsView)(),
    ),
  );
}

/// The item, once drawn: the trigger of the foot's menu.
async function item(container: ParentNode): Promise<HTMLButtonElement> {
  return drawn<HTMLButtonElement>(
    container,
    `.${archived.showArchived} .${forwarding.forwarding} > button`,
  );
}

describe("the item at the foot of the sidebar", () => {
  it("reads one port in the singular", async () => {
    forwardingThese(() => [forward(3000)]);
    const { container } = mountSidebar("/");

    expect((await item(container)).textContent).toBe("Forwarding 1 port");
  });

  /// Every Forward, forwarding or skipped: the item is how this device says it
  /// holds anything for the cluster, and a skipped one is held — it is retried.
  it("counts every Forward, skipped ones with them", async () => {
    forwardingThese(() => [
      forward(3000),
      forward(5173),
      forward(8080, { standing: { kind: "skipped", reason: "port_busy" } }),
    ]);
    const { container } = mountSidebar("/");

    expect((await item(container)).textContent).toBe("Forwarding 3 ports");
  });

  /// Beside the switch, inside the foot — which stays the pane's last child.
  it("stands beside the switch, and the foot stays last in the pane", async () => {
    forwardingThese(() => [forward(3000)]);
    const { container } = mountSidebar("/");

    const trigger = await item(container);
    const foot = container.querySelector(`.${shell.paneFoot}`)!;

    expect(foot.contains(trigger)).toBe(true);
    expect(foot.contains(screen.getByLabelText("Show archived"))).toBe(true);
    expect(foot.parentElement!.lastElementChild).toBe(foot);
  });

  it("is not drawn while nothing is forwarded", async () => {
    const fetching = forwardingThese(() => []);
    const { container } = mountSidebar("/");

    await waitFor(() =>
      expect(
        fetching.mock.calls.some(
          ([path]) => String(path) === "/api/ui/forwards",
        ),
      ).toBe(true),
    );
    await screen.findByLabelText("Show archived");
    expect(container.querySelector(`.${forwarding.forwarding}`)).toBeNull();
  });

  /// The compose page's copy of the switch, drawn only while there is no list,
  /// draws no item: with no Conversation there is no terminal to forward from.
  it("is not drawn beside the compose page's switch", async () => {
    theWorkbench(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json(HIDING_SOMETHING)),
      whenever("/api/ui/forwards", json({ forwards: [forward(3000)] })),
    );
    const { container } = mount("/compose");

    await screen.findByLabelText("Show archived");
    expect(container.querySelector(`.${forwarding.forwarding}`)).toBeNull();
  });
});

describe("the popup it opens", () => {
  it("hangs over the item rather than under it", async () => {
    forwardingThese(() => [forward(3000)]);
    const { container } = mountSidebar("/");

    fireEvent.click(await item(container));

    const card = await drawn(
      container,
      `.${forwarding.forwarding} [role="menu"]`,
    );
    expect(card.classList.contains(dropdown.up!)).toBe(true);
  });

  /// Each Forward a link to `localhost` at its port, in a tab of its own, with
  /// the device it reaches and the Conversation the terminal is on.
  it("lists each Forward as a link with its device and Conversation", async () => {
    forwardingThese(() => [
      forward(3000),
      forward(5173, {
        device: "0123456789abcdef0123456789abcdef",
        name: "mac-mini",
        os: "macOS 15",
        conversation: 9,
        title: "Fix the sidebar",
      }),
    ]);
    const { container } = mountSidebar("/");

    fireEvent.click(await item(container));
    const card = await drawn(
      container,
      `.${forwarding.forwarding} [role="menu"]`,
    );
    const links = [
      ...card.querySelectorAll<HTMLAnchorElement>("a[role=menuitem]"),
    ];

    expect(links.map((link) => link.getAttribute("href"))).toEqual([
      "http://localhost:3000/",
      "http://localhost:5173/",
    ]);
    expect(links.every((link) => link.target === "_blank")).toBe(true);

    expect(links[0]!.textContent).toContain("localhost:3000");
    expect(links[0]!.textContent).toContain("build-box");
    expect(links[0]!.textContent).toContain("Port the parser");
    expect(links[1]!.textContent).toContain("mac-mini");
    expect(links[1]!.textContent).toContain("Fix the sidebar");

    // The device's mark beside its name, as the sidebar's cards draw it.
    expect(links.every((link) => link.querySelector(`.${forwarding.os}`))).toBe(
      true,
    );
  });

  /// A skipped one is no link — `localhost` at that number is whatever this
  /// machine already had there — and says why, dimmed.
  it("dims a skipped Forward and says why", async () => {
    forwardingThese(() => [
      forward(3000),
      forward(8080, { standing: { kind: "skipped", reason: "port_busy" } }),
    ]);
    const { container } = mountSidebar("/");

    fireEvent.click(await item(container));
    const card = await drawn(
      container,
      `.${forwarding.forwarding} [role="menu"]`,
    );

    const skipped = card.querySelector(`.${forwarding.skipped}`)!;
    expect(skipped).not.toBeNull();
    expect(skipped.tagName).not.toBe("A");
    expect(skipped.getAttribute("aria-disabled")).toBe("true");
    expect(skipped.textContent).toContain("localhost:8080");
    expect(skipped.textContent).toContain("port busy here");
    expect(card.querySelectorAll("a[role=menuitem]")).toHaveLength(1);
  });

  it("closes on Escape and hands the focus back to the item", async () => {
    forwardingThese(() => [forward(3000)]);
    const { container } = mountSidebar("/");

    const trigger = await item(container);
    fireEvent.click(trigger);
    await drawn(container, `.${forwarding.forwarding} [role="menu"]`);

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() =>
      expect(
        container.querySelector(`.${forwarding.forwarding} [role="menu"]`),
      ).toBeNull(),
    );
    expect(document.activeElement).toBe(trigger);
  });

  it("closes on a press outside it", async () => {
    forwardingThese(() => [forward(3000)]);
    const { container } = mountSidebar("/");

    fireEvent.click(await item(container));
    await drawn(container, `.${forwarding.forwarding} [role="menu"]`);

    fireEvent.click(container.querySelector(`.${dropdown.backdrop}`)!);

    await waitFor(() =>
      expect(
        container.querySelector(`.${forwarding.forwarding} [role="menu"]`),
      ).toBeNull(),
    );
  });

  /// What the `forwards` Nudge does to the page — `["forwards"]` read again,
  /// see `standsFor` in `src/nudge.ts` — moves the item and an open popup with
  /// it, with nothing reloaded.
  it("follows the reading while it is open", async () => {
    let held = [forward(3000)];
    forwardingThese(() => held);
    const { container, client } = mountSidebar("/");

    const trigger = await item(container);
    fireEvent.click(trigger);
    const card = await drawn(
      container,
      `.${forwarding.forwarding} [role="menu"]`,
    );
    expect(card.querySelectorAll("a[role=menuitem]")).toHaveLength(1);

    held = [forward(3000), forward(4000)];
    await client.invalidateQueries({ queryKey: ["forwards"] });

    await waitFor(() => expect(trigger.textContent).toBe("Forwarding 2 ports"));
    await waitFor(() =>
      expect(card.querySelectorAll("a[role=menuitem]")).toHaveLength(2),
    );
    expect(card.isConnected).toBe(true);
  });
});
