//! The credentials at the top of the settings page: the GitHub token every
//! session is handed, and who its commits are by.
//!
//! Mounted on its own rather than through the page it heads, because the page
//! holds two more sections whose own files say what they do — and a "Save" that
//! could be either form's is a test asserting the page's arrangement rather
//! than the credentials. What the page holds, and what became of the two routes
//! it holds it instead of, is the last describe here.
//!
//! What the page shows is a summary and what rewrites it is a modal, so this
//! suite is in two halves: what is readable without editing — the token's state,
//! the author, and the warnings about whichever is missing — and what the form
//! does once it is opened. What a modal *is* — the dialog, Escape, a press away
//! from the card — belongs to `Modal` and is read in `modals.test.tsx`.
//!
//! `tests/fixtures/settings*.json` are golden fixtures like the profiles page's:
//! `cargo test` calls the real endpoint and writes the files, so what these
//! assertions read is what the server actually said — the unset state a fresh
//! install opens in, the settings as they stand once both are told, and the
//! answer to a save with the account GitHub verified the token as.
//!
//! What a token really verifies as, and that it is written down either way, are
//! the server's to decide — `crates/server/tests/settings.rs` is what says so.
//! This side's job is to send what was typed, to say in words what came back,
//! and never to show the token again.

import { MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type {
  ProfileEntry,
  RepoEntry,
  SettingsSaved,
  SettingsView,
} from "../src/api/types";
import { Credentials } from "../src/settings/Credentials";
import { SettingsPage } from "../src/settings/SettingsPage";
import { json, serving, whenever } from "./serving";
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };
import saved from "./fixtures/settings-saved.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const PROFILES = profiles as ProfileEntry[];
const REPOS = repos as RepoEntry[];
const UNSET = unset as SettingsView;
const SAVED = saved as SettingsSaved;

afterEach(() => {
  vi.unstubAllGlobals();
});

function mount() {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <Route path="/" component={Credentials} />
      </MemoryRouter>
    </QueryClientProvider>
  ));
}

/// The settings as they stand, with whatever the page's save is answered by.
function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// Open the form, which is what the button on the heading does.
function edit() {
  fireEvent.click(screen.getByRole("button", { name: "Edit" }));
}

/// The form, or nothing at all where it has not been opened.
function theForm(container: ParentNode): HTMLDialogElement | null {
  return container.querySelector<HTMLDialogElement>("dialog.edit-credentials");
}

/// The body the page put on the wire when it saved.
function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

const TOKEN = "ghp_fedcba9876543210";

describe("the settings as they stand", () => {
  it("says of a saved token its last four characters and when it was written", async () => {
    theSettings(TOLD);
    const { container } = mount();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(container.querySelector(".last-four")!.textContent).toBe(
      TOLD.github_token!.last_four,
    );
    expect(container.querySelector(".when")!.textContent).toBe(
      "2026-08-03 09:07 UTC",
    );
  });

  /// The author is a line rather than two fields, because reading it is what the
  /// page is for and rewriting it is the form's.
  it("says who commits are by", async () => {
    theSettings(TOLD);
    const { container } = mount();

    await waitFor(() => screen.getByText(/Commits are by/));

    expect(container.querySelector(".author-name")!.textContent).toBe(
      TOLD.git_author.name,
    );
    expect(container.querySelector(".author-email")!.textContent).toContain(
      TOLD.git_author.email,
    );
  });

  /// The one promise this page makes: the whole token is not in the payload it
  /// is drawn from, so it cannot be in the page — and the field to type one into
  /// starts empty rather than prefilled with anything.
  it("never puts a token in the field", async () => {
    theSettings(TOLD);
    mount();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    edit();
    fireEvent.click(screen.getByRole("button", { name: "Replace" }));

    expect(
      (screen.getByLabelText(/Token, pasted/) as HTMLInputElement).value,
    ).toBe("");
  });

  it("fills the author fields in with what is configured", async () => {
    theSettings(TOLD);
    mount();

    await waitFor(() => screen.getByText(/Commits are by/));
    edit();

    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
      TOLD.git_author.name,
    );
    expect((screen.getByLabelText("Email") as HTMLInputElement).value).toBe(
      TOLD.git_author.email,
    );
  });

  it("says so when the server could not be read at all", async () => {
    serving(whenever("/api/ui/settings", json({ error: "gone" }, 500)));
    mount();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("what is not configured", () => {
  /// Said here rather than found out by a session that could not push at
  /// midnight: neither of these is read out of a home directory any more. On the
  /// page rather than in the form, because whoever needs to read them is
  /// precisely whoever is not editing.
  it("warns that sessions cannot reach GitHub with no token", async () => {
    theSettings(UNSET);
    mount();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
  });

  it("warns that commits fail with no author", async () => {
    theSettings(UNSET);
    mount();

    await waitFor(() =>
      screen.getByText(/commits inside a session fail asking who the author is/),
    );
  });

  /// And says nothing else about an author nobody has given: the warning is the
  /// whole of it, where a line naming the halves would be naming two blanks.
  it("says nothing of an author with neither half", async () => {
    theSettings(UNSET);
    mount();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    expect(screen.queryByText(/Commits are by/)).toBeNull();
  });

  /// Half an author is as broken as none: git complains by name about whichever
  /// half it has not been given.
  it("warns about an author with only half of one", async () => {
    theSettings({ ...TOLD, git_author: { name: "Ada Lovelace", email: "" } });
    mount();

    await waitFor(() =>
      screen.getByText(/commits inside a session fail asking who the author is/),
    );
    // And still says the half there is, which is the half being checked.
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
  });

  it("says neither where both are configured", async () => {
    theSettings(TOLD);
    mount();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(screen.queryByText(/sessions cannot reach GitHub/)).toBeNull();
    expect(screen.queryByText(/commits inside a session fail/)).toBeNull();
  });

  /// With nothing configured the field is the form rather than a detour from it:
  /// there is no token to replace, so there is nothing to press first.
  it("opens the token field straight away", async () => {
    theSettings(UNSET);
    mount();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    edit();

    expect(screen.getByLabelText(/Token, pasted/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Replace" })).toBeNull();
  });

  it("clears both warnings once the settings land", async () => {
    theSettings(UNSET, json(SAVED));
    mount();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    edit();

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: SAVED.settings.git_author.name },
    });
    fireEvent.input(screen.getByLabelText("Email"), {
      target: { value: SAVED.settings.git_author.email },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    // The settings landing is what clears them, so it is what to wait for: an
    // assertion that they are absent would pass while the save was still out.
    await waitFor(() => screen.getByText(/A token ending/));

    expect(screen.queryByText(/sessions cannot reach GitHub/)).toBeNull();
    expect(screen.queryByText(/commits inside a session fail/)).toBeNull();
  });
});

describe("the form", () => {
  /// Nothing of it is on the page until it is asked for: what the page is for is
  /// reading what is configured, and this is settled once and then left alone.
  it("is a modal, and nothing at all until the edit button is pressed", async () => {
    theSettings(TOLD);
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(theForm(container)).toBeNull();

    edit();

    expect(theForm(container)!.open, "opened as a modal").toBe(true);
    // Both sections, under the one Save the server's one request deserves.
    expect(screen.getByLabelText("Name")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Replace" })).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Save" })).toHaveLength(1);
  });

  it("goes away once the settings are saved", async () => {
    theSettings(TOLD, json(SAVED));
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(theForm(container)).toBeNull());
  });

  /// Cancel saves nothing, and what was typed goes with it: a form opened again
  /// follows what the files say now rather than a draft nothing promised to
  /// keep.
  it("saves nothing when it is cancelled, and opens afresh after", async () => {
    const fetching = theSettings(TOLD);
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    edit();
    fireEvent.input(screen.getByLabelText("Email"), {
      target: { value: "typed@nowhere" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(theForm(container)).toBeNull());
    expect(
      fetching.mock.calls.filter(([, init]) => init?.method === "POST"),
    ).toHaveLength(0);

    edit();
    expect((screen.getByLabelText("Email") as HTMLInputElement).value).toBe(
      TOLD.git_author.email,
    );
  });
});

describe("saving", () => {
  it("sends the author fields as they were typed", async () => {
    const fetching = theSettings(TOLD, json(SAVED));
    mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    fireEvent.input(screen.getByLabelText("Email"), {
      target: { value: "ada@analytical.engine" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: {
          name: TOLD.git_author.name,
          email: "ada@analytical.engine",
        },
        github_token: "Keep",
      }),
    );
  });

  /// The author round-trips onto the summary: what the save answered with is
  /// what the page goes back to showing, because that is a fresh read of the
  /// files rather than an echo of what was sent.
  it("shows on the summary the author the save came back with", async () => {
    const rewritten: SettingsSaved = {
      settings: {
        ...SAVED.settings,
        git_author: { name: "Ada Lovelace", email: "ada@analytical.engine" },
      },
      verified: null,
    };
    theSettings(TOLD, json(rewritten));
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    fireEvent.input(screen.getByLabelText("Email"), {
      target: { value: "ada@analytical.engine" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(
        container.querySelector(".author-email")!.textContent,
      ).toContain("ada@analytical.engine"),
    );
  });

  /// A write-only field left blank means *leave it alone*: a page that read one
  /// as *clear this* would take the credentials away every time somebody
  /// corrected their own email address.
  it("keeps the configured token when the field was not filled in", async () => {
    const fetching = theSettings(UNSET, json(SAVED));
    mount();
    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    edit();

    fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "Ada Lovelace" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect((sent(fetching) as { github_token: unknown }).github_token).toBe(
        "Keep",
      ),
    );
  });

  it("sends a token that was typed, whole", async () => {
    const fetching = theSettings(UNSET, json(SAVED));
    mount();
    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    edit();

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect((sent(fetching) as { github_token: unknown }).github_token).toEqual(
        { Set: { token: TOKEN } },
      ),
    );
  });

  /// Said on the summary rather than in the form, because the form is gone by
  /// the time there is an answer to say.
  it("says which account GitHub verified the token as", async () => {
    theSettings(UNSET, json(SAVED));
    const { container } = mount();
    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    edit();

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/GitHub says it is/));
    expect(container.querySelector(".login")!.textContent).toBe("ada");
  });

  /// A token GitHub would not vouch for is saved all the same — it is pasted
  /// once out of a page that will not show it again — so the page says both
  /// halves: what is stored, and that nobody could be asked whose it is.
  it("says in words why a token could not be verified, and shows it stored", async () => {
    const unverified: SettingsSaved = {
      settings: SAVED.settings,
      verified: { Refused: { why: "gh: Bad credentials (HTTP 401)" } },
    };
    theSettings(UNSET, json(unverified));
    mount();
    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    edit();

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/Bad credentials/));
    await waitFor(() => screen.getByText(/A token ending/));
  });

  /// A server that could not write the files, which is the one thing here that
  /// is an error rather than an answer — and the one that keeps the form up,
  /// because nothing has been saved to go back to reading.
  it("says so when the settings could not be saved, and keeps the form up", async () => {
    theSettings(TOLD, json({ error: "the disk is full" }, 503));
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/could not be saved/));
    expect(theForm(container)!.open).toBe(true);
  });
});

describe("replacing and clearing the token", () => {
  /// The field to paste a credential into is one somebody asked for: with a
  /// token already configured, replacing is a press. And the way back from it is
  /// named for what it does, because "cancel" in this form is the way out of the
  /// whole of it.
  it("opens the field on the press, and closes it again on the way back", async () => {
    theSettings(TOLD);
    mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    expect(screen.queryByLabelText(/Token, pasted/)).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Replace" }));
    await waitFor(() => screen.getByLabelText(/Token, pasted/));

    fireEvent.click(
      screen.getByRole("button", { name: "Keep the saved token" }),
    );
    await waitFor(() =>
      expect(screen.queryByLabelText(/Token, pasted/)).toBeNull(),
    );
  });

  it("sends the replacement, and spends the field", async () => {
    const fetching = theSettings(TOLD, json(SAVED));
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    fireEvent.click(screen.getByRole("button", { name: "Replace" }));
    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect((sent(fetching) as { github_token: unknown }).github_token).toEqual(
        { Set: { token: TOKEN } },
      ),
    );
    // And the field is spent with the form it was in: the standing line on the
    // summary is the confirmation.
    await waitFor(() => expect(theForm(container)).toBeNull());

    edit();
    expect(screen.queryByLabelText(/Token, pasted/)).toBeNull();
  });

  /// Clearing is its own press for the reason a blank field is a `Keep`: the two
  /// cannot be the same gesture. The author fields ride along, because the
  /// server writes both files in one request.
  it("clears the token without taking the author with it", async () => {
    const cleared: SettingsSaved = {
      settings: { git_author: TOLD.git_author, github_token: null },
      verified: null,
    };
    const fetching = theSettings(TOLD, json(cleared));
    const { container } = mount();
    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    edit();

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Clear",
      }),
    );

    // And the page is the unset one again, warning and all, with the form gone:
    // clearing is a save, and a save is what takes the form away.
    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    expect(theForm(container)).toBeNull();
    expect(container.querySelector(".author-name")!.textContent).toBe(
      TOLD.git_author.name,
    );
  });
});

/// The page the credentials head, with the two lists folded onto it below them
/// and nothing to update to.
function thePage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  serving(
    whenever("/api/ui/settings", json(TOLD)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/update", json("Current")),
  );

  return render(() => (
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <Route path="/" component={SettingsPage} />
      </MemoryRouter>
    </QueryClientProvider>
  ));
}

describe("the settings page", () => {
  /// One page for everything the human configures: what Verkstead itself was
  /// told, and the two things a Conversation is settled against.
  it("holds the credentials, the profiles and the repos", async () => {
    const { container } = thePage();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));
    await waitFor(() => screen.getByText(PROFILES[0]!.name));
    await waitFor(() => screen.getByText(REPOS[0]!.name));

    // Each is a section of the one page rather than a page of its own: there is
    // one heading over the lot and one way back to the workbench.
    const page = container.querySelector(".list-page")!;
    expect(page.querySelectorAll("h1")).toHaveLength(1);
    expect(page.querySelectorAll(".back")).toHaveLength(1);
  });

  /// Summaries, lists and buttons: every form on this page is a modal now, so
  /// what the page itself carries is the way into each of them and none of their
  /// fields. What each does with what is typed is `repos.test.tsx`'s,
  /// `profiles.test.tsx`'s and this file's own.
  it("keeps every list's rows, and the way into every form", async () => {
    const { container } = thePage();

    await waitFor(() => screen.getByText(REPOS[0]!.name));

    for (const section of [".credentials", ".profiles", ".repos"]) {
      expect(
        container.querySelector(`${section} > .section-head > button`),
        `expected ${section} to head its own form`,
      ).not.toBeNull();
    }

    expect(container.querySelectorAll(".repos .repo-row")).toHaveLength(
      REPOS.length,
    );
    expect(container.querySelectorAll(".profiles .profile-row")).toHaveLength(
      PROFILES.length,
    );

    // And nothing of any of the three forms until one is asked for.
    expect(container.querySelector("dialog")).toBeNull();
    expect(container.querySelectorAll("input")).toHaveLength(
      // The notifications switch on the heading's line, which is not a form.
      container.querySelectorAll(".page-head input").length,
    );
  });

  /// The switch about this device came with the Repos it was on, and it kept
  /// the heading's line it was on there. The banner about this server came with
  /// it; where that sits is `update.test.tsx`'s.
  it("keeps the notifications switch on the heading's line", async () => {
    const { container } = thePage();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(container.querySelector(".page-head .notifications")).not.toBeNull();
  });
});

describe("the routes the fold retired", () => {
  /// Not redirected: the two paths were pages, and they are not pages any more.
  /// A redirect would be a third opinion about where the Repos live, kept alive
  /// for a bookmark nobody has.
  for (const path of ["/repos", "/profiles"]) {
    it(`answers with the no-such-page fallback at ${path}`, async () => {
      window.history.pushState({}, "", path);
      serving(json([]));

      render(() => <App />);

      await waitFor(() => screen.getByText("No such page."));
    });
  }
});
