//! The paths on the settings page: what the card says of the two lists, what the
//! pane draws of each row, and what adding or taking one away puts on the wire —
//! and the same rows on one Repo's own pane, which is where a bind scoped to a
//! Repo is drawn.
//!
//! Three things mounted apart, because that is what they are: a card in the
//! middle pane saying how the lists stand and whether anything is wrong with
//! them, the rows that rewrite them in the details pane it opens, and one Repo's
//! own binds in a section of the pane that Repo is opened at. All three read the
//! one settings query and all three save through the one endpoint, which is what
//! the last group below is about: a press on one of them must not cost what the
//! others are drawn from.
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

import type { RepoEntry, SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import { RepoBinds } from "../src/repos/RepoBinds";
import { PathsCard, PathsPane } from "../src/settings/Paths";
import rowStyles from "../src/settings/PathEditor.module.css";
import styles from "../src/settings/Paths.module.css";
import { hangs, json, serving, whenever } from "./serving";
import repos from "./fixtures/repos.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;
const REPOS = repos as RepoEntry[];

/// The settings' own entries the fixture holds, as a save puts them back on the
/// wire — one watched path, one bind every sandbox gets, and one a single Repo
/// gets. The last of those is not a row on this pane and rides along on every
/// save it makes.
const WATCHED = "/home/ada/src";
const BIND = "/var/cache/verkstead-node";
const SCOPED = "verkstead=/var/cache/verkstead-cargo";

/// The Repo that last one is written against, and the directory it names — the
/// two halves of it, because the pane that draws it knows them apart.
const REPO = "verkstead";
const OWN = "/var/cache/verkstead-cargo";

/// The rest of the settings as every save from this pane sends them: the author
/// as it stands, the token untouched, and the two the sections above own.
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
  share_viewer_url: TOLD.share_viewer_url,
  conflict_resolution: TOLD.conflict_resolution,
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

/// And the same settings with the installation having said a bind for one Repo
/// as well, which is the row that draws read-only on that Repo's pane.
function installedFor(standing: SettingsView, repo: string): SettingsView {
  return {
    ...standing,
    paths: {
      ...standing.paths,
      binds: [
        {
          path: "/var/cache/the-units-cargo",
          repo,
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

/// One Repo's own binds, as the section of that Repo's pane draws them. Given
/// the name rather than the Repo, because the name is what a bind is written
/// against and the pane around this is what knows which Repo it is on.
function mountOwn(repo = REPO) {
  return mounting(() => <RepoBinds repo={repo} />);
}

/// The settings, and the registry the pane reads beside them.
///
/// Both, because which names are registered is what tells a bind written for a
/// Repo from a stray — see `drawn` in `Paths.tsx`. The fixture's registry holds
/// the Repo the fixture's scoped bind is written for, so the ordinary case is a
/// pane with no stray on it.
function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return registered(standing, REPOS, ...answers);
}

/// The same, over a registry a test says: what makes an entry a stray is that
/// nothing on this list is called what it was written for.
function registered(
  standing: SettingsView,
  repos: RepoEntry[] | (() => Promise<Response>),
  ...answers: Array<() => Promise<Response>>
) {
  return serving(
    whenever("/api/ui/settings", json(standing)),
    whenever(
      "/api/ui/repos",
      typeof repos === "function" ? repos : json(repos),
    ),
    ...answers,
  );
}

/// The registry with the Repo the fixture's scoped bind is written for taken
/// off it, which is what unregistering one leaves behind.
const WITHOUT_THE_REPO = REPOS.filter((repo) => repo.name !== REPO);

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
  return [...list.querySelectorAll<HTMLElement>(`.${rowStyles.row}`)];
}

/// What a row names.
function path(row: ParentNode): string {
  return row.querySelector(`.${rowStyles.path}`)!.textContent ?? "";
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
  ///
  /// Every one of them, wherever it is drawn. A bind written for a Repo is read
  /// on that Repo's pane rather than here, and it goes stale unwatched exactly
  /// the same way — so a count that stopped at this section's own rows would put
  /// the only warning there is on a pane nobody opens unprompted.
  it("counts every entry the server cannot see, wherever it is drawn", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await theCard(container);

    // The watched path, the bind every sandbox gets, and the one written for a
    // Repo — which this section does not list and does count.
    await waitFor(() => screen.getByText(/3 entries the server cannot see/));
  });

  /// And it says which pane to open, because one of the three is not on this
  /// one: sending somebody to a list the row is not in is the warning that
  /// wastes the trip.
  it("sends the human to the repo's pane where one of them is a repo's", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await theCard(container);

    await waitFor(() =>
      screen.getByText(/the repo a bind is written for, to read why/),
    );
  });

  /// And says nothing of the sort where every unseen entry is one of its own.
  it("sends them only here where none of them is a repo's", async () => {
    const noneScoped: SettingsView = {
      ...TOLD,
      paths: {
        ...TOLD.paths,
        binds: TOLD.paths.binds.filter((entry) => entry.repo === null),
      },
    };

    theSettings(noneScoped);
    const { container } = mountCard();

    await theCard(container);

    await waitFor(() =>
      screen.getByText(/2 entries the server cannot see\. Open this section/),
    );
    expect(screen.queryByText(/the repo a bind is written for/)).toBeNull();
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

  /// Unless nothing is registered under the name it was written for, which is
  /// what unregistering a Repo leaves behind and what a misspelled name is from
  /// the start. No Repo's pane can draw one of those, so this one does: it is in
  /// the file, no session is given it, and a row nobody can reach is a row
  /// nobody can take away.
  it("draws a bind written for a repo nothing is registered under", async () => {
    registered(TOLD, WITHOUT_THE_REPO);
    const { container } = mountPane();

    const [, binds] = await lists(container);

    await waitFor(() => expect(rows(binds!)).toHaveLength(2));
    expect(rows(binds!).map(path)).toEqual([BIND, OWN]);
  });

  /// And says both things about it: which name it was written for, and that
  /// nothing holds that name — the second being why it is doing nothing.
  it("says on a stray which repo it was written for, and that nothing is", async () => {
    registered(TOLD, WITHOUT_THE_REPO);
    const { container } = mountPane();

    const [, binds] = await lists(container);
    await waitFor(() => expect(rows(binds!)).toHaveLength(2));

    const stray = rows(binds!)[1]!;

    expect(stray.textContent).toContain(`written for ${REPO}`);
    expect(stray.textContent).toContain("No repo is registered under that name");

    // And the row beside it, which is nobody's, says neither.
    expect(rows(binds!)[0]!.textContent).not.toContain("written for");
  });

  /// Nothing is a stray until the registry has been read. A row that appeared
  /// and vanished as that read landed would be worse than one that arrives a
  /// moment after the rest of the list.
  it("calls nothing a stray before the repos have been read", async () => {
    registered(TOLD, hangs());
    const { container } = mountPane();

    const [, binds] = await lists(container);

    expect(rows(binds!).map(path)).toEqual([BIND]);
  });

  /// And it can be taken away, which is the whole point of drawing it. What a
  /// Remove sends is where the row stands in the *file* — the stray sits behind
  /// the global bind there, and taking it away leaves that one alone.
  it("takes a stray away without disturbing the bind beside it", async () => {
    const fetching = registered(
      TOLD,
      WITHOUT_THE_REPO,
      json(answering(TOLD)),
    );
    const { container } = mountPane();

    const [, binds] = await lists(container);
    await waitFor(() => expect(rows(binds!)).toHaveLength(2));

    fireEvent.click(rows(binds!)[1]!.querySelector("button")!);

    await waitFor(() =>
      expect(sent(fetching)).toMatchObject({
        ...REST,
        watched_paths: [WATCHED],
        sandbox_binds: [BIND],
      }),
    );
  });

  /// The installation's entries are a unit's word, and there is nothing on a
  /// phone that could rewrite a unit. So they say whose they are and carry no
  /// press.
  it("draws the installation's entries labelled and read-only", async () => {
    theSettings(installed(TOLD));
    const { container } = mountPane();

    const [watched] = await lists(container);
    const [installation, settings] = rows(watched!);

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

    const [watched] = await lists(container);

    expect(rows(watched!)[0]!.querySelector(`.${rowStyles.unresolved}`)!.textContent)
      .toBe(
        "the server cannot see it: No such file or directory (os error 2)",
      );
  });

  it("says nothing on a row the server can see", async () => {
    theSettings(seen(TOLD));
    const { container } = mountPane();

    const [watched] = await lists(container);

    expect(rows(watched!)[0]!.querySelector(`.${rowStyles.unresolved}`)).toBeNull();
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
        `.${rowStyles.remove}`,
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
      rows(binds!)[0]!.querySelector<HTMLButtonElement>(`.${rowStyles.remove}`)!,
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
        `.${rowStyles.remove}`,
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

/// And the same rows on one Repo's own pane, which is where a bind written
/// against a Repo's name is drawn.
///
/// The section is mounted on its own, as the two above it are: what it is drawn
/// from is the settings read, and which Repo it is about is the name the pane
/// around it hands down. That the Repo's pane hands it down is `repos.test.tsx`'s.
describe("a repo's own binds", () => {
  /// The whole of what makes this a section rather than a filter on the Paths
  /// pane: a Repo is shown its own and nobody else's, and the bind every sandbox
  /// gets is the other pane's.
  it("draws the binds written against that repo and no others", async () => {
    theSettings(TOLD);
    const { container } = mountOwn();

    await waitFor(() => expect(rows(container)).toHaveLength(1));
    expect(rows(container).map(path)).toEqual([OWN]);
    expect(screen.queryByText(BIND)).toBeNull();
  });

  /// The same settings read on another Repo's pane, which is the other half of
  /// that: what is written for one Repo reaches no other, and the page says so
  /// by not drawing it.
  it("draws none of them on another repo's pane", async () => {
    theSettings(TOLD);
    const { container } = mountOwn("askance");

    await waitFor(() => screen.getByText("No binds of its own."));
    expect(rows(container)).toHaveLength(0);
  });

  /// A Repo with none is shown the empty editor rather than nothing at all: the
  /// pane is where somebody looks to learn that a repository can be given a
  /// directory of its own, and a section that appeared only once one existed
  /// would be a section nobody could find the first time.
  it("offers the field to a repo that has none", async () => {
    theSettings(TOLD);
    mountOwn("askance");

    await waitFor(() => screen.getByLabelText("Add a bind for this repo"));
    expect(screen.getByText(/Each entry widens what those sessions/)).toBeTruthy();
  });

  /// The installation's entries are a unit's word here as everywhere: labelled,
  /// and with nothing on them to press.
  it("draws the installation's own labelled and read-only", async () => {
    theSettings(installedFor(TOLD, REPO));
    const { container } = mountOwn();

    await waitFor(() => expect(rows(container)).toHaveLength(2));
    const [installation, settings] = rows(container);

    expect(installation!.querySelector(`.${rowStyles.source}`)!.textContent).toBe(
      "the installation's",
    );
    expect(installation!.querySelector(`.${rowStyles.remove}`)).toBeNull();

    expect(settings!.querySelector(`.${rowStyles.source}`)).toBeNull();
    expect(settings!.querySelector(`.${rowStyles.remove}`)).not.toBeNull();
  });

  /// And resolution is reported per row here as it is on the Paths pane, in the
  /// server's own words.
  it("says on a row why the server cannot see it", async () => {
    theSettings(TOLD);
    const { container } = mountOwn();

    await waitFor(() => expect(rows(container)).toHaveLength(1));
    expect(
      rows(container)[0]!.querySelector(`.${rowStyles.unresolved}`)!.textContent,
    ).toBe("the server cannot see it: there is nothing at that path");
  });

  it("says nothing on a row the server can see", async () => {
    theSettings(seen(TOLD));
    const { container } = mountOwn();

    await waitFor(() => expect(rows(container)).toHaveLength(1));
    expect(
      rows(container)[0]!.querySelector(`.${rowStyles.unresolved}`),
    ).toBeNull();
  });

  /// What the field on this section writes: the directory typed, against the
  /// name of the Repo whose pane it was typed on — and everything else in the
  /// file exactly as it stood, the watched path and the global bind included.
  it("saves a bind against this repo's name", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    mountOwn();

    const field = await waitFor(() =>
      screen.getByLabelText("Add a bind for this repo"),
    );
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED],
        sandbox_binds: [BIND, SCOPED, `${REPO}=/var/cache/npm`],
      }),
    );
  });

  /// And the installation's own is not sent back, on this pane as on the other:
  /// it was never in this file, and sending one would be asking the server to
  /// write down a flag.
  it("sends back none of the installation's own", async () => {
    const standing = installedFor(TOLD, REPO);
    const fetching = theSettings(standing, json(answering(standing)));
    mountOwn();

    const field = await waitFor(() =>
      screen.getByLabelText("Add a bind for this repo"),
    );
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED],
        sandbox_binds: [BIND, SCOPED, `${REPO}=/var/cache/npm`],
      }),
    );
  });

  /// Where the row stands on this section is not where it stands in the file:
  /// the global bind sits in front of it and is drawn on the other pane, so a
  /// removal counted off this section alone would take the wrong one away.
  it("takes one away without disturbing the global binds", async () => {
    const fetching = theSettings(TOLD, json(answering(TOLD)));
    const { container } = mountOwn();

    await waitFor(() => expect(rows(container)).toHaveLength(1));
    fireEvent.click(
      rows(container)[0]!.querySelector<HTMLButtonElement>(
        `.${rowStyles.remove}`,
      )!,
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        watched_paths: [WATCHED],
        sandbox_binds: [BIND],
      }),
    );
  });

  it("says so when the save fails", async () => {
    theSettings(TOLD, () =>
      Promise.resolve(
        new Response("nope", { status: 503, statusText: "Unavailable" }),
      ),
    );
    mountOwn();

    const field = await waitFor(() =>
      screen.getByLabelText("Add a bind for this repo"),
    );
    fireEvent.input(field, { target: { value: "/var/cache/npm" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => screen.getByText(/could not be saved/));
  });

  it("says so when the settings could not be read at all", async () => {
    serving(() =>
      Promise.resolve(
        new Response("nope", { status: 500, statusText: "Server Error" }),
      ),
    );
    mountOwn();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});
