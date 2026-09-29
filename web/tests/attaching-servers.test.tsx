//! The MCP servers a Conversation is given: the menu the composer's Attach grew
//! into, the chips a pick draws beside the attached files, and the × that takes
//! one off again.
//!
//! The other half of `attaching.test.tsx`, which is the same control asked about
//! files — and the other end of `mcp-servers.test.tsx`, which is where a server
//! is *declared*. What is here is the joining of the two: the menu lists what
//! the settings hold, and what the Conversation holds is a name that is looked
//! up in them.
//!
//! Over the golden fixtures like every other component test here, so the menu
//! and the row are drawn from what the two endpoints really answer with. The
//! drafting Conversation in `conversation.json` carries two servers — `docs`,
//! which `settings.json` declares, and `archive`, which nothing declares any
//! more — which leaves `tickets` declared and unattached for the menu to offer.

import { fireEvent, screen, waitFor, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import type { BriefEvent, ConversationView, SettingsView } from "../src/api/types";
import pill from "../src/Attaching.module.css";
import composer from "../src/workbench/Composer.module.css";
import shell from "../src/Panes.module.css";
import { OPEN, SETTINGS, drawn, mount, theWorkbench } from "./bench";
import { json, serving, whenever } from "./serving";
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };

/// A Conversation whose work has started: its Brief froze when the grilling
/// did, and the server on it froze with it.
const FROZEN = grilling as ConversationView;

/// The composer, opened on the drafting Conversation — or on a record handed in
/// its place, for the shapes the fixtures do not carry.
async function openComposer(at: ConversationView = OPEN): Promise<HTMLElement> {
  const { container } = mount(`/conversations/${at.id}`);
  return drawn(container, `.${shell.detailsPane} .${composer.composer}`);
}

/// The menu the paperclip opens, which is two presses from a cold pane: the
/// trigger, and then whatever the rows say.
async function openMenu(pane: ParentNode): Promise<HTMLElement> {
  fireEvent.click(
    await waitFor(() =>
      within(pane as HTMLElement).getByRole("button", { name: "Attach" }),
    ),
  );

  const menu = screen.getByRole("menu", { name: "Attach" });

  // The declarations are read when the menu opens rather than when the pane
  // does — see `ServerRows` in `Composer.tsx` — so the card comes down saying
  // it is reading them and fills in after. Waited out here, so that no test
  // has to know it happened.
  await waitFor(() =>
    expect(within(menu).queryByText("Reading the settings…")).toBeNull(),
  );

  return menu;
}

/// What the menu is offering, in the order it offers it.
function rows(menu: ParentNode): string[] {
  return [...menu.querySelectorAll('[role="menuitem"]')].map((row) =>
    row.textContent!.trim(),
  );
}

/// Every chip the row is drawing, which is the row minus the file pills.
function chips(pane: ParentNode): HTMLElement[] {
  return [
    ...pane.querySelectorAll<HTMLElement>(`.${pill.attachment}.${pill.server}`),
  ];
}

/// The names on them.
function named(pane: ParentNode): string[] {
  return chips(pane).map((one) =>
    one.querySelector(`.${pill.serverName}`)!.textContent!.trim(),
  );
}

/// What the page put on the wire as a POST to `path`.
function writes(
  fetching: ReturnType<typeof serving>,
  path: string,
): Array<RequestInit | undefined> {
  return fetching.mock.calls
    .filter(([asked, init]) => String(asked) === path && init?.method === "POST")
    .map(([, init]) => init);
}

/// The settings with another list of declarations in them.
function declaring(...names: string[]): SettingsView {
  return {
    ...SETTINGS,
    mcp_servers: names.map((name) => ({
      name,
      url: `https://mcp.example.com/${name}`,
    })),
  };
}

describe("the Attach menu on a draft's composer", () => {
  /// The whole of what the control became: what the button did is the first row,
  /// and the declarations are under it.
  it("offers Attach file first and then every declared server", async () => {
    theWorkbench();

    const menu = await openMenu(await openComposer());

    // `docs` and `archive` are on the Conversation already, so what is left to
    // offer is the one declaration it has not been given.
    expect(rows(menu)).toEqual(["Attach file", "tickets"]);
  });

  /// A server the Conversation holds is not in the menu: it has a chip of its
  /// own in the row, and the × on that chip is what puts it back.
  it("leaves out a server that is already attached", async () => {
    const pane = await openComposer();
    theWorkbench();

    expect(named(pane)).toEqual(["docs", "archive"]);
    expect(rows(await openMenu(pane))).not.toContain("docs");
  });

  /// And with nothing declared it is still a menu, rather than falling back to
  /// the plain button: what the human wants then is to go and declare one, and
  /// that is something a menu can say.
  it("holds Attach file and the way to the settings where none is declared", async () => {
    theWorkbench(
      whenever("/api/ui/settings", json(declaring())),
      whenever(`/api/ui/conversations/${OPEN.id}`, json({ ...OPEN, mcp_servers: [] })),
    );

    const menu = await openMenu(await openComposer());

    expect(rows(menu)).toEqual(["Attach file", "Declare an MCP server…"]);
    expect(
      screen.getByRole("menuitem", { name: "Declare an MCP server…" })
        .getAttribute("href"),
    ).toBe("/settings/mcp-servers");
  });

  /// Picking one is a request naming it, and nothing else — what the
  /// Conversation records is the name — and the chip that comes back is read
  /// off the Conversation rather than drawn from the press.
  it("attaches the server a row names and draws its chip", async () => {
    // The record as it stands before the press and as it stands after: the chip
    // arrives on the re-read the press asks for, which is what makes the row a
    // drawing of what the server says rather than of what the page did.
    let attached = false;

    const fetching = theWorkbench(
      whenever(`/api/ui/conversations/${OPEN.id}`, () =>
        json(
          attached
            ? {
                ...OPEN,
                mcp_servers: [
                  ...OPEN.mcp_servers,
                  { name: "tickets", declared: true },
                ],
              }
            : OPEN,
        )(),
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/mcp-servers/tickets`,
        () => {
          attached = true;
          return json("Attached")();
        },
        "POST",
      ),
    );

    const pane = await openComposer();
    const menu = await openMenu(pane);
    fireEvent.click(within(menu).getByRole("menuitem", { name: "tickets" }));

    await waitFor(() =>
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/mcp-servers/tickets`),
      ).toHaveLength(1),
    );

    await waitFor(() => expect(named(pane)).toContain("tickets"));

    // And the menu is gone with the press: the row that was pressed is about to
    // go, and a card still hanging under the trigger would be a list
    // rearranging itself under the hand that had finished with it.
    expect(screen.queryByRole("menu", { name: "Attach" })).toBeNull();
  });
});

describe("the chips on a draft's composer", () => {
  /// Drawn in the row the files are drawn in, after them: what the row is,
  /// taken together, is everything the human put on the work at this control.
  it("draws one chip per attached server, after the file pills", async () => {
    theWorkbench();

    const pane = await openComposer();
    const row = await drawn(pane, `.${pill.attachments}`);

    expect(named(row)).toEqual(
      OPEN.mcp_servers.map((server) => server.name),
    );

    const parts = [...row.children];
    expect(parts.indexOf(chips(row)[0]!)).toBeGreaterThan(
      parts.findIndex((one) => !one.classList.contains(pill.server!)),
    );
  });

  /// Told apart from a file at a glance, which is a class rather than a word:
  /// see `.server` in `Attaching.module.css`.
  it("draws a chip apart from a pill", async () => {
    theWorkbench();

    const row = await drawn(await openComposer(), `.${pill.attachments}`);
    const files = [...row.children].filter(
      (one) => !one.classList.contains(pill.server!),
    );

    expect(files).toHaveLength(OPEN.attachments.length);
    expect(chips(row)).toHaveLength(OPEN.mcp_servers.length);
  });

  /// The × is how one comes off, and it names the server it takes: a row of
  /// names with an unnamed × on each is a control a screen reader cannot tell
  /// from the one beside it.
  it("takes one off from its own ×", async () => {
    const fetching = theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/mcp-servers/docs/remove`,
        json("Removed"),
        "POST",
      ),
    );

    await openComposer();

    fireEvent.click(await waitFor(() => screen.getByRole("button", { name: "Remove docs" })));

    await waitFor(() =>
      expect(
        writes(
          fetching,
          `/api/ui/conversations/${OPEN.id}/mcp-servers/docs/remove`,
        ),
      ).toHaveLength(1),
    );
  });

  /// A chip whose declaration has been deleted says so rather than vanishing —
  /// and keeps its ×, that being the whole point of drawing it.
  it("says so where the declaration has gone, and still takes it off", async () => {
    theWorkbench();

    const pane = await openComposer();
    const gone = await waitFor(() => {
      const found = chips(pane).find((chip) =>
        chip.classList.contains(pill.missing!),
      );
      expect(found, "expected a chip whose server is gone").toBeTruthy();
      return found!;
    });

    expect(gone.textContent).toContain("no longer declared");
    expect(gone.textContent).toContain("archive");
    expect(
      within(gone).getByRole("button", { name: "Remove archive" }),
    ).toBeTruthy();
  });

  /// What could not be done, said under the row it happened in — one line for
  /// the whole row, a chip being a name on a line with nowhere in it to say a
  /// sentence.
  it("says under the row what could not be attached", async () => {
    theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/mcp-servers/tickets`,
        json("NoSuchServer"),
        "POST",
      ),
    );

    const pane = await openComposer();
    const menu = await openMenu(pane);
    fireEvent.click(within(menu).getByRole("menuitem", { name: "tickets" }));

    await waitFor(() =>
      expect(pane.textContent).toContain(
        "That server is no longer declared in the settings.",
      ),
    );
  });
});

describe("the servers on a frozen brief", () => {
  /// The Brief on a record, which is what the pane is opened at.
  function briefOn(at: ConversationView): BriefEvent {
    const found = at.timeline.find(
      (event): event is { Brief: BriefEvent } => "Brief" in event,
    );

    if (!found) throw new Error("the fixture should carry a Brief");
    return found.Brief;
  }

  async function openBrief(at: ConversationView): Promise<HTMLElement> {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json({ showing: false })),
      whenever("/api/ui/repos", json([])),
      whenever("/api/ui/profiles", json([])),
      whenever("/api/ui/abandoned-roadmaps", json([])),
      whenever("/api/ui/settings", json(SETTINGS)),
      whenever(`/api/ui/conversations/${at.id}`, json(at)),
      json([]),
    );

    const { container } = mount(
      `/conversations/${at.id}/events/${briefOn(at).id}`,
    );

    return drawn(container, `.${shell.detailsPane}`);
  }

  /// The same chips with nothing to press on one: the servers froze with the
  /// Brief, and the composer that could have changed them is gone.
  it("draws the chips read-only", async () => {
    expect(FROZEN.mcp_servers.length).toBeGreaterThan(0);

    const pane = await openBrief(FROZEN);
    const row = await drawn(pane, `.${pill.attachments}`);

    expect(named(row)).toEqual(FROZEN.mcp_servers.map((one) => one.name));
    for (const chip of chips(row)) {
      expect(chip.querySelector("button")).toBeNull();
    }

    expect(screen.queryByRole("button", { name: "Attach" })).toBeNull();
  });
});

/// And the property the whole of this turns on: what holds the servers is the
/// **Conversation**, not the Brief. So a Conversation that has been steered into
/// a further round — a Draft whose newest Brief is its second — draws the
/// servers it already had, attached and the human's to take off, where its files
/// are the round's own.
describe("a later drafting round", () => {
  /// The drafting fixture with a second Brief after its first, which is the
  /// shape a steered round leaves: the composer serves the newest one.
  function steered(): ConversationView {
    const first = OPEN.timeline.find(
      (event): event is { Brief: BriefEvent } => "Brief" in event,
    )!;

    return {
      ...OPEN,
      attachments: [],
      timeline: [
        ...OPEN.timeline,
        { Brief: { ...first.Brief, id: first.Brief.id + 1000, frozen: false } },
      ],
    };
  }

  it("shows the Conversation's servers attached and removable", async () => {
    const at = steered();
    theWorkbench(whenever(`/api/ui/conversations/${at.id}`, json(at)));

    const pane = await openComposer(at);
    const row = await drawn(pane, `.${pill.attachments}`);

    expect(named(row)).toEqual(at.mcp_servers.map((one) => one.name));

    // Read off the elements rather than through the roles: a Conversation with
    // a Timeline draws the pane beside one, and the pane a window this narrow
    // is not on is hidden from the accessibility tree.
    expect(
      chips(row).map((chip) =>
        chip.querySelector("button")?.getAttribute("aria-label"),
      ),
    ).toEqual(at.mcp_servers.map((one) => `Remove ${one.name}`));
  });
});
