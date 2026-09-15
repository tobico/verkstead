//! The sandbox binds on the settings page: what the card says of them, what the
//! pane draws of each row, and what adding or taking one away puts on the wire.
//!
//! Two things mounted apart, because that is what they are: a card in the
//! middle pane saying how the list stands and whether anything is wrong with it,
//! and the rows that rewrite it in the details pane it opens. Both read the one
//! settings query and both save through the one endpoint, which is what the
//! groups below keep returning to: a press on one row must not cost what the
//! rest of the page is drawn from.
//!
//! Three things are worth a test rather than a reading of the source, and they
//! are what most of this file is:
//!
//! - **Whose an entry is.** The installation's own are a unit's word and nothing
//!   here can rewrite one, so they draw labelled and without a press. A page
//!   that offered to remove one would be a page offering something the server
//!   would silently ignore.
//! - **What a save carries.** One request writes the whole of `config.yaml`, so
//!   adding a bind must not cost a token, a build cache size or the bind
//!   standing beside it.
//! - **What a row reports.** Whether the server can see what an entry names is
//!   the one thing a human cannot check from a phone, and it is said in the
//!   server's own words on the row itself.
//! - **What a browse writes.** The field browses, and what a browse leaves in it
//!   goes to the server exactly as a typed path does — the dropdown is a way of
//!   writing the box rather than a second way of saving. How the dropdown itself
//!   behaves is asked in `browsing.test.tsx`, where the field is driven on its
//!   own.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with. What resolution *means*
//! is the server's — `crates/server/tests/settings.rs` is what says so — and the
//! variants below only move an entry between the two sources, which no router
//! behind a fixture has both of.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  DirectoryListing,
  SettingsSaved,
  SettingsView,
} from "../src/api/types";
import card from "../src/CardButton.module.css";
import {
  SandboxBindsCard,
  SandboxBindsPane,
} from "../src/settings/SandboxBinds";
import rowStyles from "../src/settings/PathEditor.module.css";
import styles from "../src/settings/SandboxBinds.module.css";
import { browse, held, listingAt, rows as offered, tap } from "./fields";
import { json, serving, whenever, type Answer } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The settings' own entries the fixture holds, as a save puts them back on the
/// wire. Two binds, which is what makes a Remove worth asking about: what one
/// press sends is the list with that one entry gone and the other where it was.
const BIND = "/var/cache/verkstead-node";
const BESIDE = "/var/cache/verkstead-cargo";

/// The rest of the settings as every save from this pane sends them: the author
/// as it stands, the token untouched, and what the sections above own.
const REST = {
  git_author: TOLD.git_author,
  github_token: "Keep",
  // The rules ride along as an action rather than a value: nothing this
  // form does says anything about them — see [`IgnoredCommentsEdit`].
  ignored_comments: "Keep",
  rust_build_cache: {
    enabled: TOLD.rust_build_cache.enabled,
    size: TOLD.rust_build_cache.size,
  },
  // And the Cleanup as the read left it, each duration as the string a form
  // holds — see [`heldCleanup`].
  cleanup: {
    trim: { enabled: true, days: "5" },
    delete: { enabled: true, days: "90" },
  },
  conflict_resolution: TOLD.conflict_resolution,
  share_on_done: TOLD.share_on_done,
};

/// The same settings with the installation having said one of its own as well,
/// which no fixture carries: the router that writes them is started with none of
/// its own, the way a standalone install is.
///
/// It goes in front of the settings' own, which is the order the server composes
/// the list in: a flag is said once when the machine is set up, and the file is
/// where somebody has been adding to it since.
function installed(standing: SettingsView): SettingsView {
  return {
    ...standing,
    paths: {
      binds: [
        {
          path: "/etc/verkstead/certs",
          source: "Installation",
          resolution: "Resolves",
        },
        ...standing.paths.binds,
      ],
    },
  };
}

/// And the same settings with every entry resolving, which is the ordinary
/// machine: the fixture's own name directories that are not on whatever wrote
/// it, so every row in it is unresolved.
function seen(standing: SettingsView): SettingsView {
  return {
    ...standing,
    paths: {
      binds: standing.paths.binds.map((entry) => ({
        ...entry,
        resolution: "Resolves" as const,
      })),
    },
  };
}

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

/// The card in the middle pane, and what pressing it asked for.
function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <SandboxBindsCard open={open} press={press} />),
    press,
  };
}

/// The binds in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <SandboxBindsPane back={back} />), back };
}

/// The settings, which is the one read this pane makes.
function theSettings(standing: SettingsView, ...answers: Array<Answer>) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// What a save answers with, which is the settings as they now stand.
function answering(standing: SettingsView): SettingsSaved {
  return { settings: standing, verified: null, refused: [] };
}

function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// The card itself, once it is drawn — waited for, because it stands on a read.
async function theCard(container: ParentNode): Promise<HTMLElement> {
  return await waitFor(() => {
    const face = container.querySelector<HTMLElement>(`.${styles.bindsCard}`);
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

/// The pane once the read it stands on has landed.
///
/// Waited for on the field that adds a path rather than on a row: the list is
/// the whole of the pane now, and a machine with nothing on it draws the empty
/// line where the rows would be.
async function list(container: ParentNode): Promise<ParentNode> {
  await waitFor(() => screen.getByLabelText("Add a path"));
  return container;
}

/// The pane's rows.
function rows(list: ParentNode): HTMLElement[] {
  return [...list.querySelectorAll<HTMLElement>(`.${rowStyles.row}`)];
}

/// What a row names.
function path(row: ParentNode): string {
  return row.querySelector(`.${rowStyles.path}`)!.textContent ?? "";
}

describe("the card", () => {
  /// What somebody scanning the page is after: what the section is, and how
  /// much of the list stands.
  it("says how many paths stand", async () => {
    theSettings(seen(TOLD));
    const { container } = mountCard();

    const face = await theCard(container);

    expect(face.querySelector("h2")!.textContent).toBe("Sandbox binds");
    expect(container.querySelector(`.${styles.standing}`)!.textContent).toBe(
      "2 paths.",
    );
  });

  /// A machine nobody has added one to is the ordinary state rather than one
  /// half set up, so the card counts and says nothing else.
  it("says nothing is wrong where nothing is configured at all", async () => {
    theSettings(UNSET);
    const { container } = mountCard();

    await theCard(container);

    expect(container.querySelector(`.${styles.standing}`)!.textContent).toBe(
      "0 paths.",
    );
    expect(container.querySelector(`.${styles.warning}`)).toBeNull();
  });

  /// And the other thing the browser can see and the human cannot: an entry that
  /// is saved, is in the file, and does nothing.
  it("counts every entry the server cannot see", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await theCard(container);

    await waitFor(() => screen.getByText(/2 entries the server cannot see/));
  });

  /// And it sends the human here to read why: this is where every row anybody
  /// can do anything about stands.
  it("sends the human to this section to read why", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await theCard(container);

    await waitFor(() =>
      screen.getByText(/2 entries the server cannot see\. Open this section/),
    );
  });

  it("says nothing of the kind where every entry resolves", async () => {
    theSettings(seen(TOLD));
    const { container } = mountCard();

    await theCard(container);

    expect(container.querySelector(`.${styles.warning}`)).toBeNull();
  });

  it("opens the pane when it is pressed", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    fireEvent.click(await theCard(container));

    expect(press).toHaveBeenCalled();
  });

  it("reads as open while its pane is", async () => {
    theSettings(TOLD);
    const { container } = mountCard(true);

    const face = await theCard(container);
    expect(face.classList).toContain(card.open);
    expect(face.getAttribute("aria-pressed")).toBe("true");
  });

  it("says so when the server could not be read at all", async () => {
    serving(() =>
      Promise.resolve(
        new Response("nope", { status: 500, statusText: "Server Error" }),
      ),
    );
    mountCard();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("the pane", () => {
  /// The pane's own title says what these are, so the one subsection heading
  /// inside it was saying it twice and is gone.
  it("draws no heading of its own inside the pane", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    const binds = await list(container);

    expect(binds.querySelector("h1")!.textContent).toBe("Sandbox binds");
    expect(binds.querySelector("h2")).toBeNull();
  });

  it("draws every bind", async () => {
    theSettings(installed(TOLD));
    const { container } = mountPane();

    const binds = await list(container);

    expect(rows(binds).map(path)).toEqual([
      "/etc/verkstead/certs",
      BIND,
      BESIDE,
    ]);
  });

  /// A bind could once be written for one Repo — `name=path` — and that grammar
  /// is gone. The server drops such an entry as it reads the file, so there is
  /// nothing for this pane to draw and nothing for it to explain.
  it("draws nothing for a bind written for a repo", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    const binds = await list(container);

    expect(rows(binds).map(path)).toEqual([BIND, BESIDE]);
    expect(screen.queryByText(/written for/)).toBeNull();
  });

  /// The installation's entries are a unit's word, and there is nothing on a
  /// phone that could rewrite a unit. So they say whose they are and carry no
  /// press.
  it("draws the installation's entries labelled and read-only", async () => {
    theSettings(installed(TOLD));
    const { container } = mountPane();

    const [installation, settings] = rows(await list(container));

    expect(installation!.querySelector(`.${rowStyles.source}`)!.textContent).toBe(
      "the installation's",
    );
    expect(installation!.querySelector(`.${rowStyles.remove}`)).toBeNull();

    // And the settings' own, which are the ones there is something to do to.
    expect(settings!.querySelector(`.${rowStyles.source}`)).toBeNull();
    expect(settings!.querySelector(`.${rowStyles.remove}`)).not.toBeNull();
  });

  /// The one thing a human cannot check from a phone, in the server's own words
  /// — on a nix install this sentence is how somebody learns the unit has to be
  /// widened before what they saved can work.
  it("says on a row why the server cannot see it", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    const binds = await list(container);

    expect(rows(binds)[0]!.querySelector(`.${rowStyles.unresolved}`)!.textContent)
      .toBe("the server cannot see it: there is nothing at that path");
  });

  it("says nothing on a row the server can see", async () => {
    theSettings(seen(TOLD));
    const { container } = mountPane();

    const binds = await list(container);

    expect(rows(binds)[0]!.querySelector(`.${rowStyles.unresolved}`)).toBeNull();
  });

  /// A machine nobody has added a bind to: the empty line rather than a warning,
  /// because there is nothing wrong with a sandbox that needs no extra directory.
  it("draws the empty line where nothing is configured at all", async () => {
    theSettings(UNSET);
    const { container } = mountPane();

    const binds = await list(container);

    await waitFor(() => screen.getByText("No paths are configured yet."));
    expect(rows(binds)).toHaveLength(0);
    expect(container.querySelector(`.${styles.warning}`)).toBeNull();
  });

  /// What the section is for, in one line. Everything else it used to say —
  /// what a bind is for, and what each one widens — has gone with the rest of
  /// the page's explanations.
  it("says in one line what the section configures", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() =>
      screen.getByText(
        "Configures additional paths which are accessible from within the sandbox.",
      ),
    );
    expect(screen.queryByText(/Each entry widens what a session/)).toBeNull();
  });

  it("says so when the settings could not be read at all", async () => {
    serving(() =>
      Promise.resolve(
        new Response("nope", { status: 500, statusText: "Server Error" }),
      ),
    );
    mountPane();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("adding a row", () => {
  /// The round trip, and the whole of what a save from this pane has to get
  /// right: the new entry on the end of the list, and everything else — the bind
  /// standing beside it, the author, the token and the build cache — exactly as
  /// it stood.
  it("saves a bind without disturbing anything else", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    mountPane();

    const field = await waitFor(() => screen.getByLabelText("Add a path"));
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        sandbox_binds: [BIND, BESIDE, "/var/cache/npm"],
      }),
    );
  });

  /// The installation's entries were never in this file, and sending one would
  /// be asking the server to write down a flag.
  it("sends back none of the installation's own", async () => {
    const standing = installed(TOLD);
    const fetching = theSettings(standing, json(answering(standing)));
    mountPane();

    const field = await waitFor(() => screen.getByLabelText("Add a path"));
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        sandbox_binds: [BIND, BESIDE, "/var/cache/npm"],
      }),
    );
  });

  /// The answer to the save is what the page then draws, which is the point of
  /// saving on the press: only the server knows whether the new entry resolves.
  it("draws the row the answer came back with", async () => {
    const now: SettingsView = {
      ...TOLD,
      paths: {
        binds: [
          ...TOLD.paths.binds,
          {
            path: "/var/cache/npm",
            source: "Settings",
            resolution: "Resolves",
          },
        ],
      },
    };
    theSettings(TOLD, json(answering(now)));
    const { container } = mountPane();

    const field = await waitFor(() => screen.getByLabelText("Add a path"));
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(async () => {
      const binds = await list(container);
      expect(rows(binds).map(path)).toEqual([BIND, BESIDE, "/var/cache/npm"]);
    });

    // And the box is empty again: what was in it has gone to the server, and the
    // row is what says so.
    expect((field as HTMLInputElement).value).toBe("");
  });

  it("sends nothing at all for an empty field", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    mountPane();

    await waitFor(() => screen.getByLabelText("Add a path"));
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    expect(
      fetching.mock.calls.some(([, init]) => init?.method === "POST"),
    ).toBe(false);
  });

  it("says so when the save fails", async () => {
    theSettings(TOLD, () =>
      Promise.resolve(
        new Response("nope", { status: 503, statusText: "Unavailable" }),
      ),
    );
    mountPane();

    const field = await waitFor(() => screen.getByLabelText("Add a path"));
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});

describe("browsing for one", () => {
  /// One level of the filesystem, as the endpoint behind the dropdown answers
  /// for it: the human's own home, and two directories inside it to browse to.
  const HOME: DirectoryListing = {
    Listed: {
      path: "/home/ada",
      entries: [
        { name: "src", path: "/home/ada/src", kind: "Directory" },
        { name: "work", path: "/home/ada/work", kind: "Directory" },
      ],
    },
  };

  /// The one directory every browse here reaches, held for the request the
  /// field makes: every path field asks anywhere the server can read.
  const inHome = whenever(listingAt("/home/ada"), json(HOME));

  /// Browse a field down to `/home/ada` and tap `work`, which is what leaves a
  /// path in the box that nobody typed.
  async function browsed(label: string): Promise<void> {
    const field = await waitFor(() => screen.getByLabelText(label));

    fireEvent.input(field, { target: { value: "/home/ada/" } });
    browse(label);

    await waitFor(() => expect(offered(label)).toContain("work"));
    tap(label, "work");

    expect(held(label)).toBe("/home/ada/work");
  }

  /// The point of the whole component: what a browse leaves in the field is
  /// what Add sends, and it is sent the way a typed path is — the same one
  /// request that writes the whole file, with everything else riding along.
  it("saves a bind a browse wrote, as a typed one is saved", async () => {
    const fetching = theSettings(TOLD, inHome, json(answering(TOLD)));
    mountPane();

    await browsed("Add a path");
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        sandbox_binds: [BIND, BESIDE, "/home/ada/work"],
      }),
    );
  });

});

describe("taking a row away", () => {
  /// What a Remove sends is the list with that one entry gone and every other
  /// one where it was — one request writes the whole of `config.yaml`, so a
  /// list sent short is a list emptied.
  it("keeps the bind beside the one that is removed", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    const { container } = mountPane();

    const binds = await list(container);
    fireEvent.click(
      rows(binds)[0]!.querySelector<HTMLButtonElement>(`.${rowStyles.remove}`)!,
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        sandbox_binds: [BESIDE],
      }),
    );
  });

  /// And where the row stands on the page is not where it stands in the file,
  /// once the installation has said one of its own in front of them: those are
  /// not in the file at all, so a removal counted off the page would take the
  /// wrong one away.
  it("counts past the installation's entries", async () => {
    const standing = installed(TOLD);
    const fetching = theSettings(standing, json(answering(standing)));
    const { container } = mountPane();

    const binds = await list(container);
    // The second row, which is the first the settings own.
    fireEvent.click(
      rows(binds)[1]!.querySelector<HTMLButtonElement>(`.${rowStyles.remove}`)!,
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        sandbox_binds: [BESIDE],
      }),
    );
  });
});
