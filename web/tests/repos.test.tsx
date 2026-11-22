//! The Repos section of the settings page: the card saying how many are
//! registered, the pane it opens listing every one of them with a Remove
//! beside it, and the registration the Repo dropdown draws.
//!
//! The two halves are mounted apart, because that is what they are: a card in
//! the middle pane and the pane it opens beside it. The page that puts them
//! together — which path the card opens, and whether it reads as open while
//! that pane stands — is `settings.test.tsx`'s, along with the arithmetic
//! behind it.
//!
//! `tests/fixtures/repos.json` is a golden fixture like the profiles': `cargo
//! test` renders the real `/api/ui/repos` and writes the file, so what these
//! assertions read is the endpoint's own words.
//!
//! What is worth proving about a removal is that it is asked about first and
//! that a refusal reads as a refusal on the row it was pressed on. It is one
//! press in a list on a phone now, rather than the last press on a pane about
//! the repository, so the card in front of it is what stands between a thumb
//! and a registry entry; and a list of rows is where saying *which* repo a
//! refusal is about stops being automatic.
//!
//! What is worth proving about the registration is the same as it always was:
//! which paths are refused is the server's — the tests over there are what say
//! a relative path or one that is no repository root is turned away — and this
//! side's whole job is to say which of them happened in words the human can act
//! on, inside the card the path is about to be corrected in. It is asked from
//! the Repo dropdown now, so it is **Open repo** that is mounted for it; what
//! that modal does with a Repo it lands is `composing.test.tsx`'s.
//!
//! And that the browse writes the same box the typing does. The field is the
//! shared one — how the dropdown itself behaves is `browsing.test.tsx`'s, where
//! it is driven on its own — so what is asked here is what this form made of it:
//! the home an empty field opens on, the fixture the server wrote for the
//! directory under it, browsed down to the repository in that, and registered
//! exactly as a typed path is.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  DevicesView,
  DirectoryListing,
  RepoEntry,
  SettingsView,
} from "../src/api/types";
import card from "../src/CardButton.module.css";
import { Reaching, type Device } from "../src/reaching";
import {
  OpenRepo,
  ReposCard,
  ReposPane,
  type RepoRefused,
} from "../src/repos/RepoList";
import styles from "../src/repos/RepoList.module.css";
import { ANNEX_REPOS } from "../src/remembered";
import settingsCard from "../src/settings/SettingsCard.module.css";
import head from "../src/workbench/PaneHead.module.css";
import { drawn } from "./bench";
import {
  browse,
  held,
  listingAt,
  marked,
  rows as offered,
  tap,
} from "./fields";
import {
  askedFor,
  json,
  serving,
  whenever,
  type Answer,
} from "./serving";
import { GITHUB } from "../src/forges";
import { CLIENT, oneList } from "./authors";
import repos from "./fixtures/repos.json" with { type: "json" };
import listing from "./fixtures/directories.json" with { type: "json" };
import keeping from "./fixtures/devices-keeping.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };

const REPOS = repos as RepoEntry[];
const FIRST = REPOS[0]!;

afterEach(() => {
  vi.unstubAllGlobals();
});

/// No retries: a test that asked for a refusal should see it at once, rather
/// than after the three attempts a real page is right to make.
function client(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

/// Whichever half of the section a test is about, over one query client: both
/// halves read the same list, so a test mounting the pair reads it once, exactly
/// as the page does. The client comes back with it, for the tests that are about
/// what a write leaves out of date rather than about what is drawn.
function mounting(what: () => JSX.Element) {
  const queries = client();

  return {
    ...render(() => (
      <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
    )),
    queries,
  };
}

/// The card in the middle pane, and what pressing it asked for.
function mountCard(open = false) {
  const press = vi.fn();

  return {
    ...mounting(() => <ReposCard open={open} press={press} />),
    press,
  };
}

/// The pane it opens, and what its way back asked for.
function mountPane() {
  const back = vi.fn();

  return { ...mounting(() => <ReposPane back={back} />), back };
}

/// And the registration, on the modal the Repo dropdown opens it as.
function mountOpen() {
  const close = vi.fn();
  const landed = vi.fn();

  return {
    ...mounting(() => <OpenRepo close={close} landed={landed} />),
    close,
    landed,
  };
}

/// The list as it stands, with whatever the pane's writes are answered by.
function theRepos(...answers: Array<Answer>) {
  return serving(whenever("/api/ui/repos", json(REPOS)), ...answers);
}

/// And the removal of one Repo, answered however the test says.
function removing(repo: RepoEntry, answer: () => Promise<Response>) {
  return whenever(`/api/ui/repos/${repo.id}/remove`, answer, "POST");
}

/// One repo's row, by the name on it.
function theRow(name: string): HTMLElement {
  return screen.getByText(name).closest(`.${styles.repo}`)!;
}

/// Press the Remove on one repo's row, which asks rather than acts.
function removePressed(name: string) {
  fireEvent.click(
    theRow(name).querySelector<HTMLButtonElement>(`.${styles.remove}`)!,
  );
}

/// The card the press is answered with, once it is up.
async function asked(): Promise<HTMLElement> {
  return await drawn<HTMLElement>(document, `dialog.${styles.confirming}`);
}

/// The two ways out of it, in the order they are drawn.
function ways(sheet: HTMLElement): HTMLButtonElement[] {
  return [
    ...sheet.querySelectorAll<HTMLButtonElement>(
      `.${styles.confirmingOut} button`,
    ),
  ];
}

/// Confirm the press that was asked about, which is the removal itself.
async function confirmed() {
  fireEvent.click(ways(await asked())[1]!);
}

/// Type a path into the registration and send it.
function register(path: string) {
  fireEvent.input(screen.getByLabelText(/absolute path/i), {
    target: { value: path },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open" }));
}

describe("the card", () => {
  it("asks the server for the Repos it has been told about", async () => {
    const fetching = theRepos();
    mountCard();

    await waitFor(() => screen.getByText(`${REPOS.length} repos`));
    expect(fetching).toHaveBeenCalledWith("/api/ui/repos", expect.anything());
  });

  /// A count is the whole of what the card says, in its one line beside the
  /// title: every item on the list it opens is a name, and there is nothing
  /// else about one to scan for.
  it("says how many are registered", async () => {
    theRepos();
    const { container } = mountCard();

    await waitFor(() =>
      expect(
        container.querySelector(`.${settingsCard.status}`)?.textContent,
      ).toBe(`${REPOS.length} repos`),
    );
    expect(container.querySelector("h2")!.textContent).toBe("Repos");
  });

  /// In English rather than as `1 repos`.
  it("counts one of them in the singular", async () => {
    serving(whenever("/api/ui/repos", json([FIRST])));
    mountCard();

    await waitFor(() => screen.getByText("1 repo"));
  });

  /// And none is said as none rather than as `0 repos` — and as a reading,
  /// not a warning: a Verkstead with nothing registered yet is waiting on
  /// nobody, the first Repo being registered from wherever it is wanted.
  /// A Repo whose remote no forge answers for says so on its row, under the
  /// path — and one on a forge says nothing about it.
  it("says a repo on no forge is on none, naming the host", async () => {
    const hosted: RepoEntry[] = [
      {
        ...FIRST,
        forge: { kind: "Unmatched", host: "git.example.org" },
      },
    ];
    serving(whenever("/api/ui/repos", json(hosted)));
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));

    expect(theRow(FIRST.name).textContent).toBe(
      `${FIRST.name}${FIRST.path}No forge for git.example.orgRemove`,
    );
  });

  it("says so plainly when none are registered", async () => {
    serving(whenever("/api/ui/repos", json([])));
    mountCard();

    const line = await waitFor(() => screen.getByText("None registered"));
    expect(line.classList).toContain(settingsCard.status);
    expect(line.classList).not.toContain(settingsCard.warn);
  });

  /// A card is pressed to open the pane beside it, like every other card on
  /// this page: an `article` rather than a button, because it holds more than a
  /// run of text.
  it("opens the pane when it is pressed", async () => {
    theRepos();
    const { container, press } = mountCard();

    const face = await drawn<HTMLElement>(container, `.${styles.reposCard}`);
    expect(face.classList).toContain(card.card);
    expect(face.getAttribute("role")).toBe("button");
    expect(face.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(face);
    expect(press).toHaveBeenCalled();
  });

  it("reads as open while its pane is", async () => {
    theRepos();
    const { container } = mountCard(true);

    const face = await drawn<HTMLElement>(container, `.${styles.reposCard}`);
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);
  });

  it("shows the server's own wording when the list cannot be read", async () => {
    serving(
      whenever(
        "/api/ui/repos",
        json({ error: "The registered repos couldn't be read." }, 500),
      ),
    );
    mountCard();

    await waitFor(() =>
      screen.getByText(/The registered repos couldn't be read/),
    );
  });
});

describe("the pane it opens", () => {
  it("lists every registered repo by name, in the order it was given", async () => {
    theRepos();
    const { container } = mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    expect(
      [...container.querySelectorAll(`.${styles.repo} .${styles.name}`)].map(
        (name) => name.textContent,
      ),
    ).toEqual(REPOS.map((repo) => repo.name));
  });

  /// A row is the name, the directory under it, and the one press there is to
  /// make about a Repo. What a repository *holds* is read where it is used —
  /// the branches on the composer, the roadmaps in the new conversation
  /// dropdown — rather than listed here.
  it("puts the name, its path and a Remove on a row, and nothing else", async () => {
    theRepos();
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));

    const row = theRow(FIRST.name);
    expect(
      row.querySelector<HTMLButtonElement>(`.${styles.remove}`)!.textContent,
    ).toBe("Remove");
    expect(row.textContent).toBe(`${FIRST.name}${FIRST.path}Remove`);
    // Not a card, and nothing to open: a name is not a surface to read
    // something off.
    expect(row.querySelector(`.${card.card}`)).toBeNull();
  });

  /// A Repo on a forge its Author holds no token for says so on its row: the
  /// forge is not asked, so no wrap-up is started there (Set 1878, Q3).
  it("says on a row that its author has no token for its forge", async () => {
    const on: RepoEntry = {
      ...FIRST,
      author: null,
      forge: { kind: "On", uuid: GITHUB, name: "GitHub" },
    };
    const author = {
      id: "6f1c2a52-7d0e-4c55-9a43-0d6b8f3e2a11",
      display_name: "Work",
      name: "Ada Lovelace",
      email: "ada@work.example",
    };
    serving(
      whenever("/api/ui/repos", json([on])),
      whenever("/api/ui/authors", json([{ ...author, tokens: [] }])),
    );
    mountPane();

    await waitFor(() => screen.getByText("No GitHub token for Work"));
  });

  /// And says nothing of it where the Author holds one.
  it("says nothing of a token its author holds", async () => {
    const on: RepoEntry = {
      ...FIRST,
      author: null,
      forge: { kind: "On", uuid: GITHUB, name: "GitHub" },
    };
    serving(
      whenever("/api/ui/repos", json([on])),
      whenever(
        "/api/ui/authors",
        json([
          {
            id: "6f1c2a52-7d0e-4c55-9a43-0d6b8f3e2a11",
            display_name: "Work",
            name: "Ada Lovelace",
            email: "ada@work.example",
            tokens: [GITHUB],
          },
        ]),
      ),
    );
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    expect(theRow(FIRST.name).textContent).toBe(`${FIRST.name}${FIRST.path}Remove`);
  });

  /// And the path is on the row because the name is not unique.
  ///
  /// A name is the directory's own and nothing stops two registered
  /// repositories in different places sharing one — the store orders its list
  /// by `name, id` for that reason. This is the one list that puts them side by
  /// side, with an unregistering behind each, so two rows that read the same
  /// would be a press somebody could only get right by luck.
  it("tells two repos of one name apart", async () => {
    const twins: RepoEntry[] = [
      { id: 3, name: "api", path: "/srv/repos/api", default_branch: "main", author: null, author_unpicked: false, github: null, origin: null, forge: null },
      { id: 4, name: "api", path: "/work/api", default_branch: "main", author: null, author_unpicked: false, github: null, origin: null, forge: null },
    ];
    serving(whenever("/api/ui/repos", json(twins)));
    const { container } = mountPane();

    await waitFor(() =>
      expect(container.querySelectorAll(`.${styles.repo}`)).toHaveLength(2),
    );

    expect(
      [...container.querySelectorAll(`.${styles.repo} .${styles.path}`)].map(
        (path) => path.textContent,
      ),
    ).toEqual(["/srv/repos/api", "/work/api"]);
  });

  it("says so plainly when none are registered", async () => {
    serving(whenever("/api/ui/repos", json([])));
    const { container } = mountPane();

    await waitFor(() => screen.getByText(/No repos are registered yet\./));
    expect(container.querySelector(`.${styles.repo}`)).toBeNull();
  });

  it("shows the server's own wording when the list cannot be read", async () => {
    serving(
      whenever(
        "/api/ui/repos",
        json({ error: "The registered repos couldn't be read." }, 500),
      ),
    );
    mountPane();

    await waitFor(() =>
      screen.getByText(/The registered repos couldn't be read/),
    );
  });

  /// The way back out of it, in the slot every pane keeps for it: a change of
  /// level rather than a navigation, which is the page's to make.
  it("goes back to the settings", async () => {
    theRepos();
    const { container, back } = mountPane();

    const out = await drawn<HTMLButtonElement>(container, `.${head.back}`);
    expect(out.textContent).toContain("Settings");

    fireEvent.click(out);
    expect(back).toHaveBeenCalled();
  });
});

describe("removing a repo", () => {
  /// Asked about before anything happens, the way a close over a run in flight
  /// is: it is one press in a list, on a phone, and it cannot be taken back.
  it("asks before it takes anything off the registry", async () => {
    const fetching = theRepos(removing(FIRST, json("Removed")));
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);

    const sheet = await asked();
    expect(
      sheet.querySelector(`.${styles.confirmingTitle}`)!.textContent,
    ).toBe(`Remove ${FIRST.name}?`);
    expect(ways(sheet).map((way) => way.textContent)).toEqual([
      "Cancel",
      "Remove",
    ]);

    // And nothing has been asked of the server: the press was a question.
    expect(
      fetching.mock.calls.filter(([, init]) => init && "method" in init),
    ).toHaveLength(0);
  });

  /// The card names the repo because the list it was pressed in is behind it: a
  /// card asking about *this repo* over a list of them would be asking about
  /// whichever row the human remembers pressing.
  it("names the repo the press was made on", async () => {
    theRepos(removing(REPOS[1]!, json("Removed")));
    mountPane();

    await waitFor(() => screen.getByText(REPOS[1]!.name));
    removePressed(REPOS[1]!.name);

    expect(
      (await asked()).querySelector(`.${styles.confirmingTitle}`)!.textContent,
    ).toBe(`Remove ${REPOS[1]!.name}?`);
  });

  it("leaves the registry alone where the card is answered the other way", async () => {
    const fetching = theRepos(removing(FIRST, json("Removed")));
    const { container } = mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    fireEvent.click(ways(await asked())[0]!);

    await waitFor(() =>
      expect(container.querySelector(`dialog.${styles.confirming}`)).toBeNull(),
    );
    expect(
      fetching.mock.calls.filter(([, init]) => init && "method" in init),
    ).toHaveLength(0);
  });

  it("asks the server to take it off the registry once the press is confirmed", async () => {
    const fetching = theRepos(removing(FIRST, json("Removed")));
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    await confirmed();

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        `/api/ui/repos/${FIRST.id}/remove`,
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  /// The list is read again, and the roadmaps waiting with it: the offers are
  /// drawn from whatever is registered, so a repository leaving takes whatever
  /// it was holding out of them.
  it("reads the list and the roadmap offers again", async () => {
    theRepos(removing(FIRST, json("Removed")));
    const { queries } = mountPane();
    const invalidated = vi.spyOn(queries, "invalidateQueries");

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    await confirmed();

    await waitFor(() =>
      expect(invalidated).toHaveBeenCalledWith({ queryKey: ["repos"] }),
    );
    expect(invalidated).toHaveBeenCalledWith({
      queryKey: ["abandoned-roadmaps"],
    });
  });

  /// Refused rather than taken out from under the work going on in it, and said
  /// under the row the press was made on — which is what says which repo the
  /// refusal is about, now that the press is one of several on a list.
  it("says why a repo with live work on it could not be removed", async () => {
    theRepos(removing(FIRST, json("InUse")));
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    await confirmed();

    await waitFor(() =>
      screen.getByText(/A conversation that's still going is on it/),
    );
    expect(theRow(FIRST.name).querySelector(`.${styles.failure}`)).toBeTruthy();
    // And said about that repo alone: no other row carries it.
    expect(theRow(REPOS[1]!.name).querySelector(`.${styles.failure}`)).toBeNull();
  });

  /// A page left open in another tab, whose repo somebody has already taken
  /// away: a refusal in words rather than a failure.
  it("says so where the repo is off the registry already", async () => {
    theRepos(removing(FIRST, json("NoSuchRepo")));
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    await confirmed();

    await waitFor(() => screen.getByText(/off the registry already/));
  });

  /// And a server that could not answer at all, which is the one thing here
  /// that is an error rather than an outcome — said on the same row, for the
  /// same reason.
  it("says so when the server could not answer", async () => {
    theRepos(
      removing(FIRST, json({ error: "The repo couldn't be removed." }, 500)),
    );
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    await confirmed();

    await waitFor(() => screen.getByText(/couldn't be removed/));
    expect(theRow(FIRST.name).querySelector(`.${styles.failure}`)).toBeTruthy();
  });
});

/// The registration, which has left the settings: it is asked from the Repo
/// dropdown, over the **Open repo** modal, and nothing on the settings page
/// registers a repository at all.
/// The settings as the fixture has them: one Author.
const ONE_AUTHOR = told as SettingsView;
const WORK = ONE_AUTHOR.authors[0]!;

/// And a second, with a display name, which is what its rows read as.
const HOME: SettingsView["authors"][number] = {
  id: "9b1e7c2d-4a5f-4e8b-8c3d-2f1a0e9d8c7b",
  display_name: "Home",
  name: "A. L.",
  email: "al@home.example",
  tokens: [],
};

const TWO_AUTHORS: SettingsView = {
  ...ONE_AUTHOR,
  authors: [WORK, HOME],
};

/// The fixture's Repos, the first recorded as `WORK`, the second as `author`.
function recorded(second: string | null): RepoEntry[] {
  return [
    { ...REPOS[0]!, author: WORK.id },
    { ...REPOS[1]!, author: second, author_unpicked: second === null },
  ];
}

/// One row's Author dropdown, where it is drawn.
function authorOf(name: string): HTMLSelectElement | null {
  return theRow(name).querySelector<HTMLSelectElement>("select");
}

/// The repoint of one Repo, answered however the test says.
function repointing(repo: RepoEntry, answer: () => Promise<Response>) {
  return whenever(`/api/ui/repos/${repo.id}/author`, answer, "POST");
}

describe("the author of a repo", () => {
  it("is not drawn with one author configured", async () => {
    const fetching = serving(
      whenever("/api/ui/repos", json(recorded(WORK.id))),
      whenever("/api/ui/settings", json(ONE_AUTHOR)),
      whenever("/api/ui/authors", json(oneList(ONE_AUTHOR))),
    );
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    // Read the settings before saying nothing is drawn off them.
    await waitFor(() =>
      expect(askedFor(fetching, "/api/ui/authors")).toBeGreaterThan(0),
    );
    await new Promise((settle) => setTimeout(settle, 0));

    expect(authorOf(FIRST.name)).toBeNull();
    expect(screen.queryByLabelText("Author")).toBeNull();
    expect(theRow(FIRST.name).textContent).toBe(
      `${FIRST.name}${FIRST.path}Remove`,
    );
  });

  it("shows each repo's author by name once there are two", async () => {
    serving(
      whenever("/api/ui/repos", json(recorded(HOME.id))),
      whenever("/api/ui/settings", json(TWO_AUTHORS)),
      whenever("/api/ui/authors", json(oneList(TWO_AUTHORS))),
    );
    mountPane();

    await waitFor(() => expect(authorOf(FIRST.name)).not.toBeNull());

    const first = authorOf(FIRST.name)!;
    expect(first.selectedOptions[0]!.textContent).toBe("Ada Lovelace");
    expect(authorOf(REPOS[1]!.name)!.selectedOptions[0]!.textContent).toBe(
      "Home",
    );
    expect([...first.options].map((option) => option.textContent)).toEqual([
      "Ada Lovelace",
      "Home",
    ]);
  });

  it("says so where a repo has none picked, or one that is gone", async () => {
    serving(
      whenever("/api/ui/repos", json(recorded(null))),
      whenever("/api/ui/settings", json(TWO_AUTHORS)),
      whenever("/api/ui/authors", json(oneList(TWO_AUTHORS))),
    );
    const { unmount } = mountPane();

    await waitFor(() =>
      screen.getByText("No author is picked for this repo yet."),
    );
    expect(
      theRow(REPOS[1]!.name).textContent!.includes("No author is picked"),
    ).toBe(true);
    unmount();

    serving(
      whenever("/api/ui/repos", json(recorded("a-uuid-nobody-holds"))),
      whenever("/api/ui/settings", json(TWO_AUTHORS)),
      whenever("/api/ui/authors", json(oneList(TWO_AUTHORS))),
    );
    mountPane();

    await waitFor(() =>
      screen.getByText(
        "This repo's author isn't configured any more. Pick another.",
      ),
    );
  });

  /// An Author made on another device counts towards the two, a repo pointed
  /// at one is named by it with no machine, and every row offers it
  /// (ADR-0028).
  it("names an author made on another device, unmarked", async () => {
    serving(
      whenever("/api/ui/repos", json(recorded(CLIENT.id))),
      whenever("/api/ui/settings", json(ONE_AUTHOR)),
      whenever("/api/ui/authors", json(oneList(ONE_AUTHOR, CLIENT))),
    );
    mountPane();

    await waitFor(() => expect(authorOf(REPOS[1]!.name)).not.toBeNull());

    expect(
      authorOf(REPOS[1]!.name)!.selectedOptions[0]!.textContent,
    ).toBe("Client");
    expect(authorOf(FIRST.name)!.selectedOptions[0]!.textContent).toBe(
      WORK.name,
    );
    expect(
      screen.queryByText(/isn't configured any more/),
    ).toBeNull();

    // And a repo of this device's own Author can be repointed at it.
    expect(
      [...authorOf(FIRST.name)!.options].map((option) => option.text),
    ).toEqual([WORK.name, "Client"]);
  });

  /// A start on a repo whose author was deleted by hand is sent here, so the
  /// row draws its picker with one author left too — and only that row.
  it("draws the picker for a gone author with one author left", async () => {
    serving(
      whenever("/api/ui/repos", json(recorded("a-uuid-nobody-holds"))),
      whenever("/api/ui/settings", json(ONE_AUTHOR)),
      whenever("/api/ui/authors", json(oneList(ONE_AUTHOR))),
    );
    mountPane();

    await waitFor(() => expect(authorOf(REPOS[1]!.name)).not.toBeNull());
    screen.getByText(
      "This repo's author isn't configured any more. Pick another.",
    );
    expect(authorOf(FIRST.name)).toBeNull();
  });

  it("asks the server to repoint it and reads the list again", async () => {
    const fetching = serving(
      whenever("/api/ui/repos", json(recorded(HOME.id))),
      whenever("/api/ui/settings", json(TWO_AUTHORS)),
      whenever("/api/ui/authors", json(oneList(TWO_AUTHORS))),
      repointing(
        FIRST,
        json({ Repointed: { ...FIRST, author: HOME.id } }),
      ),
    );
    const { queries } = mountPane();
    const invalidated = vi.spyOn(queries, "invalidateQueries");

    await waitFor(() => expect(authorOf(FIRST.name)).not.toBeNull());
    fireEvent.change(authorOf(FIRST.name)!, { target: { value: HOME.id } });

    await waitFor(() =>
      expect(invalidated).toHaveBeenCalledWith({ queryKey: ["repos"] }),
    );
    const sent = fetching.mock.calls.find(
      ([path]) => String(path) === `/api/ui/repos/${FIRST.id}/author`,
    );
    expect(JSON.parse(String(sent![1]!.body))).toEqual({ author: HOME.id });
  });

  it("names the live work a refused repoint is about, under its row", async () => {
    serving(
      whenever("/api/ui/repos", json(recorded(HOME.id))),
      whenever("/api/ui/settings", json(TWO_AUTHORS)),
      whenever("/api/ui/authors", json(oneList(TWO_AUTHORS))),
      repointing(
        FIRST,
        json({ InUse: { on: ["rate-limiting"], beside: ["docs-pass"] } }),
      ),
    );
    mountPane();

    await waitFor(() => expect(authorOf(FIRST.name)).not.toBeNull());
    fireEvent.change(authorOf(FIRST.name)!, { target: { value: HOME.id } });

    const failure = await waitFor(() => {
      const line = theRow(FIRST.name).querySelector(`.${styles.failure}`);
      expect(line).not.toBeNull();
      return line!;
    });
    expect(failure.textContent).toBe(
      "rate-limiting is still going on this repo. docs-pass is still going " +
        "with this repo as a companion. Finish or close them before changing " +
        "the author.",
    );
    expect(
      [...failure.querySelectorAll("code")].map((code) => code.textContent),
    ).toEqual(["rate-limiting", "docs-pass"]);
    expect(
      theRow(REPOS[1]!.name).querySelector(`.${styles.failure}`),
    ).toBeNull();
  });
});

describe("the registration the dropdown draws", () => {
  /// Every way the server can turn a path away, each said in its own words: a
  /// refusal the human cannot tell from another is a refusal they cannot act
  /// on.
  ///
  /// A path registered already is not among them any more — this form is a way
  /// *onto* a repository, so that answer lands on the one somebody named, which
  /// is `composing.test.tsx`'s.
  it.each([
    ["NotARepository", /isn't a git repository/i],
    ["Missing", /nothing at that path/i],
    ["NotAbsolute", /starting with a slash/i],
    ["NoDefaultBranch", /no branch to call its default/i],
  ] satisfies Array<[RepoRefused, RegExp]>)(
    "says why a path was refused as %s",
    async (outcome, said) => {
      theRepos(json(outcome));
      const { landed } = mountOpen();

      register("/elsewhere/verkstead");

      // Beside the field, and the card still standing: a refusal is answered by
      // correcting the path, and the field keeps what was typed because that is
      // the path about to be corrected.
      await waitFor(() => screen.getByText(said));
      expect(landed).not.toHaveBeenCalled();
      expect(
        (screen.getByLabelText(/absolute path/i) as HTMLInputElement).value,
      ).toBe("/elsewhere/verkstead");
    },
  );

  it("drops the refusal as soon as the path is being changed", async () => {
    theRepos(json("NotARepository"));
    mountOpen();

    register("/elsewhere/notes");
    await waitFor(() => screen.getByText(/isn't a git repository/i));

    fireEvent.input(screen.getByLabelText(/absolute path/i), {
      target: { value: "/srv/repos/verkstead" },
    });

    expect(screen.queryByText(/isn't a git repository/i)).toBeNull();
  });

  /// A server that could not answer at all, which is the one thing here that is
  /// an error rather than an outcome.
  it("shows the server's own wording when it could not answer at all", async () => {
    theRepos(json({ error: "The repo couldn't be registered." }, 500));
    const { landed } = mountOpen();

    register("/srv/repos/verkstead");

    await waitFor(() => screen.getByText(/The repo couldn't be registered\./));
    expect(landed).not.toHaveBeenCalled();
  });

  it("sends nothing at all for an empty path", async () => {
    const fetching = theRepos();
    mountOpen();

    // The button is the guard: there is nothing to send, so there is nothing to
    // press.
    expect(
      screen.getByRole("button", { name: "Open" }).hasAttribute("disabled"),
    ).toBe(true);
    expect(
      fetching.mock.calls.filter(([, init]) => init && "method" in init),
    ).toHaveLength(0);
  });
});

describe("browsing for one", () => {
  /// What is under the directory this browse goes through — the fixture the
  /// server's own tests wrote, so what this form is filled in from is what the
  /// endpoint really answers with.
  const SRC = listing as DirectoryListing;

  /// And the home the empty field opens on, which is where the server answers
  /// an ask with no path. Written here rather than pinned to a fixture: it is
  /// a different directory on every machine, and what matters to this form is
  /// that the browse starts somewhere and walks.
  const HOME: DirectoryListing = {
    Listed: {
      path: "/home/ada",
      entries: [{ name: "src", path: "/home/ada/src", kind: "Directory" }],
    },
  };

  /// The label the field is found by, which is the one the form has always had.
  const FIELD = "Absolute path of a git repository";

  /// The two levels of the filesystem this browse goes through, and whatever
  /// the registration itself is answered by.
  function theBrowse(...answers: Array<Answer>) {
    return serving(
      whenever("/api/ui/repos", json(REPOS)),
      whenever(listingAt(null), json(HOME)),
      whenever(listingAt("/home/ada/src"), json(SRC)),
      ...answers,
    );
  }

  /// Browse the empty field down to the repository under the home, which is the
  /// whole of what this form asks of a browse.
  async function browsedToTheRepo(): Promise<void> {
    browse(FIELD);

    await waitFor(() => expect(offered(FIELD)).toContain("src"));
    tap(FIELD, "src");

    await waitFor(() => expect(offered(FIELD)).toContain("verkstead"));
    tap(FIELD, "verkstead");
  }

  /// Unbounded, because a Repo may be registered from anywhere the server can
  /// read: what the browse opens on is a starting point rather than a fence.
  it("opens on the server's home and fills the field from there", async () => {
    theBrowse();
    mountOpen();

    await browsedToTheRepo();

    expect(held(FIELD)).toBe("/home/ada/src/verkstead");
  });

  /// And marked, because a repository is what this form is being filled in
  /// with — and a leaf, because there is nothing under one it is after.
  it("marks the repository among the directories and stops there", async () => {
    theBrowse();
    mountOpen();

    browse(FIELD);
    await waitFor(() => expect(offered(FIELD)).toContain("src"));
    tap(FIELD, "src");

    await waitFor(() =>
      expect(offered(FIELD)).toEqual(["Up to /home/ada", "assets", "verkstead"]),
    );
    expect(marked(FIELD)).toEqual(["verkstead"]);

    tap(FIELD, "verkstead");

    // Where it was, filtered to what it took, rather than inside it: the browse
    // has arrived.
    expect(offered(FIELD)).toEqual(["Up to /home/ada", "verkstead"]);
  });

  /// The point of the whole component: registering a browsed path is
  /// registering a typed one. The press sends the box, and what the server makes
  /// of it goes on deciding everything.
  it("registers a browsed path exactly as a typed one", async () => {
    const fetching = theBrowse(json({ Added: REPOS[0] }));
    const { landed } = mountOpen();

    await browsedToTheRepo();
    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        "/api/ui/repos",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ path: "/home/ada/src/verkstead" }),
        }),
      ),
    );
    await waitFor(() => expect(landed).toHaveBeenCalledWith(REPOS[0]));
  });

  /// And a refusal is still the server's: what the browse offered is not a
  /// promise about what the press will do.
  it("says a refusal of a browsed path the way it says a typed one's", async () => {
    theBrowse(json("NotARepository"));
    mountOpen();

    await browsedToTheRepo();
    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => screen.getByText(/isn't a git repository/i));
    expect(held(FIELD)).toBe("/home/ada/src/verkstead");
  });
});

/// And the question the card asks first on a draft whose device is an
/// **Annex**: which machine of the pair the repository is on (ADR-0024, *Open
/// repo asks where the repository is*).
///
/// What is worth proving here is the four things the card decides for itself.
/// That the two options are labelled off the member rows rather than out of a
/// constant, because a Docker box beside a Mac has to name itself with no new
/// copy. That each side points the whole form at its own device — the browse,
/// and the registration behind it — since that is the one thing the choice
/// does. That the Keeper's side does not stop at the registration: it clones on
/// the Annex from the origin that registration reported, and what a repository
/// with no origin and a clone that failed come to is this card's to say. And
/// that a draft on anything but an Annex is asked nothing at all, which is
/// every other Open repo card in the app.
describe("the choice an Annex's draft asks first", () => {
  /// A Windows machine keeping a WSL box, which is the pair this was drawn for:
  /// this device's own row is the Keeper, and the member wearing `kept_by` is
  /// the Annex beside it.
  const KEEPER = (keeping as DevicesView).this;
  const BOX = (keeping as DevicesView).members.find(
    (member) => member.identity.kept_by !== null,
  )!.identity;

  /// The reading as those two platforms, which is what the labels are read off:
  /// the words themselves are the fixture's to say, and nothing here holds a
  /// platform name against a device by name.
  const PAIR: DevicesView = {
    ...(keeping as DevicesView),
    this: { ...KEEPER, os: "Windows" },
    members: (keeping as DevicesView).members.map((member) =>
      member.identity.device === BOX.device
        ? { ...member, identity: { ...BOX, os: "Linux (WSL)" } }
        : member,
    ),
  };

  /// And the same cluster with nothing kept: two linked devices and no Annex
  /// anywhere, which is what every other draft is on.
  const LINKED: DevicesView = {
    ...PAIR,
    members: PAIR.members.map((member) => ({
      ...member,
      identity: { ...member.identity, kept_by: null },
    })),
  };

  /// Where a call for the Annex stands, the prefix taking the place of
  /// `/api/ui` — see `src/api/client.ts`, which is the one place a path is
  /// composed.
  const onTheBox = (path: string) =>
    `/api/ui/members/${BOX.device}${path}`;

  /// The field's label, which is the one the form has always had.
  const FIELD = "Absolute path of a git repository";

  /// What each machine's own browse answers with, so that which of them a
  /// listing came from is readable off the rows.
  const IN_THE_BOX: DirectoryListing = {
    Listed: {
      path: "/home/verkstead",
      entries: [
        { name: "widgets", path: "/home/verkstead/widgets", kind: "Repository" },
      ],
    },
  };

  const ON_WINDOWS: DirectoryListing = {
    Listed: {
      path: "C:\\Users\\ada",
      entries: [
        { name: "src", path: "C:\\Users\\ada\\src", kind: "Directory" },
      ],
    },
  };

  /// The Repo the Annex's own side lands.
  const OPENED: RepoEntry = { ...FIRST, id: 71, name: "widgets" };

  /// What the Keeper's registration lands there: the Repo on that machine,
  /// carrying the `origin` the server read off the repository, which is what the
  /// clone is made from — and the Author it is recorded as, which the clone
  /// follows.
  const KEEPERS: RepoEntry = {
    ...FIRST,
    id: 12,
    name: "widgets",
    path: "C:\\Users\\ada\\src\\widgets",
    author: "7a1f",
    origin: "https://github.com/you/widgets.git",
  };

  /// And the Repo the clone leaves on the Annex, which is the draft's: a Repo id
  /// is one machine's, and the one above is the Keeper's.
  const CLONE: RepoEntry = {
    ...FIRST,
    id: 71,
    name: "widgets",
    path: "/srv/repos/widgets",
  };

  /// The card as a draft on `device` draws it, with the cluster `view` says.
  ///
  /// Behind a `Show` the way the dropdown draws it — see `repoRows` in
  /// `src/workbench/Setup.tsx` — so that a way out takes the card away rather
  /// than only being counted: what a press still running does once the card has
  /// gone is one of the things asked about below.
  function mountOn(device: Device, view: DevicesView, ...answers: Array<Answer>) {
    const [up, setUp] = createSignal(true);
    const close = vi.fn(() => setUp(false));
    const landed = vi.fn();
    const fetching = serving(
      whenever("/api/ui/devices", json(view)),
      whenever(listingAt(null), json(ON_WINDOWS)),
      whenever(listingAt(null, BOX.device), json(IN_THE_BOX)),
      // And the box's own list of Authors, which neither side draws a picker
      // off: who a repo is committed as is `the author of a repo` above, and
      // what is asked here is which machine the form is pointed at.
      whenever(onTheBox("/authors"), json([])),
      ...answers,
    );

    return {
      ...mounting(() => (
        <Show when={up()}>
          <Reaching.Provider value={() => device}>
            <OpenRepo close={close} landed={landed} />
          </Reaching.Provider>
        </Show>
      )),
      fetching,
      close,
      landed,
    };
  }

  /// The rows of the choice, by the words each reads as — none at all where
  /// there is no choice drawn.
  function sides(): string[] {
    return [
      ...document.querySelectorAll<HTMLElement>(`.${styles.where} label`),
    ].map((row) => row.textContent!.trim());
  }

  /// The radios themselves, for the one thing about them that is not their
  /// words: whether they will take a press at all.
  function radios(): HTMLInputElement[] {
    return [
      ...document.querySelectorAll<HTMLInputElement>(
        `.${styles.where} input`,
      ),
    ];
  }

  /// An answer the test lets go when it chooses, which is what a clone that is
  /// still out looks like from here. `hangs` never answers at all, and what is
  /// asked below is both halves: the card while the press is running, and what
  /// it does with the answer when it lands.
  function outstanding(body: unknown): {
    answer: () => Promise<Response>;
    settle: () => void;
  } {
    let letGo = () => {};
    const waiting = new Promise<void>((settle) => {
      letGo = settle;
    });

    return {
      answer: () =>
        waiting.then(
          () =>
            new Response(JSON.stringify(body), {
              status: 200,
              headers: { "content-type": "application/json" },
            }),
        ),
      settle: () => letGo(),
    };
  }

  /// And which of them is picked.
  function picked(): string | null {
    const on = [
      ...document.querySelectorAll<HTMLInputElement>(
        `.${styles.where} input`,
      ),
    ].find((radio) => radio.checked);

    return on?.closest("label")?.textContent?.trim() ?? null;
  }

  /// Press one of them, as a row is pressed — see `rowPress`.
  function pressing(reading: string): HTMLElement {
    const row = [
      ...document.querySelectorAll<HTMLElement>(`.${styles.where} label`),
    ].find((label) => label.textContent!.trim() === reading);

    if (!row) {
      throw new Error(`no ${reading} row: ${sides().join(", ")}`);
    }

    fireEvent.click(row);
    return row;
  }

  /// And the same press where it is meant to take, which is every case but the
  /// one about the choice being held.
  ///
  /// It refuses a row that is holding rather than pressing it, because a held
  /// row answers a press by doing nothing — see `rowPress` — and a case that
  /// pressed one a tick too early would pass whatever the card did. The choice
  /// is held for as long as a press is out, and comes back a tick after the
  /// refusal that ended one is drawn.
  function pick(reading: string): void {
    const row = pressing(reading);

    if (row.querySelector("input")!.disabled) {
      throw new Error(`the ${reading} row is held: a press is still out`);
    }
  }

  /// Type a path into the field and send it.
  function register(path: string): void {
    fireEvent.input(screen.getByLabelText(FIELD), { target: { value: path } });
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
  }

  /// And how many registrations landed on one path, which is what says a side
  /// wrote to its own machine and to no other.
  function writes(
    fetching: ReturnType<typeof serving>,
    path: string,
  ): number {
    return fetching.mock.calls.filter(
      ([asked, init]) => String(asked) === path && init?.method === "POST",
    ).length;
  }

  /// The two options, each named by its own machine's platform, with the
  /// Keeper's first — a repository cloned on Windows being the case the choice
  /// was added for.
  it("names the two machines by the platforms the member rows say", async () => {
    mountOn(BOX.device, PAIR);

    await waitFor(() => expect(sides()).toEqual(["On Windows", "On Linux"]));
    // The parenthesis on the Annex's OS word tells two rows of one hostname
    // apart on the Devices list; it is not a platform, so it is not a label.
    expect(sides()).not.toContain("On Linux (WSL)");
  });

  /// And the Annex's own side is where the card stands: it is the device the
  /// dropdown is for and the filesystem this card has always browsed, so the
  /// question is added without an answer being changed.
  it("stands on the Annex's own side", async () => {
    mountOn(BOX.device, PAIR);

    await waitFor(() => expect(picked()).toBe("On Linux"));
  });

  /// The Linux side, which is the whole of this half: the browse is the box's
  /// own filesystem — and, through the server, the login user's home, which is
  /// where a repository the human cloned by hand in that shell is — the
  /// registration lands on the box's registry, and the Repo is the draft's.
  it("browses the Annex and lands the repo on its registry", async () => {
    const { fetching, landed } = mountOn(
      BOX.device,
      PAIR,
      whenever(onTheBox("/repos"), json({ Added: OPENED }), "POST"),
    );

    await waitFor(() => expect(sides()).toHaveLength(2));

    browse(FIELD);
    await waitFor(() => expect(offered(FIELD)).toContain("widgets"));
    tap(FIELD, "widgets");
    expect(held(FIELD)).toBe("/home/verkstead/widgets");

    // The box's own directories and nothing of this machine's.
    expect(askedFor(fetching, listingAt(null, BOX.device))).toBe(1);
    expect(askedFor(fetching, listingAt(null))).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => expect(landed).toHaveBeenCalledWith(OPENED));
    expect(fetching).toHaveBeenCalledWith(
      onTheBox("/repos"),
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ path: "/home/verkstead/widgets" }),
      }),
    );
  });

  /// And the Windows side the other way round, whole: the field is the Keeper's
  /// filesystem, the press registers there — which is what makes the repository
  /// clonable everywhere else, and is the point of that half rather than a side
  /// effect — and then the Annex clones it from the origin that registration
  /// reported, as that Repo's Author, into the parent an Annex remembers. One
  /// press, no terminal.
  it("registers on the Keeper and clones on the Annex in one press", async () => {
    const { fetching, landed } = mountOn(
      BOX.device,
      PAIR,
      whenever("/api/ui/repos", json({ Added: KEEPERS }), "POST"),
      whenever(onTheBox("/repos/clone"), json({ Cloned: CLONE }), "POST"),
    );

    await waitFor(() => expect(sides()).toHaveLength(2));
    pick("On Windows");
    await waitFor(() => expect(picked()).toBe("On Windows"));

    browse(FIELD);
    await waitFor(() => expect(offered(FIELD)).toContain("src"));

    expect(askedFor(fetching, listingAt(null))).toBe(1);
    expect(askedFor(fetching, listingAt(null, BOX.device))).toBe(0);

    register("C:\\Users\\ada\\src\\widgets");

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        "/api/ui/repos",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ path: "C:\\Users\\ada\\src\\widgets" }),
        }),
      ),
    );

    // The parent off the Annex's own memory rather than out of a constant here:
    // nothing has been cloned on this box, and a box Verkstead made declares
    // `/srv/repos` its one path.
    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        onTheBox("/repos/clone"),
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({
            origin: KEEPERS.origin,
            parent: ANNEX_REPOS,
            author: KEEPERS.author,
          }),
        }),
      ),
    );

    // And the draft goes onto the clone rather than onto the Keeper's Repo.
    await waitFor(() => expect(landed).toHaveBeenCalledWith(CLONE));
    expect(landed).toHaveBeenCalledTimes(1);
  });

  /// A repository with no origin is nothing the Annex can clone, so the press is
  /// refused with the two ways round it. The registration on the Keeper stands:
  /// it is true, it is harmless, and it makes the retry a shorter one.
  it("refuses a repository with no origin and keeps the registration", async () => {
    const { fetching, landed } = mountOn(
      BOX.device,
      PAIR,
      whenever(
        "/api/ui/repos",
        json({ Added: { ...KEEPERS, origin: null } }),
        "POST",
      ),
    );

    await waitFor(() => expect(sides()).toHaveLength(2));
    pick("On Windows");
    await waitFor(() => expect(picked()).toBe("On Windows"));

    register("C:\\Users\\ada\\src\\widgets");

    const said = await waitFor(() => screen.getByText(/no origin/i));
    // Named by the Keeper's own platform, as the choice above it is.
    expect(said.textContent).toContain("work on it on Windows");

    expect(writes(fetching, "/api/ui/repos")).toBe(1);
    expect(writes(fetching, onTheBox("/repos/clone"))).toBe(0);
    expect(landed).not.toHaveBeenCalled();
  });

  /// And a clone that failed is said in git's own words, on the card, with the
  /// registration standing for the same reason.
  it("says why a clone failed and keeps the registration", async () => {
    const { fetching, landed } = mountOn(
      BOX.device,
      PAIR,
      whenever("/api/ui/repos", json({ Added: KEEPERS }), "POST"),
      whenever(
        onTheBox("/repos/clone"),
        json({ Refused: "fatal: Authentication failed." }),
        "POST",
      ),
    );

    await waitFor(() => expect(sides()).toHaveLength(2));
    pick("On Windows");
    await waitFor(() => expect(picked()).toBe("On Windows"));

    register("C:\\Users\\ada\\src\\widgets");

    await waitFor(() => screen.getByText("fatal: Authentication failed."));

    expect(writes(fetching, "/api/ui/repos")).toBe(1);
    expect(landed).not.toHaveBeenCalled();
  });

  /// What the Windows side says above its field: the clone it is about to make,
  /// and the one thing a clone from the origin leaves behind. Both machines
  /// named by their own platforms, and neither said on the Annex's own side,
  /// where nothing is cloned at all.
  it("says branches that aren't pushed stay on the Keeper's machine", async () => {
    mountOn(BOX.device, PAIR);

    await waitFor(() => expect(sides()).toHaveLength(2));
    expect(document.querySelector(`.${styles.clones}`)).toBeNull();

    pick("On Windows");

    const said = await waitFor(() => {
      const note = document.querySelector(`.${styles.clones}`);
      expect(note).toBeTruthy();
      return note!;
    });

    expect(said.textContent!.replace(/\s+/g, " ")).toBe(
      "Verkstead clones it on Linux from its origin. Branches you haven't pushed stay on Windows.",
    );
  });

  /// Moving the choice draws the form afresh: a path on one machine is nothing
  /// on the other, and a refusal about one says nothing about the other either.
  /// The choice is held while a press is out, as the dropdown holds its
  /// clonable group while a clone runs — see `repoRows` in
  /// `src/workbench/Setup.tsx`. The Keeper's press ends in a clone that takes
  /// as long as the repository is big, and the choice is what the form is drawn
  /// off: moving it under a running clone is the one gesture that would leave
  /// the press with nothing drawing it.
  it("holds the choice while a press is out", async () => {
    const clone = outstanding({ Cloned: CLONE });
    const { landed } = mountOn(
      BOX.device,
      PAIR,
      whenever("/api/ui/repos", json({ Added: KEEPERS }), "POST"),
      whenever(onTheBox("/repos/clone"), clone.answer, "POST"),
    );

    await waitFor(() => expect(sides()).toHaveLength(2));
    pick("On Windows");
    await waitFor(() => expect(picked()).toBe("On Windows"));

    register("C:\\Users\\ada\\src\\widgets");

    await waitFor(() =>
      expect(radios().every((radio) => radio.disabled)).toBe(true),
    );

    // And the row takes none either, a row offering nothing its control
    // refuses — see `rowPress`. Pressed raw, this being the one case about a
    // press that is meant not to take.
    pressing("On Linux");
    expect(picked()).toBe("On Windows");

    // Let go, and the press finishes as it always did.
    clone.settle();
    await waitFor(() => expect(landed).toHaveBeenCalledWith(CLONE));
    await waitFor(() =>
      expect(radios().every((radio) => radio.disabled)).toBe(false),
    );
  });

  /// And the one way out a press is not held off is closing the card, which
  /// leaves the clone running. What lands then is kept and what it would have
  /// moved is dropped: the Repo is registered on the Annex and the dropdown
  /// lists it, but the draft is not moved onto it minutes later, on top of
  /// whatever was done instead.
  it("keeps a clone the card was closed under and drops the draft's move", async () => {
    const clone = outstanding({ Cloned: CLONE });
    const { fetching, close, landed } = mountOn(
      BOX.device,
      PAIR,
      whenever("/api/ui/repos", json({ Added: KEEPERS }), "POST"),
      whenever(onTheBox("/repos/clone"), clone.answer, "POST"),
    );

    await waitFor(() => expect(sides()).toHaveLength(2));
    pick("On Windows");
    await waitFor(() => expect(picked()).toBe("On Windows"));

    register("C:\\Users\\ada\\src\\widgets");
    await waitFor(() => expect(writes(fetching, "/api/ui/repos")).toBe(1));

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(close).toHaveBeenCalled());

    // The clone was sent and is answered after the card has gone.
    expect(writes(fetching, onTheBox("/repos/clone"))).toBe(1);
    clone.settle();

    // Nothing to assert the absence of but the absence: the landing is given
    // every chance to arrive before it is said not to have.
    await new Promise((settle) => setTimeout(settle, 0));
    await new Promise((settle) => setTimeout(settle, 0));
    expect(landed).not.toHaveBeenCalled();
  });

  it("empties the field when the choice moves", async () => {
    mountOn(BOX.device, PAIR, json("NotARepository"));

    await waitFor(() => expect(sides()).toHaveLength(2));
    register("/home/verkstead/notes");
    await waitFor(() => screen.getByText(/isn't a git repository/i));

    // The refusal ended the press, and the choice comes back a tick after it is
    // drawn — so it is waited for rather than pressed through.
    await waitFor(() =>
      expect(radios().every((radio) => radio.disabled)).toBe(false),
    );
    pick("On Windows");

    await waitFor(() => expect(held(FIELD)).toBe(""));
    expect(screen.queryByText(/isn't a git repository/i)).toBeNull();
  });

  /// And every other draft is asked nothing: a device that is nobody's Annex,
  /// and a device in no cluster at all, each draw the card they always did.
  it.each([
    ["a member that is nobody's annex", BOX.device, LINKED],
    ["the device the browser opened", null, LINKED],
  ] satisfies Array<[string, Device, DevicesView]>)(
    "asks nothing on %s",
    async (_, device, view) => {
      mountOn(device, view);

      await waitFor(() => expect(screen.getByLabelText(FIELD)).toBeTruthy());
      // Read the cluster before saying nothing is drawn off it.
      await new Promise((settle) => setTimeout(settle, 0));

      expect(sides()).toEqual([]);
      expect(document.querySelector(`.${styles.where}`)).toBeNull();
    },
  );
});
