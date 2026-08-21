//! The Repo list and the form that adds to it, fed by the payload the server
//! actually writes.
//!
//! `tests/fixtures/repos.json` is a golden fixture like the two Set lists':
//! `cargo test` renders the real `/api/ui/repos` and writes the file, so what
//! these assertions read is the endpoint's own words.
//!
//! What is worth proving here is that a refusal reads as a refusal. The boundary
//! itself is the server's — the tests over there are what say a path outside a
//! Watched Path is turned away — and this side's whole job is to say which of
//! them happened in words the human can act on.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Registered, RepoEntry } from "../src/api/types";
import { RepoList } from "../src/repos/RepoList";
import { mount, texts } from "./listing";
import { json, serving, whenever } from "./serving";
import repos from "./fixtures/repos.json" with { type: "json" };

const REPOS = repos as RepoEntry[];
const FIRST = REPOS[0]!;

/// This page asks about updating as well as about the Repos — the banner moved
/// here when the pending list retired — and is told there is nothing to update
/// to throughout. Held to its own path rather than left in the order the
/// answers are handed out: a page with two things to fetch has no fixed order
/// between them, and the banner is `update.test.tsx`'s subject, not this file's.
const CURRENT = whenever("/api/ui/update", json("Current"));

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Type a path into the form and send it.
function register(path: string) {
  const field = screen.getByLabelText(/absolute path/i);
  fireEvent.input(field, { target: { value: path } });
  fireEvent.click(screen.getByRole("button", { name: "Register" }));
}

describe("the Repo list", () => {
  it("asks the server for the Repos it has been told about", async () => {
    const fetching = serving(CURRENT, json(REPOS));
    mount(RepoList);

    await waitFor(() => screen.getByText(FIRST.name));
    expect(fetching).toHaveBeenCalledWith("/api/ui/repos", expect.anything());
  });

  it("draws a row per Repo, with what the server said about each", async () => {
    serving(CURRENT, json(REPOS));
    mount(RepoList);

    const row = (await waitFor(() => screen.getByText(FIRST.name))).closest(
      "li",
    )!;

    // The resolved path the server recorded, not whatever was typed to register
    // it: that is the directory Verkstead will work in, and the point of showing
    // it is that it can be checked.
    expect(row.querySelector(".path")!.textContent).toBe(FIRST.path);
    expect(row.querySelector(".branch")!.textContent).toBe(FIRST.default_branch);
  });

  it("keeps the order it was given", async () => {
    serving(CURRENT, json(REPOS));
    const { container } = mount(RepoList);

    await waitFor(() => screen.getByText(FIRST.name));

    expect(texts(container, ".repo-row .title")).toEqual(
      REPOS.map((repo) => repo.name),
    );
  });

  it("says so plainly when none are registered", async () => {
    serving(CURRENT, json([]));
    mount(RepoList);

    await waitFor(() => screen.getByText("No repos are registered yet."));
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("shows the server's own wording when the list cannot be read", async () => {
    serving(CURRENT, json({ error: "the registered Repos could not be read" }, 500));
    mount(RepoList);

    await waitFor(() =>
      screen.getByText(/the registered Repos could not be read/),
    );
  });
});

describe("registering a repo", () => {
  it("sends the path that was typed, and shows what came back", async () => {
    const arrival: RepoEntry = {
      id: 7,
      name: "verkstead",
      path: "/srv/repos/verkstead",
      default_branch: "main",
    };

    const fetching = serving(CURRENT, json([]), json("Added"), json([arrival]));
    mount(RepoList);
    await waitFor(() => screen.getByText("No repos are registered yet."));

    register("/srv/repos/verkstead");

    // The path goes out as the server's own request shape, and the list is read
    // again afterwards — the repo appearing on it is the confirmation.
    await waitFor(() => screen.getByText(arrival.name));
    expect(fetching).toHaveBeenCalledWith(
      "/api/ui/repos",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ path: "/srv/repos/verkstead" }),
      }),
    );
  });

  it("empties the field once the repo is taken, so the next one starts clean", async () => {
    serving(CURRENT, json([]), json("Added"), json(REPOS));
    mount(RepoList);
    await waitFor(() => screen.getByText("No repos are registered yet."));

    register("/srv/repos/verkstead");

    await waitFor(() =>
      expect(
        (screen.getByLabelText(/absolute path/i) as HTMLInputElement).value,
      ).toBe(""),
    );
  });

  /// Every way the server can turn a path away, each said in its own words: a
  /// refusal the human cannot tell from another is a refusal they cannot act on.
  const REFUSALS: Array<[Exclude<Registered, "Added">, RegExp]> = [
    ["OutsideWatchedPaths", /outside the watched paths/i],
    ["NotARepository", /not a git repository/i],
    ["AlreadyRegistered", /registered already/i],
    ["Missing", /nothing at that path/i],
    ["NotAbsolute", /starting with a slash/i],
    ["NoDefaultBranch", /no branch to call its default/i],
  ];

  for (const [outcome, said] of REFUSALS) {
    it(`says why a path was refused as ${outcome}`, async () => {
      serving(CURRENT, json([]), json(outcome));
      mount(RepoList);
      await waitFor(() => screen.getByText("No repos are registered yet."));

      register("/elsewhere/verkstead");

      await waitFor(() => expect(screen.getByText(said)).toBeTruthy());
      // Nothing was registered, so the field keeps what was typed: it is the
      // path the human is about to correct.
      expect(
        (screen.getByLabelText(/absolute path/i) as HTMLInputElement).value,
      ).toBe("/elsewhere/verkstead");
    });
  }

  it("drops the refusal as soon as the path is being changed", async () => {
    serving(CURRENT, json([]), json("OutsideWatchedPaths"));
    mount(RepoList);
    await waitFor(() => screen.getByText("No repos are registered yet."));

    register("/elsewhere/verkstead");
    await waitFor(() => screen.getByText(/outside the watched paths/i));

    fireEvent.input(screen.getByLabelText(/absolute path/i), {
      target: { value: "/srv/repos/verkstead" },
    });

    expect(screen.queryByText(/outside the watched paths/i)).toBeNull();
  });

  it("shows the server's own wording when it could not answer at all", async () => {
    serving(CURRENT, json([]), json({ error: "the Repo could not be registered" }, 500));
    mount(RepoList);
    await waitFor(() => screen.getByText("No repos are registered yet."));

    register("/srv/repos/verkstead");

    await waitFor(() => screen.getByText(/the Repo could not be registered/));
  });

  it("sends nothing at all for an empty path", async () => {
    const fetching = serving(CURRENT, json([]));
    mount(RepoList);
    await waitFor(() => screen.getByText("No repos are registered yet."));

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
