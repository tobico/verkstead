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

import type {
  AddableView,
  AdditionView,
  DeviceIdentity,
  RepoEntry,
  SetView,
  SettingsView,
} from "../src/api/types";
import pill from "../src/Attaching.module.css";
import { ADDITION_REFUSAL } from "../src/set/Answering";
import { addedWith } from "../src/workbench/Timeline";
import sheet from "../src/set/Sheet.module.css";
import { SETTINGS } from "./bench";
import { answering } from "./reading";
import { json, readable, serving, whenever } from "./serving";
import answered from "./fixtures/set-answered.json" with { type: "json" };
import locked from "./fixtures/set-locked.json" with { type: "json" };
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

  /// What the Submit put on the work is drawn under the Answer it came with,
  /// and what came at the comment box under the comment.
  it("draws what was added as chips under each answer", async () => {
    await answering({
      ...readable(answered),
      additions: [
        { ...server(4, "tickets", "Q1"), state: "Applied" },
        {
          id: 5,
          label: null,
          added: { Device: { device: "1".repeat(32), name: "the-laptop" } },
          state: "Applied",
        },
      ],
    });

    expect(chipsOn("Q1")).toEqual(["tickets"]);
    expect(chipsOn(COMMENT)).toEqual(["the-laptop"]);
  });

  /// And on a set locked before anybody answered it, what was picked and
  /// never put on is struck through and says so.
  it("draws what a lock left unapplied struck through", async () => {
    const { page } = await answering({
      ...readable(locked),
      additions: [{ ...server(4, "tickets", "Q1"), state: "Struck" }],
    });

    expect(chipsOn("Q1")).toEqual(["tickets"]);
    const chip = page.querySelector(`.${pill.server}`)!;
    expect(chip.classList.contains(pill.struck!)).toBe(true);
    expect(chip.textContent).toContain("not added");
  });
});

describe("the timeline line under an answered set", () => {
  it("names what the answer added, a sentence per kind", () => {
    expect(addedWith([])).toBeNull();
    expect(
      addedWith([
        { Server: { name: "docs", declared: true } },
        { Server: { name: "tickets", declared: true } },
        { Device: { device: "1".repeat(32), name: "the-laptop" } },
      ]),
    ).toBe(
      "You added the docs and tickets MCP servers. You let the agent move this to the-laptop.",
    );
  });
});

describe("the timeline line under a set that added repos", () => {
  it("names each with its mode, and a read-write one with its branch", () => {
    expect(
      addedWith([
        {
          Companion: {
            repo_id: 7,
            name: "docs",
            mode: "ReadWrite",
            base_ref: null,
            branch: "limits",
          },
        },
        {
          Companion: {
            repo_id: 8,
            name: "infra",
            mode: "ReadOnly",
            base_ref: null,
            branch: null,
          },
        },
        {
          OpenedUp: { repo_id: 9, name: "website", base_ref: null, branch: "counting" },
        },
      ]),
    ).toBe(
      "You added docs read-write on limits, infra read-only and website opened up to read-write on counting.",
    );
  });
});

/// A registered repo, as the Repo list and the set's offer carry one.
function repo(id: number, name: string): RepoEntry {
  return {
    id,
    name,
    path: `/srv/${name}`,
    default_branch: "main",
    author: "work",
    author_unpicked: false,
  };
}

function device(id: string, name: string): DeviceIdentity {
  return {
    device: id,
    fingerprint: "AA:BB",
    name,
    os: "Linux",
    addresses: [],
    kept_by: null,
    version: null,
    install: null,
  };
}

const DOCS = repo(7, "docs");
const INFRA = repo(8, "infra");
const LAPTOP = device("11111111111111111111111111111111", "the-laptop");
const VM = device("22222222222222222222222222222222", "the-vm");

/// What the server offers past the servers: two repos to add, one read-only
/// companion to open up, and two devices.
const ADDABLE: AddableView = {
  repos: [DOCS],
  read_only: [
    {
      repo: INFRA,
      mode: "ReadOnly",
      base_ref: null,
      branch: "",
      worktree: null,
      base_commit: null,
    },
  ],
  devices: [LAPTOP, VM],
  branch: "rate-limiter",
};

const OFFERING: SetView = { ...WAITING, addable: ADDABLE };

/// Each repo's branches, which the card's base picker reads.
const BRANCHES = [DOCS, INFRA].map((one) =>
  whenever(`/api/ui/repos/${one.id}/branches`, json(["main", "develop"])),
);

describe("the repos and devices on the Attach menu", () => {
  /// The repo rows open a card, so they carry the ellipsis; the devices act at
  /// once. Every waiting set's menu, the comment box's included.
  it("offers each repo to add, each read-only companion to open up, and each device", async () => {
    await answering(OFFERING, SETTINGS_READ);

    for (const name of [menuOf("Q1"), menuOf(COMMENT)]) {
      const menu = await open(name);

      expect(rows(menu).slice(-4)).toEqual([
        "docs…",
        "Open up infra…",
        "the-laptop",
        "the-vm",
      ]);
      expect(
        within(menu).getByRole("group", { name: "May be transferred to" }),
      ).toBeTruthy();

      fireEvent.keyDown(menu, { key: "Escape" });
    }
  });

  /// Neither the server's leaving-out nor the sheet's: a repo or a device
  /// picked on the set already is not offered again.
  it("leaves out a repo and a device already picked on the set", async () => {
    await answering(
      {
        ...OFFERING,
        additions: [
          {
            id: 4,
            label: "Q3",
            added: {
              OpenedUp: {
                repo_id: INFRA.id,
                name: "infra",
                base_ref: null,
                branch: null,
              },
            },
            state: "Pending",
          },
          {
            id: 5,
            label: null,
            added: { Device: { device: VM.device, name: "the-vm" } },
            state: "Pending",
          },
        ],
      },
      SETTINGS_READ,
    );

    const menu = await open(menuOf("Q1"));
    expect(rows(menu)).not.toContain("Open up infra…");
    expect(rows(menu)).not.toContain("the-vm");
    expect(rows(menu)).toContain("the-laptop");
  });

  /// No cluster, or nobody in it the work could go to: no group at all.
  it("draws no device group where there is no other device", async () => {
    await answering(
      { ...OFFERING, addable: { ...ADDABLE, devices: [] } },
      SETTINGS_READ,
    );

    const menu = await open(menuOf("Q1"));
    expect(
      within(menu).queryByRole("group", { name: "May be transferred to" }),
    ).toBeNull();
  });
});

describe("a repo added from an answer", () => {
  const path = `/api/ui/sets/${WAITING.id}/additions`;

  /// The card opens read-only off the default branch, and says nothing about a
  /// branch until the switch makes it read-write.
  it("opens a card prefilled read-only off the default branch", async () => {
    await answering(OFFERING, SETTINGS_READ, ...BRANCHES);

    fireEvent.click(within(await open(menuOf("Q1"))).getByText("docs…"));

    const card = await waitFor(() => screen.getByRole("dialog"));
    expect(within(card).getByRole("heading").textContent).toBe("Add docs");
    expect(
      (within(card).getByRole("switch") as HTMLInputElement).checked,
    ).toBe(false);
    expect(within(card).getByLabelText("Base for docs").textContent).toContain(
      "Default branch (main)",
    );
    expect(within(card).queryByLabelText("Branch in docs")).toBeNull();
  });

  /// What the card says is what the record is asked to hold.
  it("records the repo with its mode, base and branch under the question", async () => {
    const { fetching } = await answering(
      OFFERING,
      SETTINGS_READ,
      ...BRANCHES,
      whenever(path, json("Answered"), "POST"),
    );

    fireEvent.click(within(await open(menuOf("Q1"))).getByText("docs…"));
    const card = await waitFor(() => screen.getByRole("dialog"));

    fireEvent.click(within(card).getByRole("switch"));
    const base = within(card).getByLabelText(
      "Base for docs",
    ) as HTMLSelectElement;
    await waitFor(() => expect(base.options).toHaveLength(3));
    fireEvent.change(base, { target: { value: "develop" } });

    const branch = within(card).getByLabelText(
      "Branch in docs",
    ) as HTMLInputElement;
    expect(branch.value, "filled in with what mirroring comes to").toBe(
      "rate-limiter",
    );
    fireEvent.input(branch, { target: { value: "docs-for-limits" } });

    fireEvent.click(within(card).getByRole("button", { name: "Add repo" }));

    await waitFor(() =>
      expect(writes(fetching, path)).toEqual([
        {
          label: "Q1",
          pick: {
            Companion: {
              repo_id: DOCS.id,
              mode: "ReadWrite",
              base_ref: "develop",
              branch: "docs-for-limits",
            },
          },
        },
      ]),
    );
    expect(screen.queryByRole("dialog"), "and the card goes").toBeNull();
  });

  /// Opening one up is the same card with the mode fixed.
  it("opens a read-only companion up through the card with no switch", async () => {
    const { fetching } = await answering(
      OFFERING,
      SETTINGS_READ,
      ...BRANCHES,
      whenever(path, json("Answered"), "POST"),
    );

    fireEvent.click(
      within(await open(menuOf(COMMENT))).getByText("Open up infra…"),
    );
    const card = await waitFor(() => screen.getByRole("dialog"));

    expect(within(card).queryByRole("switch")).toBeNull();
    expect(
      (within(card).getByLabelText("Branch in infra") as HTMLInputElement).value,
    ).toBe("rate-limiter");

    fireEvent.click(within(card).getByRole("button", { name: "Open up" }));

    await waitFor(() =>
      expect(writes(fetching, path)).toEqual([
        {
          label: null,
          pick: {
            OpenedUp: { repo_id: INFRA.id, base_ref: null, branch: null },
          },
        },
      ]),
    );
  });
});

describe("a device ticked from an answer", () => {
  /// No card: the row is the tick.
  it("is recorded at once under the question", async () => {
    const path = `/api/ui/sets/${WAITING.id}/additions`;
    const { fetching } = await answering(
      OFFERING,
      SETTINGS_READ,
      whenever(path, json("Answered"), "POST"),
    );

    fireEvent.click(within(await open(menuOf("Q1"))).getByText("the-laptop"));

    await waitFor(() =>
      expect(writes(fetching, path)).toEqual([
        { label: "Q1", pick: { Device: { device: LAPTOP.device } } },
      ]),
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("the chips of what was picked", () => {
  /// Each says what was picked in words: a repo's name with its mode and, for
  /// read-write, its branch — mirroring's where none was named — and a
  /// device's name.
  it("names the repo with its mode and branch, and the device", async () => {
    await answering({
      ...OFFERING,
      additions: [
        {
          id: 4,
          label: "Q1",
          added: {
            Companion: {
              repo_id: DOCS.id,
              name: "docs",
              mode: "ReadWrite",
              base_ref: null,
              branch: "docs-for-limits",
            },
          },
          state: "Pending",
        },
        {
          id: 5,
          label: "Q1",
          added: {
            Companion: {
              repo_id: 9,
              name: "website",
              mode: "ReadOnly",
              base_ref: null,
              branch: null,
            },
          },
          state: "Pending",
        },
        {
          id: 6,
          label: "Q1",
          added: {
            OpenedUp: {
              repo_id: INFRA.id,
              name: "infra",
              base_ref: null,
              branch: null,
            },
          },
          state: "Pending",
        },
        {
          id: 7,
          label: "Q1",
          added: { Device: { device: LAPTOP.device, name: "the-laptop" } },
          state: "Pending",
        },
      ],
    });

    expect(chipsOn("Q1")).toEqual(["docs", "website", "infra", "the-laptop"]);

    const row = screen.getByRole("list", { name: "Attached to Q1" });
    const said = [...row.querySelectorAll(`.${pill.server}`)].map((chip) =>
      chip.textContent!.replace("×", "").trim(),
    );
    expect(said).toEqual([
      "docsread-write on docs-for-limits",
      "websiteread-only",
      "infraopened up on rate-limiter",
      "the-laptop",
    ]);

    expect(
      screen.getByRole("button", { name: "Remove the-laptop" }),
      "and each comes off with its ×",
    ).toBeTruthy();
  });

  /// A repo unregistered since says so, as a server undeclared does.
  it("says so where the repo has gone", async () => {
    await answering({
      ...OFFERING,
      additions: [
        {
          id: 4,
          label: "Q1",
          added: {
            Companion: {
              repo_id: DOCS.id,
              name: null,
              mode: "ReadOnly",
              base_ref: null,
              branch: null,
            },
          },
          state: "Pending",
        },
      ],
    });

    const row = screen.getByRole("list", { name: "Attached to Q1" });
    expect(row.textContent).toContain("no longer registered");
  });
});
