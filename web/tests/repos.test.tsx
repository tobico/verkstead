//! The Repos section of the settings page: the cards for what is registered,
//! the pane one of them opens, and the pane that registers another.
//!
//! The three are mounted apart, because that is what they are: cards in the
//! middle pane carrying what a list is scanned for, and each pane on its own in
//! the details beside them. The page that puts them together — which path a card
//! opens, and whether it reads as open while that pane stands — is
//! `settings.test.tsx`'s, along with the arithmetic behind it.
//!
//! `tests/fixtures/repos.json` and `repo.json` are golden fixtures like the
//! profiles': `cargo test` renders the real `/api/ui/repos` and
//! `/api/ui/repos/{id}` and writes the files, so what these assertions read is
//! the endpoints' own words.
//!
//! What is worth proving here is that a refusal reads as a refusal. The boundary
//! itself is the server's — the tests over there are what say a path outside a
//! Watched Path is turned away — and this side's whole job is to say which of
//! them happened in words the human can act on, inside the pane the path is
//! about to be corrected in.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  Registered,
  RepoEntry,
  RepoView,
  SettingsView,
} from "../src/api/types";
import card from "../src/CardButton.module.css";
import button from "../src/IconButton.module.css";
import { RepoDetails, RepoList, RepoPane } from "../src/repos/RepoList";
import styles from "../src/repos/RepoList.module.css";
import head from "../src/workbench/PaneHead.module.css";
import { drawn } from "./bench";
import { json, serving, whenever } from "./serving";
import repos from "./fixtures/repos.json" with { type: "json" };
import opened from "./fixtures/repo.json" with { type: "json" };
import settings from "./fixtures/settings.json" with { type: "json" };

const REPOS = repos as RepoEntry[];
const FIRST = REPOS[0]!;

/// One of them opened, which is the pane's own read — the same repository the
/// first card is about, so a test can mount the pair and have them agree.
const OPENED: RepoView = { ...(opened as RepoView), id: FIRST.id };

/// And the settings the pane's own Sandbox Configuration is drawn out of, which
/// hold one bind for this repository by name. What that section is *about* is
/// `paths.test.tsx`'s; what is asked here is only that the pane carries it.
const SETTINGS = settings as SettingsView;

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

/// The cards in the middle pane, and what pressing one of them — or the plus
/// above them — asked for.
function mountCards(opening: number | "new" | null = null) {
  const open = vi.fn();
  const add = vi.fn();

  return {
    ...mounting(() => <RepoList opening={opening} open={open} add={add} />),
    open,
    add,
  };
}

/// The pane one of those cards opens, and what its two ways out asked for.
function mountOpened(id = FIRST.id) {
  const back = vi.fn();
  const done = vi.fn();

  return {
    ...mounting(() => <RepoDetails repo={id} back={back} done={done} />),
    back,
    done,
  };
}

/// The form in the details pane, and what its two ways out asked for.
function mountPane() {
  const back = vi.fn();
  const done = vi.fn();

  return { ...mounting(() => <RepoPane back={back} done={done} />), back, done };
}

/// The list as it stands, with whatever the pane's writes are answered by.
function theRepos(...answers: Array<() => Promise<Response>>) {
  return serving(whenever("/api/ui/repos", json(REPOS)), ...answers);
}

/// And one Repo opened, which is a read of its own rather than a row of that
/// list — with whatever the removal on it is answered by.
function theOpened(
  view: RepoView = OPENED,
  removal?: () => Promise<Response>,
) {
  return serving(
    whenever(`/api/ui/repos/${view.id}`, json(view)),
    // The pane reads the settings as well as the Repo, for the binds scoped to
    // it — the same read every other section of this page makes.
    whenever("/api/ui/settings", json(SETTINGS)),
    ...(removal
      ? [whenever(`/api/ui/repos/${view.id}/remove`, removal, "POST")]
      : []),
  );
}

/// Press the Remove that unregisters the Repo, once the read behind it has
/// landed.
///
/// By where it stands rather than by its word: the binds section above it draws
/// a Remove on every row the settings own, and both presses say the same thing
/// about two different things.
async function removePressed() {
  fireEvent.click(
    await drawn<HTMLButtonElement>(
      document,
      `.${styles.actions} .${styles.remove}`,
    ),
  );
}

/// One repo's card, by the name on it.
function theCard(name: string): HTMLElement {
  return screen.getByText(name).closest(`.${styles.repo}`)!;
}

/// Type a path into the pane and send it.
function register(path: string) {
  fireEvent.input(screen.getByLabelText(/absolute path/i), {
    target: { value: path },
  });
  fireEvent.click(screen.getByRole("button", { name: "Register" }));
}

describe("the cards", () => {
  it("asks the server for the Repos it has been told about", async () => {
    const fetching = theRepos();
    mountCards();

    await waitFor(() => screen.getByText(FIRST.name));
    expect(fetching).toHaveBeenCalledWith("/api/ui/repos", expect.anything());
  });

  /// The card's face is what a list is scanned for, which for a Repo is all
  /// three: the name it is picked by, the directory Verkstead will work in, and
  /// what a Conversation will branch from.
  ///
  /// The path shown is the resolved one the server recorded rather than whatever
  /// was typed to register it — that is the directory Verkstead will actually
  /// work in, and the point of showing it is that it can be checked.
  it("draws a card per Repo, with what the server said about each", async () => {
    theRepos();
    mountCards();

    await waitFor(() => screen.getByText(FIRST.name));

    const face = theCard(FIRST.name);
    expect(face.querySelector(`.${styles.path}`)!.textContent).toBe(FIRST.path);
    expect(face.querySelector(`.${styles.branch}`)!.textContent).toBe(
      FIRST.default_branch,
    );
  });

  it("keeps the order it was given", async () => {
    theRepos();
    const { container } = mountCards();

    await waitFor(() => screen.getByText(FIRST.name));

    expect(
      [...container.querySelectorAll(`.${styles.repo} .${styles.title}`)].map(
        (name) => name.textContent,
      ),
    ).toEqual(REPOS.map((repo) => repo.name));
  });

  /// A card is pressed to open the pane beside it, like every other card in the
  /// app: an `article` rather than a button, because it holds more than a run of
  /// text, with the press, the tab stop and the role that says what it is put on
  /// the article by `CardButton`.
  it("opens the repo when a card is pressed", async () => {
    theRepos();
    const { open } = mountCards();

    await waitFor(() => screen.getByText(FIRST.name));

    const face = theCard(FIRST.name);
    expect(face.classList).toContain(card.card);
    expect(face.getAttribute("role")).toBe("button");
    expect(face.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(face);
    expect(open).toHaveBeenCalledWith(FIRST.id);
  });

  it("reads a card as open while its pane is", async () => {
    theRepos();
    mountCards(FIRST.id);

    await waitFor(() => screen.getByText(FIRST.name));

    const face = theCard(FIRST.name);
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);

    // And the others are not: a details pane shows one thing.
    expect(theCard(REPOS[1]!.name).getAttribute("aria-pressed")).toBe("false");
  });

  /// The list is what stays in the pane. There is no form on it at all: adding
  /// one is a pane of its own now, and the modal it was drawn in is gone.
  it("keeps no form beside the cards", async () => {
    theRepos();
    const { container } = mountCards();

    await waitFor(() => screen.getByText(FIRST.name));

    expect(container.querySelector("form")).toBeNull();
    expect(container.querySelector("dialog")).toBeNull();
    expect(screen.queryByLabelText(/absolute path/i)).toBeNull();
  });

  it("says so plainly when none are registered", async () => {
    serving(whenever("/api/ui/repos", json([])));
    mountCards();

    await waitFor(() => screen.getByText("No repos are registered yet."));
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("shows the server's own wording when the list cannot be read", async () => {
    serving(
      whenever(
        "/api/ui/repos",
        json({ error: "the registered Repos could not be read" }, 500),
      ),
    );
    mountCards();

    await waitFor(() =>
      screen.getByText(/the registered Repos could not be read/),
    );
  });
});

describe("the plus that adds one", () => {
  /// An `IconButton`, for the reason the gear at the head of the conversations
  /// is one: it is another thing in the pane that is selected and opened into
  /// the pane beside it, rather than a quiet text button of its own kind.
  it("asks for the form when it is pressed", async () => {
    theRepos();
    const { container, add } = mountCards();

    const plus = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Add a repo"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("false");
    expect(plus.classList).not.toContain(button.open);

    fireEvent.click(plus);
    expect(add).toHaveBeenCalled();
  });

  it("reads as open while the form is", async () => {
    theRepos();
    const { container } = mountCards("new");

    const plus = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Add a repo"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("true");
    expect(plus.classList).toContain(button.open);
  });

  /// The plus and a card are two selections in the one pane: a Repo being open
  /// is not the form being open, and the plus says so.
  it("stays shut while a repo's own pane is open", async () => {
    theRepos();
    const { container } = mountCards(FIRST.id);

    const plus = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Add a repo"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("false");
  });
});

/// The pane a card opens: everything about a Repo the list has no room for,
/// read when somebody opens one rather than carried by the row.
describe("the pane a card opens", () => {
  /// A read of its own, keyed by the Repo: none of what it shows is on the list,
  /// so there is nothing on the list for it to have taken instead.
  it("asks the server about the one Repo it is about", async () => {
    const fetching = theOpened();
    mountOpened();

    await waitFor(() => screen.getByText(OPENED.path));
    expect(fetching).toHaveBeenCalledWith(
      `/api/ui/repos/${FIRST.id}`,
      expect.anything(),
    );
  });

  /// Titled by the repository rather than by a word, because a pane about one
  /// thing is named by that thing.
  it("is titled by the repo", async () => {
    theOpened();
    const { container } = mountOpened();

    await waitFor(() => screen.getByText(OPENED.path));
    expect(container.querySelector("h1")!.textContent).toBe(OPENED.name);
  });

  it("draws the path and the default branch", async () => {
    theOpened();
    const { container } = mountOpened();

    await waitFor(() => screen.getByText(OPENED.path));

    const facts = container.querySelector(`.${styles.facts}`)!;
    expect(facts.querySelector(`.${styles.path}`)!.textContent).toBe(
      OPENED.path,
    );
    expect(facts.querySelector(`.${styles.branch}`)!.textContent).toBe(
      OPENED.default_branch,
    );
  });

  /// Counted apart because they are read for different reasons: what is on this
  /// Repo now, and what has been.
  it("counts the live and the finished conversations apart", async () => {
    theOpened();
    const { container } = mountOpened();

    await waitFor(() => screen.getByText(OPENED.path));

    expect(container.querySelector(`.${styles.live}`)!.textContent).toBe(
      `${OPENED.live} live`,
    );
    expect(container.querySelector(`.${styles.finished}`)!.textContent).toBe(
      `${OPENED.finished} finished`,
    );
  });

  it("lists every branch git gave it, in the order it gave them", async () => {
    theOpened();
    const { container } = mountOpened();

    await waitFor(() => screen.getByText(OPENED.path));

    expect(
      [...container.querySelectorAll(`.${styles.branchList} li`)].map(
        (branch) => branch.textContent,
      ),
    ).toEqual(OPENED.branches);
  });

  /// The same reading the notice under the new-conversation box makes, said here
  /// whether or not there is any: the notice is drawn only where there is
  /// something to say, and this pane is an account of the Repo.
  it("names the roadmaps waiting in it, with the stage each would start", async () => {
    theOpened();
    const { container } = mountOpened();

    await waitFor(() => screen.getByText(OPENED.path));

    const waiting = [...container.querySelectorAll(`.${styles.roadmap}`)];
    expect(waiting).toHaveLength(OPENED.roadmaps.length);
    expect(waiting[0]!.querySelector(`.${styles.title}`)!.textContent).toBe(
      OPENED.roadmaps[0]!.title,
    );
    expect(waiting[0]!.querySelector(`.${styles.stage}`)!.textContent).toBe(
      `${OPENED.roadmaps[0]!.stage}: ${OPENED.roadmaps[0]!.stage_title}`,
    );
  });

  it("says so plainly where nothing is waiting to be adopted", async () => {
    theOpened({ ...OPENED, roadmaps: [] });
    mountOpened();

    await waitFor(() => screen.getByText("Nothing is waiting to be adopted."));
  });

  /// The one section on this pane that is settings rather than facts, carrying
  /// the binds written against this repository's own name. What it draws and
  /// what a press on it saves is `paths.test.tsx`'s; what is asked here is that
  /// the pane holds it at all, out of the settings read beside the Repo's own.
  it("carries the binds scoped to this repo", async () => {
    theOpened();
    mountOpened();

    await waitFor(() => screen.getByText("/var/cache/verkstead-cargo"));
    expect(screen.getByText("Sandbox configuration")).toBeTruthy();

    // And not the bind every sandbox gets, which is the Paths section's.
    expect(screen.queryByText("/var/cache/verkstead-node")).toBeNull();
  });

  /// A link followed after somebody took the repo away, which the server says
  /// with a 404: a line rather than an error the human is meant to act on.
  it("says the repo is gone where there is no such id", async () => {
    serving(
      whenever(
        `/api/ui/repos/${FIRST.id}`,
        json({ error: `there is no Repo ${FIRST.id}` }, 404),
      ),
    );
    mountOpened();

    await waitFor(() => screen.getByText("That repo is gone."));
    expect(screen.queryByText(/Could not read this repo/)).toBeNull();
  });

  /// And a server that could not answer at all, which is a failure rather than
  /// an absence.
  it("shows the server's own wording when it could not answer at all", async () => {
    serving(
      whenever(
        `/api/ui/repos/${FIRST.id}`,
        json({ error: "the Repo could not be read" }, 500),
      ),
    );
    mountOpened();

    await waitFor(() => screen.getByText(/the Repo could not be read/));
  });

  /// The way back out of it, in the slot every pane keeps for it: a change of
  /// level rather than a navigation, which is the page's to make.
  it("goes back to the settings", async () => {
    theOpened();
    const { container, back } = mountOpened();

    const out = await drawn<HTMLButtonElement>(container, `.${head.back}`);
    expect(out.textContent).toContain("Settings");

    fireEvent.click(out);
    expect(back).toHaveBeenCalled();
  });
});

describe("removing a repo", () => {
  /// In the pane rather than on the card: a destructive press beside a list
  /// somebody is only reading is one waiting to be made by mistake — the reason
  /// a Profile's Remove moved into its pane too.
  it("asks the server to take the repo it is about off the registry", async () => {
    const fetching = theOpened(OPENED, json("Removed"));
    const { done } = mountOpened();

    await removePressed();

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        `/api/ui/repos/${FIRST.id}/remove`,
        expect.objectContaining({ method: "POST" }),
      ),
    );
    // The pane is spent: it was about something that is not registered any
    // more, and the cards behind it are what say the removal landed.
    await waitFor(() => expect(done).toHaveBeenCalled());
  });

  /// What removing one means, said before it is done: Verkstead stops offering
  /// the repository, and nothing that was worked in it moves.
  it("says that removing it is an unregistering rather than a delete", async () => {
    theOpened();
    mountOpened();

    await waitFor(() => screen.getByText(/takes it off the registry/));
  });

  /// Refused rather than taken out from under the work going on in it, and said
  /// in the pane, because that is where the press was made.
  it("says why a repo with live work on it could not be removed", async () => {
    theOpened(OPENED, json("InUse"));
    const { done } = mountOpened();

    await removePressed();

    await waitFor(() =>
      screen.getByText(/A conversation that is still going is on it/),
    );
    expect(done).not.toHaveBeenCalled();
  });

  /// A pane left open in another tab, whose repo somebody has already taken
  /// away: a refusal in words rather than a failure.
  it("says so where the repo is off the registry already", async () => {
    theOpened(OPENED, json("NoSuchRepo"));
    const { done } = mountOpened();

    await removePressed();

    await waitFor(() => screen.getByText(/off the registry already/));
    expect(done).not.toHaveBeenCalled();
  });

  /// And a server that could not answer at all, which is the one thing here
  /// that is an error rather than an outcome.
  it("says so when the server could not answer", async () => {
    theOpened(OPENED, json({ error: "the Repo could not be removed" }, 500));
    mountOpened();

    await removePressed();

    await waitFor(() => screen.getByText(/could not be removed/));
  });
});

describe("the pane the plus opens", () => {
  /// Blank, and standing on nothing the server has said: registering a Repo is
  /// naming a path, and the form does not wait on a read it has no use for.
  it("opens empty", async () => {
    theRepos();
    const { container } = mountPane();

    expect(
      (screen.getByLabelText(/absolute path/i) as HTMLInputElement).value,
    ).toBe("");
    // A pane rather than a modal, with no second way out: a details pane is
    // left by opening something else or by the way back its head draws.
    expect(container.querySelector("dialog")).toBeNull();
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
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

  it("sends the path that was typed", async () => {
    const fetching = theRepos(json("Added"));
    mountPane();

    register("/srv/repos/verkstead");

    // The path goes out as the server's own request shape.
    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        "/api/ui/repos",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ path: "/srv/repos/verkstead" }),
        }),
      ),
    );
  });

  /// A pane that has been spent: what says the registration landed is the card
  /// behind it, which is where the human is put back.
  it("spends the pane once the server took the path", async () => {
    theRepos(json("Added"));
    const { done } = mountPane();

    register("/srv/repos/verkstead");

    await waitFor(() => expect(done).toHaveBeenCalled());
  });

  /// And the roadmaps waiting are read again with the list, as they are when a
  /// repo is removed. The offers are drawn from whatever is registered, in the
  /// conversations pane standing right beside this one — so a repository
  /// arriving with an unadopted roadmap in it, or a taken-away path registered
  /// again, has something to offer the moment it lands.
  it("reads the roadmap offers again with the list", async () => {
    theRepos(json("Added"));
    const { queries, done } = mountPane();
    const invalidated = vi.spyOn(queries, "invalidateQueries");

    register("/srv/repos/verkstead");

    await waitFor(() => expect(done).toHaveBeenCalled());
    expect(invalidated).toHaveBeenCalledWith({ queryKey: ["repos"] });
    expect(invalidated).toHaveBeenCalledWith({
      queryKey: ["abandoned-roadmaps"],
    });
  });

  /// Every way the server can turn a path away, each said in its own words: a
  /// refusal the human cannot tell from another is a refusal they cannot act on.
  it.each([
    ["OutsideWatchedPaths", /outside the watched paths/i],
    ["NotARepository", /not a git repository/i],
    ["AlreadyRegistered", /registered already/i],
    ["Missing", /nothing at that path/i],
    ["NotAbsolute", /starting with a slash/i],
    ["NoDefaultBranch", /no branch to call its default/i],
  ] satisfies Array<[Exclude<Registered, "Added">, RegExp]>)(
    "says why a path was refused as %s",
    async (outcome, said) => {
      theRepos(json(outcome));
      const { done } = mountPane();

      register("/elsewhere/verkstead");

      // Beside the field, and the pane still standing: a refusal is answered by
      // correcting the path, and the field keeps what was typed because that is
      // the path about to be corrected.
      await waitFor(() => screen.getByText(said));
      expect(done).not.toHaveBeenCalled();
      expect(
        (screen.getByLabelText(/absolute path/i) as HTMLInputElement).value,
      ).toBe("/elsewhere/verkstead");
    },
  );

  it("drops the refusal as soon as the path is being changed", async () => {
    theRepos(json("OutsideWatchedPaths"));
    mountPane();

    register("/elsewhere/verkstead");
    await waitFor(() => screen.getByText(/outside the watched paths/i));

    fireEvent.input(screen.getByLabelText(/absolute path/i), {
      target: { value: "/srv/repos/verkstead" },
    });

    expect(screen.queryByText(/outside the watched paths/i)).toBeNull();
  });

  /// A server that could not answer at all, which is the one thing here that is
  /// an error rather than an outcome.
  it("shows the server's own wording when it could not answer at all", async () => {
    theRepos(json({ error: "the Repo could not be registered" }, 500));
    const { done } = mountPane();

    register("/srv/repos/verkstead");

    await waitFor(() => screen.getByText(/the Repo could not be registered/));
    expect(done).not.toHaveBeenCalled();
  });

  it("sends nothing at all for an empty path", async () => {
    const fetching = theRepos();
    mountPane();

    // The button is the guard: there is nothing to send, so there is nothing to
    // press.
    expect(
      screen.getByRole("button", { name: "Register" }).hasAttribute("disabled"),
    ).toBe(true);
    expect(
      fetching.mock.calls.filter(([, init]) => init && "method" in init),
    ).toHaveLength(0);
  });
});
