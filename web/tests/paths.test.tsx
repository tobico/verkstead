//! The paths on the settings page: what the card says of the two lists, what the
//! pane draws of each row, and what adding or taking one away puts on the wire.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying how the lists stand and whether anything is wrong with them, and
//! the rows that rewrite them in the details pane it opens.
//!
//! Three things are worth a test rather than a reading of the source, and they
//! are what most of this file is:
//!
//! - **Whose an entry is.** The installation's own are a unit's word and nothing
//!   here can rewrite one, so they draw labelled and without a press. A page
//!   that offered to remove one would be a page offering something the server
//!   would silently ignore.
//! - **What a save carries.** One request writes the whole of `config.yaml`, so
//!   adding a watched path must not cost a bind, a token or a build cache size —
//!   and it must not cost a Repo's own bind either, which is a row this pane
//!   never draws and still has to send back.
//! - **What a row reports.** Whether the server can see what an entry names is
//!   the one thing a human cannot check from a phone, and it is said in the
//!   server's own words on the row itself.
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

import type { SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import { PathsCard, PathsPane } from "../src/settings/Paths";
import styles from "../src/settings/Paths.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The settings' own entries the fixture holds, as a save puts them back on the
/// wire — one watched path, one bind every sandbox gets, and one a single Repo
/// gets. The last of those is not a row on this pane and rides along on every
/// save it makes.
const WATCHED = "/home/ada/src";
const BIND = "/var/cache/verkstead-node";
const SCOPED = "verkstead=/var/cache/verkstead-cargo";

/// The rest of the settings as every save from this pane sends them: the author
/// as it stands, the token untouched, and the two the sections above own.
const REST = {
  git_author: TOLD.git_author,
  github_token: "Keep",
  rust_build_cache: {
    enabled: TOLD.rust_build_cache.enabled,
    size: TOLD.rust_build_cache.size,
  },
  share_viewer_url: TOLD.share_viewer_url,
};

/// The same settings with the installation having said one of each as well,
/// which no fixture carries: the router that writes them is started with none of
/// its own, the way a standalone install is.
///
/// They go in front of the settings' own, which is the order the server composes
/// the lists in: a flag is said once when the machine is set up, and the file is
/// where somebody has been adding to it since.
function installed(standing: SettingsView): SettingsView {
  return {
    ...standing,
    paths: {
      watched: [
        {
          path: "/srv/work",
          source: "Installation",
          resolution: "Resolves",
        },
        ...standing.paths.watched,
      ],
      binds: [
        {
          path: "/etc/verkstead/certs",
          repo: null,
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
      watched: standing.paths.watched.map((entry) => ({
        ...entry,
        resolution: "Resolves" as const,
      })),
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
    ...mounting(() => <PathsCard open={open} press={press} />),
    press,
  };
}

/// The two lists in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <PathsPane back={back} />), back };
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// What a save answers with, which is the settings as they now stand.
function answering(standing: SettingsView): SettingsSaved {
  return { settings: standing, verified: null };
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
    const face = container.querySelector<HTMLElement>(`.${styles.pathsCard}`);
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

/// The two lists of the pane, in the order they are read down: the watched paths
/// and then the binds.
async function lists(container: ParentNode): Promise<HTMLElement[]> {
  return await waitFor(() => {
    const both = [...container.querySelectorAll<HTMLElement>(`.${styles.list}`)];
    expect(both, "expected both lists to be drawn").toHaveLength(2);
    return both;
  });
}

/// One list's rows.
function rows(list: ParentNode): HTMLElement[] {
  return [...list.querySelectorAll<HTMLElement>(`.${styles.row}`)];
}

/// What a row names.
function path(row: ParentNode): string {
  return row.querySelector(`.${styles.path}`)!.textContent ?? "";
}

describe("the card", () => {
  /// What somebody scanning the page is after: how much of each list stands.
  /// The binds counted are the ones every sandbox gets — a Repo's own is on that
  /// Repo's pane, and counting it here would be counting a row this section
  /// cannot show.
  it("says how many watched paths and binds stand", async () => {
    theSettings(seen(TOLD));
    const { container } = mountCard();

    await theCard(container);

    expect(container.querySelector(`.${styles.standing}`)!.textContent).toBe(
      "1 watched path, and 1 bind every sandbox gets.",
    );
  });

  /// The state a fresh standalone install opens in, said with what it costs
  /// rather than as two empty lists that would read as a page with nothing left
  /// to ask for.
  it("says with no watched path that no repo can be registered", async () => {
    theSettings(UNSET);
    const { container } = mountCard();

    await theCard(container);

    await waitFor(() => screen.getByText(/No watched path is configured/));
    expect(screen.getByText(/no repo can be registered/)).toBeTruthy();
    expect(container.querySelector(`.${styles.warning}`)).not.toBeNull();
  });

  /// And the other thing the browser can see and the human cannot: an entry that
  /// is saved, is in the file, and does nothing.
  it("says how many entries the server cannot see", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await theCard(container);

    // The watched path and the bind every sandbox gets. The Repo's own bind is
    // unresolved too and is not this card's to count.
    await waitFor(() =>
      screen.getByText(/2 entries the server cannot see/),
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
  it("draws the two lists apart", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    const [watched, binds] = await lists(container);

    expect(watched!.querySelector("h2")!.textContent).toBe("Watched paths");
    expect(binds!.querySelector("h2")!.textContent).toBe("Sandbox binds");
  });

  it("draws every watched path and every global bind", async () => {
    theSettings(installed(TOLD));
    const { container } = mountPane();

    const [watched, binds] = await lists(container);

    expect(rows(watched!).map(path)).toEqual(["/srv/work", WATCHED]);
    expect(rows(binds!).map(path)).toEqual(["/etc/verkstead/certs", BIND]);
  });

  /// A bind scoped to one Repo is that Repo's pane's, so it is no row here — and
  /// the save below is what says it is still in the file.
  it("leaves a Repo's own bind off this pane", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    const [, binds] = await lists(container);

    expect(rows(binds!).map(path)).toEqual([BIND]);
    expect(screen.queryByText("/var/cache/verkstead-cargo")).toBeNull();
  });

  /// The installation's entries are a unit's word, and there is nothing on a
  /// phone that could rewrite a unit. So they say whose they are and carry no
  /// press.
  it("draws the installation's entries labelled and read-only", async () => {
    theSettings(installed(TOLD));
    const { container } = mountPane();

    const [watched] = await lists(container);
    const [installation, settings] = rows(watched!);

    expect(installation!.querySelector(`.${styles.source}`)!.textContent).toBe(
      "the installation's",
    );
    expect(installation!.querySelector(`.${styles.remove}`)).toBeNull();

    // And the settings' own, which are the ones there is something to do to.
    expect(settings!.querySelector(`.${styles.source}`)).toBeNull();
    expect(settings!.querySelector(`.${styles.remove}`)).not.toBeNull();
  });

  /// The one thing a human cannot check from a phone, in the server's own words
  /// — on a nix install this sentence is how somebody learns the unit has to be
  /// widened before what they saved can work.
  it("says on a row why the server cannot see it", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    const [watched] = await lists(container);

    expect(rows(watched!)[0]!.querySelector(`.${styles.unresolved}`)!.textContent)
      .toBe(
        "the server cannot see it: No such file or directory (os error 2)",
      );
  });

  it("says nothing on a row the server can see", async () => {
    theSettings(seen(TOLD));
    const { container } = mountPane();

    const [watched] = await lists(container);

    expect(rows(watched!)[0]!.querySelector(`.${styles.unresolved}`)).toBeNull();
  });

  /// The state a fresh standalone install opens in, with what it costs said
  /// beside it.
  it("says with no watched path that nothing can be registered", async () => {
    theSettings(UNSET);
    const { container } = mountPane();

    const [watched, binds] = await lists(container);

    await waitFor(() =>
      screen.getByText(/No watched path is configured anywhere/),
    );
    expect(screen.getByText(/nothing can be registered/)).toBeTruthy();
    expect(rows(watched!)).toHaveLength(0);
    expect(rows(binds!)).toHaveLength(0);
  });

  /// What every entry on the bind list costs, said beside the editor rather than
  /// as a step to press through.
  it("says beside the binds what each one widens", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => screen.getByText(/Each entry widens what a session/));
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
  /// right: the new entry on the end of its own list, and everything else — the
  /// binds, the Repo's own among them, the author, the token and the build cache
  /// — exactly as it stood.
  it("saves a watched path without disturbing anything else", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    mountPane();

    const field = await waitFor(() =>
      screen.getByLabelText("Add a watched path"),
    );
    fireEvent.input(field, { target: { value: "/home/ada/work" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]!);

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED, "/home/ada/work"],
        sandbox_binds: [BIND, SCOPED],
      }),
    );
  });

  it("saves a bind the same way", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    mountPane();

    const field = await waitFor(() => screen.getByLabelText("Add a bind"));
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[1]!);

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED],
        sandbox_binds: [BIND, SCOPED, "/var/cache/npm"],
      }),
    );
  });

  /// The installation's entries were never in this file, and sending one would
  /// be asking the server to write down a flag.
  it("sends back none of the installation's own", async () => {
    const standing = installed(TOLD);
    const fetching = theSettings(standing, json(answering(standing)));
    mountPane();

    const field = await waitFor(() =>
      screen.getByLabelText("Add a watched path"),
    );
    fireEvent.input(field, { target: { value: "/home/ada/work" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]!);

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED, "/home/ada/work"],
        sandbox_binds: [BIND, SCOPED],
      }),
    );
  });

  /// The answer to the save is what the page then draws, which is the point of
  /// saving on the press: only the server knows whether the new entry resolves.
  it("draws the row the answer came back with", async () => {
    const now: SettingsView = {
      ...TOLD,
      paths: {
        ...TOLD.paths,
        watched: [
          ...TOLD.paths.watched,
          {
            path: "/home/ada/work",
            source: "Settings",
            resolution: "Resolves",
          },
        ],
      },
    };
    theSettings(TOLD, json(answering(now)));
    const { container } = mountPane();

    const field = await waitFor(() =>
      screen.getByLabelText("Add a watched path"),
    );
    fireEvent.input(field, { target: { value: "/home/ada/work" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]!);

    await waitFor(async () => {
      const [watched] = await lists(container);
      expect(rows(watched!).map(path)).toEqual([WATCHED, "/home/ada/work"]);
    });

    // And the box is empty again: what was in it has gone to the server, and the
    // row is what says so.
    expect((field as HTMLInputElement).value).toBe("");
  });

  it("sends nothing at all for an empty field", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    mountPane();

    await waitFor(() => screen.getByLabelText("Add a watched path"));
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]!);

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

    const field = await waitFor(() =>
      screen.getByLabelText("Add a watched path"),
    );
    fireEvent.input(field, { target: { value: "/home/ada/work" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]!);

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});

describe("taking a row away", () => {
  it("sends the list without the row that was pressed", async () => {
    const fetching = theSettings(TOLD, json(answering(UNSET)));
    const { container } = mountPane();

    const [watched] = await lists(container);
    fireEvent.click(
      rows(watched!)[0]!.querySelector<HTMLButtonElement>(
        `.${styles.remove}`,
      )!,
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [],
        sandbox_binds: [BIND, SCOPED],
      }),
    );
  });

  /// Where the row stands on this pane is not where it stands in the file: a
  /// Repo's own bind sits among the settings' binds and is not drawn here, so a
  /// removal counted off the page would take the wrong one away.
  it("keeps a Repo's own bind when a global one is removed", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    const { container } = mountPane();

    const [, binds] = await lists(container);
    fireEvent.click(
      rows(binds!)[0]!.querySelector<HTMLButtonElement>(`.${styles.remove}`)!,
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED],
        sandbox_binds: [SCOPED],
      }),
    );
  });

  /// And neither is it where it stands among the settings' own, once the
  /// installation has said one of its own in front of them.
  it("counts past the installation's entries", async () => {
    const standing = installed(TOLD);
    const fetching = theSettings(standing, json(answering(standing)));
    const { container } = mountPane();

    const [watched] = await lists(container);
    // The second row, which is the only one the settings own.
    fireEvent.click(
      rows(watched!)[1]!.querySelector<HTMLButtonElement>(
        `.${styles.remove}`,
      )!,
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [],
        sandbox_binds: [BIND, SCOPED],
      }),
    );
  });
});
