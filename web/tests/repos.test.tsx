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
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { DirectoryListing, RepoEntry } from "../src/api/types";
import card from "../src/CardButton.module.css";
import {
  OpenRepo,
  ReposCard,
  ReposPane,
  type RepoRefused,
} from "../src/repos/RepoList";
import styles from "../src/repos/RepoList.module.css";
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
import { json, serving, whenever, type Answer } from "./serving";
import repos from "./fixtures/repos.json" with { type: "json" };
import listing from "./fixtures/directories.json" with { type: "json" };

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

    await waitFor(() => screen.getByText(/registered/));
    expect(fetching).toHaveBeenCalledWith("/api/ui/repos", expect.anything());
  });

  /// A count is the whole of what the card says: every item on the list it
  /// opens is a name, and there is nothing else about one to scan for.
  it("says how many are registered", async () => {
    theRepos();
    const { container } = mountCard();

    await waitFor(() =>
      expect(
        container.querySelector(`.${styles.standing}`)!.textContent,
      ).toBe(`${REPOS.length} repos registered.`),
    );
    expect(container.querySelector("h2")!.textContent).toBe("Repos");
  });

  /// In English rather than as `1 repos`.
  it("counts one of them in the singular", async () => {
    serving(whenever("/api/ui/repos", json([FIRST])));
    mountCard();

    await waitFor(() => screen.getByText("1 repo registered."));
  });

  it("says so plainly when none are registered", async () => {
    serving(whenever("/api/ui/repos", json([])));
    mountCard();

    await waitFor(() => screen.getByText("No repos are registered yet."));
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
        json({ error: "the registered Repos could not be read" }, 500),
      ),
    );
    mountCard();

    await waitFor(() =>
      screen.getByText(/the registered Repos could not be read/),
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

  /// And the path is on the row because the name is not unique.
  ///
  /// A name is the directory's own and nothing stops two registered
  /// repositories in different places sharing one — the store orders its list
  /// by `name, id` for that reason. This is the one list that puts them side by
  /// side, with an unregistering behind each, so two rows that read the same
  /// would be a press somebody could only get right by luck.
  it("tells two repos of one name apart", async () => {
    const twins: RepoEntry[] = [
      { id: 3, name: "api", path: "/srv/repos/api", default_branch: "main" },
      { id: 4, name: "api", path: "/work/api", default_branch: "main" },
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

    await waitFor(() => screen.getByText("No repos are registered yet."));
    expect(container.querySelector(`.${styles.repo}`)).toBeNull();
  });

  it("shows the server's own wording when the list cannot be read", async () => {
    serving(
      whenever(
        "/api/ui/repos",
        json({ error: "the registered Repos could not be read" }, 500),
      ),
    );
    mountPane();

    await waitFor(() =>
      screen.getByText(/the registered Repos could not be read/),
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
      "Keep it",
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
      screen.getByText(/A conversation that is still going is on it/),
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
      removing(FIRST, json({ error: "the Repo could not be removed" }, 500)),
    );
    mountPane();

    await waitFor(() => screen.getByText(FIRST.name));
    removePressed(FIRST.name);
    await confirmed();

    await waitFor(() => screen.getByText(/could not be removed/));
    expect(theRow(FIRST.name).querySelector(`.${styles.failure}`)).toBeTruthy();
  });
});

/// The registration, which has left the settings: it is asked from the Repo
/// dropdown, over the **Open repo** modal, and nothing on the settings page
/// registers a repository at all.
describe("the registration the dropdown draws", () => {
  /// Every way the server can turn a path away, each said in its own words: a
  /// refusal the human cannot tell from another is a refusal they cannot act
  /// on.
  ///
  /// A path registered already is not among them any more — this form is a way
  /// *onto* a repository, so that answer lands on the one somebody named, which
  /// is `composing.test.tsx`'s.
  it.each([
    ["NotARepository", /not a git repository/i],
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
    await waitFor(() => screen.getByText(/not a git repository/i));

    fireEvent.input(screen.getByLabelText(/absolute path/i), {
      target: { value: "/srv/repos/verkstead" },
    });

    expect(screen.queryByText(/not a git repository/i)).toBeNull();
  });

  /// A server that could not answer at all, which is the one thing here that is
  /// an error rather than an outcome.
  it("shows the server's own wording when it could not answer at all", async () => {
    theRepos(json({ error: "the Repo could not be registered" }, 500));
    const { landed } = mountOpen();

    register("/srv/repos/verkstead");

    await waitFor(() => screen.getByText(/the Repo could not be registered/));
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

    await waitFor(() => screen.getByText(/not a git repository/i));
    expect(held(FIELD)).toBe("/home/ada/src/verkstead");
  });
});
