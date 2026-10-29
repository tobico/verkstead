//! What the human adds to the work from a Question Set's sheet (ADR-0028): the
//! composer's plus-menu at every Answer field and at the comment box, its MCP
//! server rows, the chip a pick is drawn as, and the × that takes it off.
//!
//! Over the golden fixtures, one per role a Set is asked from — a grilling's
//! proposal, an implementation's ordinary round, a follow-up's and an
//! investigation's — because every waiting Set offers the menu whoever asked.

import { faPaperclip, faPlus } from "@fortawesome/free-solid-svg-icons";
import { fireEvent, screen, waitFor, within } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AdditionView, SetView, SettingsView } from "../src/api/types";
import pill from "../src/Attaching.module.css";
import { ADDITION_REFUSAL } from "../src/set/Answering";
import sheet from "../src/set/Sheet.module.css";
import { SETTINGS } from "./bench";
import { answering } from "./reading";
import { json, readable, serving, whenever } from "./serving";
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answeringSet from "./fixtures/set-answering.json" with { type: "json" };
import followingUp from "./fixtures/set-following-up.json" with { type: "json" };
import investigating from "./fixtures/set-investigating.json" with { type: "json" };
import proposing from "./fixtures/set-proposing.json" with { type: "json" };

/// The Set page's renderer is a page's own doing and these fixtures need none
/// of it; mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

/// The Set every test but the first is about: `Q1`, `Q2` with `Q2a` and
/// `Q2b`, and `Q3`.
const WAITING = readable(answeringSet);

/// One waiting Set per role that asks one.
const ROLES: Array<[string, SetView]> = [
  ["an implementation", WAITING],
  ["a grilling's proposal", readable(proposing)],
  ["a follow-up", readable(followingUp)],
  ["an investigation", readable(investigating)],
];

/// What the settings declare, which is what the server rows are drawn from.
function declaring(...names: string[]): SettingsView {
  return {
    ...SETTINGS,
    mcp_servers: names.map((name) => ({
      name,
      url: `https://mcp.example.com/${name}`,
      headers: [],
    })),
  };
}

const SETTINGS_READ = whenever(
  "/api/ui/settings",
  json(declaring("docs", "tickets", "alerts")),
);

/// A server picked on the Set, as the record holds it.
function server(id: number, name: string, label: string | null): AdditionView {
  return {
    id,
    label,
    added: { Server: { name, declared: true } },
    state: "Pending",
  };
}

/// The menus' own names: one per question, and the comment box's.
const menuOf = (label: string) => `Attach to ${label}`;
const COMMENT = "the question set";

/// Open one menu and wait out the settings read its server rows make.
async function open(name: string): Promise<HTMLElement> {
  fireEvent.click(screen.getByRole("button", { name }));

  const menu = screen.getByRole("menu", { name });
  await waitFor(() =>
    expect(within(menu).queryByText("Reading the settings…")).toBeNull(),
  );

  return menu;
}

function rows(menu: ParentNode): string[] {
  return [...menu.querySelectorAll('[role="menuitem"]')].map((row) =>
    row.textContent!.trim(),
  );
}

/// The server chips in the row named for `onto`.
function chipsOn(onto: string): string[] {
  const row = screen.queryByRole("list", { name: `Attached to ${onto}` });

  return row === null
    ? []
    : [...row.querySelectorAll(`.${pill.server} .${pill.serverName}`)].map(
        (one) => one.textContent!.trim(),
      );
}

/// The POSTs to one path, with their bodies.
function writes(
  fetching: ReturnType<typeof serving>,
  path: string,
): unknown[] {
  return fetching.mock.calls
    .filter(([asked, init]) => String(asked) === path && init?.method === "POST")
    .map(([, init]) => JSON.parse(String(init?.body ?? "{}")) as unknown);
}

beforeEach(() => localStorage.clear());
afterEach(() => localStorage.clear());

describe("the Attach menu on a waiting set", () => {
  for (const [role, set] of ROLES) {
    it(`is at every answer field and the comment box on ${role}'s sheet`, async () => {
      const { page } = await answering(set, SETTINGS_READ);

      const fields = [
        ...page.querySelectorAll<HTMLTextAreaElement>(
          'textarea[name$="-free-text"]',
        ),
      ];
      expect(fields.length, "the fixture asks something").toBeGreaterThan(0);

      for (const field of fields) {
        const label = field.name.replace(/-free-text$/, "");
        const trigger = screen.getByRole("button", { name: menuOf(label) });

        expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
        expect(trigger.querySelector("path")!.getAttribute("d")).toBe(
          faPlus.icon[4],
        );
      }

      expect(
        screen.getByRole("button", { name: menuOf(COMMENT) }),
        "and the comment box has one",
      ).toBeTruthy();

      expect(
        [...page.querySelectorAll("path")].some(
          (path) => path.getAttribute("d") === faPaperclip.icon[4],
        ),
        "no plain paperclip is left on the sheet",
      ).toBe(false);
    });
  }

  /// *Attach file…* first, then the declarations the Conversation does not
  /// have yet, then *Add a server…*.
  it("offers a file and every declared server the conversation lacks", async () => {
    await answering({ ...WAITING, servers: ["docs"] }, SETTINGS_READ);

    expect(rows(await open(menuOf("Q1")))).toEqual([
      "Attach file…",
      "tickets",
      "alerts",
      "Add a server…",
    ]);
  });

  /// The comment box takes no file of its own, so its menu has the server
  /// rows alone.
  it("offers no file at the comment box", async () => {
    await answering(WAITING, SETTINGS_READ);

    expect(rows(await open(menuOf(COMMENT)))).toEqual([
      "docs",
      "tickets",
      "alerts",
      "Add a server…",
    ]);
  });

  /// A server picked anywhere on the Set is not offered again: a Set adds a
  /// server once.
  it("leaves out a server already picked on the set", async () => {
    await answering(
      { ...WAITING, additions: [server(4, "tickets", "Q3")] },
      SETTINGS_READ,
    );

    expect(rows(await open(menuOf("Q1")))).toEqual([
      "Attach file…",
      "docs",
      "alerts",
      "Add a server…",
    ]);
  });
});

describe("a server added from an answer", () => {
  /// The pick goes up at once, under the question it was picked at, and the
  /// chip that comes back is the record's.
  it("is recorded on the set under that question and drawn as a chip", async () => {
    const path = `/api/ui/sets/${WAITING.id}/additions`;
    const { fetching, settles } = await answering(
      WAITING,
      SETTINGS_READ,
      whenever(
        path,
        json({ Added: { addition: server(4, "tickets", "Q1") } }),
        "POST",
      ),
    );

    settles({ ...WAITING, additions: [server(4, "tickets", "Q1")] });

    fireEvent.click(within(await open(menuOf("Q1"))).getByText("tickets"));

    await waitFor(() =>
      expect(writes(fetching, path)).toEqual([
        { label: "Q1", pick: { Server: { name: "tickets" } } },
      ]),
    );
    await waitFor(() => expect(chipsOn("Q1")).toEqual(["tickets"]));
    expect(chipsOn("Q2a"), "and under that question alone").toEqual([]);
  });

  /// At the comment box it names no question.
  it("is recorded under no question from the comment box", async () => {
    const path = `/api/ui/sets/${WAITING.id}/additions`;
    const { fetching } = await answering(
      WAITING,
      SETTINGS_READ,
      whenever(
        path,
        json({ Added: { addition: server(4, "docs", null) } }),
        "POST",
      ),
    );

    fireEvent.click(within(await open(menuOf(COMMENT))).getByText("docs"));

    await waitFor(() =>
      expect(writes(fetching, path)).toEqual([
        { label: null, pick: { Server: { name: "docs" } } },
      ]),
    );
  });

  /// Drawn off the Set, which is what a reload and a second device both read:
  /// each chip under the question it was picked at, and the comment box's
  /// under it.
  it("is drawn from the record under the question each names", async () => {
    await answering({
      ...WAITING,
      additions: [server(4, "tickets", "Q1"), server(5, "alerts", null)],
    });

    expect(chipsOn("Q1")).toEqual(["tickets"]);
    expect(chipsOn(COMMENT)).toEqual(["alerts"]);
    expect(chipsOn("Q3")).toEqual([]);
  });

  /// And a declaration deleted since says so on the chip.
  it("says so where its declaration has gone", async () => {
    await answering({
      ...WAITING,
      additions: [
        {
          ...server(4, "tickets", "Q1"),
          added: { Server: { name: "tickets", declared: false } },
        },
      ],
    });

    const row = screen.getByRole("list", { name: "Attached to Q1" });
    expect(row.textContent).toContain("no longer declared");
  });

  it("is taken off by its own ×", async () => {
    const path = `/api/ui/sets/${WAITING.id}/additions/4/remove`;
    const { fetching } = await answering(
      { ...WAITING, additions: [server(4, "tickets", "Q1")] },
      whenever(path, json("Removed"), "POST"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Remove tickets" }));

    await waitFor(() => expect(writes(fetching, path)).toHaveLength(1));
  });

  /// A refusal is said under the question it happened at.
  it("says under the question what could not be added", async () => {
    const path = `/api/ui/sets/${WAITING.id}/additions`;
    const { page } = await answering(
      WAITING,
      SETTINGS_READ,
      whenever(path, json("Answered"), "POST"),
    );

    fireEvent.click(within(await open(menuOf("Q1"))).getByText("tickets"));

    const question = page
      .querySelector('textarea[name="Q1-free-text"]')!
      .closest(`.${sheet.ask}`)!;
    await waitFor(() =>
      expect(question.textContent).toContain(ADDITION_REFUSAL.Answered),
    );
  });

  /// And a removal the server refuses, the same way.
  it("says under the question what could not be taken off", async () => {
    const path = `/api/ui/sets/${WAITING.id}/additions/4/remove`;
    const { page } = await answering(
      { ...WAITING, additions: [server(4, "tickets", "Q1")] },
      whenever(path, json("Locked"), "POST"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Remove tickets" }));

    const question = page
      .querySelector('textarea[name="Q1-free-text"]')!
      .closest(`.${sheet.ask}`)!;
    await waitFor(() =>
      expect(question.textContent).toContain(ADDITION_REFUSAL.Locked),
    );
  });
});

describe("a settled set", () => {
  /// Answered, it is the record: nothing to add with and nothing to take off.
  it("draws no Attach menu", async () => {
    await answering({
      ...readable(answered),
      additions: [server(4, "tickets", "Q1")],
    });

    expect(screen.queryByRole("button", { name: menuOf("Q1") })).toBeNull();
    expect(screen.queryByRole("button", { name: menuOf(COMMENT) })).toBeNull();
    expect(screen.queryByRole("button", { name: "Remove tickets" })).toBeNull();
  });
});
