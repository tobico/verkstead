//! The credentials at the head of the settings pane: the GitHub token every
//! session is handed, and who its commits are by. And the settings page they
//! head, which stands on the same three panes the workbench does.
//!
//! The two halves are mounted apart, because that is what they are now: a card
//! in the middle pane carrying what is configured, and the form that rewrites it
//! in the details pane beside it. Each is mounted on its own — the page holds
//! two more sections whose own files say what they do, and a "Save" that could
//! be either form's is a test asserting the page's arrangement rather than the
//! credentials — and the pair is mounted together only where a round trip is
//! what is being asked about: what the form saved, said back on the card.
//!
//! So this suite is in three parts: what is readable on the card without
//! opening anything — the token's state, the author, and the warnings about
//! whichever is missing — what the form does in its pane, and what the page
//! itself draws around the two.
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

import {
  MemoryRouter,
  Route,
  createMemoryHistory,
} from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type {
  ConversationEntry,
  ProfileEntry,
  RepoEntry,
  RepoView,
  SettingsSaved,
  SettingsView,
  ShowingArchived,
} from "../src/api/types";
import button from "../src/IconButton.module.css";
import shell from "../src/Panes.module.css";
import profileList from "../src/profiles/ProfileList.module.css";
import notifications from "../src/push/Notifications.module.css";
import repoList from "../src/repos/RepoList.module.css";
import card from "../src/CardButton.module.css";
import { GithubCard, GithubPane } from "../src/settings/Credentials";
import styles from "../src/settings/Credentials.module.css";
import buildCache from "../src/settings/BuildCache.module.css";
import shareViewer from "../src/settings/ShareViewer.module.css";
import paths from "../src/settings/Paths.module.css";
import {
  SettingsPage,
  panes as settingsPanes,
} from "../src/settings/SettingsPage";
import {
  openingAt,
  opensProfile,
  opensRepo,
  pathTo,
  profileOpened,
  repoOpened,
} from "../src/settings/openings";
import head from "../src/workbench/PaneHead.module.css";
import { drawn } from "./bench";
import { json, serving, whenever } from "./serving";
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import repo from "./fixtures/repo.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };
import saved from "./fixtures/settings-saved.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;

/// The paths the fixture holds, as a save puts them back on the wire: the
/// settings' own entries, and a Repo's bind in the `name=path` grammar the file
/// keeps them in. Every section's save carries them, because one request writes
/// the whole of `config.yaml` — a list left out would be a list emptied.
const PATHS = {
  watched_paths: ["/home/ada/src"],
  sandbox_binds: [
    "/var/cache/verkstead-node",
    "verkstead=/var/cache/verkstead-cargo",
  ],
};
const PROFILES = profiles as ProfileEntry[];
const REPOS = repos as RepoEntry[];
const FIRST_REPO = REPOS[0]!;
/// The Repo the fixture opens, which is the first of the list it belongs to.
const OPENED: RepoView = { ...(repo as RepoView), id: FIRST_REPO.id };
const SIDEBAR = conversations as ConversationEntry[];
const UNSET = unset as SettingsView;
const SAVED = saved as SettingsSaved;

/// The sidebar's one setting, off — which is where a workbench nobody has
/// archived anything in stands.
const HIDING_ARCHIVED: ShowingArchived = { showing: false };

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

/// Whatever half of the credentials a test is about, over one query client:
/// both halves read the same two files, so a test mounting the pair is reading
/// them once, exactly as the page does.
function mounting(what: () => JSX.Element) {
  const queries = client();

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

/// The card in the middle pane, and what pressing it asked for.
function mountCard(open = false) {
  const press = vi.fn();
  return { ...mounting(() => <GithubCard open={open} press={press} />), press };
}

/// The form in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <GithubPane back={back} />), back };
}

/// Both, as the page has them while the pane is open: what the form saves is
/// what the card goes back to showing.
function mountBoth() {
  return mounting(() => (
    <>
      <GithubCard open press={() => {}} />
      <GithubPane back={() => {}} />
    </>
  ));
}

/// The settings as they stand, with whatever the save is answered by.
function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
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

describe("the card", () => {
  it("says of a saved token its last four characters and when it was written", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(container.querySelector(`.${styles.lastFour}`)!.textContent).toBe(
      TOLD.github_token!.last_four,
    );
    expect(container.querySelector(`.${styles.tokenStanding} span`)!.textContent).toBe(
      "2026-08-03 09:07 UTC",
    );
  });

  /// The author is a line rather than two fields, because reading it is what the
  /// card is for and rewriting it is the pane's.
  it("says who commits are by", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/Commits are by/));

    expect(container.querySelector(`.${styles.authorName}`)!.textContent).toBe(
      TOLD.git_author.name,
    );
    expect(container.querySelector(`.${styles.authorEmail}`)!.textContent).toContain(
      TOLD.git_author.email,
    );
  });

  /// It is a card like every other card in the app: pressed to open the pane
  /// beside it, and drawn as the open one while that pane is what is being
  /// read. An `article` rather than a button, because it holds paragraphs.
  it("opens the pane when it is pressed", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    const face = await drawn<HTMLElement>(container, `.${styles.githubCard}`);
    expect(face.getAttribute("role")).toBe("button");
    expect(face.getAttribute("aria-pressed")).toBe("false");
    expect(face.classList).not.toContain(card.open);

    fireEvent.click(face);
    expect(press).toHaveBeenCalled();
  });

  it("reads as open while its pane is", async () => {
    theSettings(TOLD);
    const { container } = mountCard(true);

    const face = await drawn<HTMLElement>(container, `.${styles.githubCard}`);
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);
  });

  it("says so when the server could not be read at all", async () => {
    serving(whenever("/api/ui/settings", json({ error: "gone" }, 500)));
    mountCard();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("what is not configured", () => {
  /// Said here rather than found out by a session that could not push at
  /// midnight: neither of these is read out of a home directory any more. On the
  /// card rather than in the form, because whoever needs to read them is
  /// precisely whoever is not editing.
  it("warns that sessions cannot reach GitHub with no token", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
  });

  it("warns that commits fail with no author", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() =>
      screen.getByText(/commits inside a session fail asking who the author is/),
    );
  });

  /// And says nothing else about an author nobody has given: the warning is the
  /// whole of it, where a line naming the halves would be naming two blanks.
  it("says nothing of an author with neither half", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    expect(screen.queryByText(/Commits are by/)).toBeNull();
  });

  /// Half an author is as broken as none: git complains by name about whichever
  /// half it has not been given.
  it("warns about an author with only half of one", async () => {
    theSettings({ ...TOLD, git_author: { name: "Ada Lovelace", email: "" } });
    mountCard();

    await waitFor(() =>
      screen.getByText(/commits inside a session fail asking who the author is/),
    );
    // And still says the half there is, which is the half being checked.
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
  });

  it("says neither where both are configured", async () => {
    theSettings(TOLD);
    mountCard();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(screen.queryByText(/sessions cannot reach GitHub/)).toBeNull();
    expect(screen.queryByText(/commits inside a session fail/)).toBeNull();
  });

  /// With nothing configured the field is the form rather than a detour from it:
  /// there is no token to replace, so there is nothing to press first.
  it("opens the token field straight away", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByLabelText(/Token, pasted/));
    expect(screen.queryByRole("button", { name: "Replace" })).toBeNull();
  });

  /// The round trip: what the pane saved is what the card goes back to showing,
  /// because the answer to a save is a fresh read of the two files.
  it("clears both warnings on the card once the settings land", async () => {
    theSettings(UNSET, json(SAVED));
    const { container } = mountBoth();

    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));

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
    await drawn(container, `.${styles.githubCard} .${styles.tokenStanding}`);

    expect(screen.queryByText(/sessions cannot reach GitHub/)).toBeNull();
    expect(screen.queryByText(/commits inside a session fail/)).toBeNull();
  });
});

describe("the form", () => {
  /// A pane rather than a modal: it is the details of the card beside it, and
  /// what stands in a details pane is a pane's worth of blocks under the pane's
  /// own head.
  it("stands in a pane, with both sections under one Save", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    await waitFor(() => screen.getByLabelText("Name"));

    expect(container.querySelector("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "Replace" })).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Save" })).toHaveLength(1);
  });

  /// The way out of a details pane is the way back its head draws, hidden by the
  /// frame wherever the pane it goes back to is already on screen. A Cancel
  /// beside it would be a second way out of a pane that has one.
  it("is left by the way back rather than by a cancel", async () => {
    theSettings(TOLD);
    const { container, back } = mountPane();

    await waitFor(() => screen.getByLabelText("Name"));
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();

    const out = container.querySelector<HTMLButtonElement>(`.${head.back}`)!;
    expect(out.textContent).toContain("Settings");
    fireEvent.click(out);
    expect(back).toHaveBeenCalled();
  });

  /// The one promise this pane makes: the whole token is not in the payload it
  /// is drawn from, so it cannot be in the page — and the field to type one into
  /// starts empty rather than prefilled with anything.
  it("never puts a token in the field", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => screen.getByRole("button", { name: "Replace" }));
    fireEvent.click(screen.getByRole("button", { name: "Replace" }));

    expect(
      (screen.getByLabelText(/Token, pasted/) as HTMLInputElement).value,
    ).toBe("");
  });

  it("fills the author fields in with what is configured", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() =>
      expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
        TOLD.git_author.name,
      ),
    );
    expect((screen.getByLabelText("Email") as HTMLInputElement).value).toBe(
      TOLD.git_author.email,
    );
  });

  it("says so when the settings could not be read at all", async () => {
    serving(whenever("/api/ui/settings", json({ error: "gone" }, 500)));
    mountPane();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("saving", () => {
  it("sends the author fields as they were typed", async () => {
    const fetching = theSettings(TOLD, json(SAVED));
    mountPane();
    await waitFor(() => screen.getByLabelText("Email"));

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
        // The build cache rides along as it stands, because the endpoint
        // writes the whole of `config.yaml` and this form only means to
        // change the author.
        rust_build_cache: {
          enabled: TOLD.rust_build_cache.enabled,
          size: TOLD.rust_build_cache.size,
        },
        share_viewer_url: TOLD.share_viewer_url,
        conflict_resolution: TOLD.conflict_resolution,
        ...PATHS,
      }),
    );
  });

  /// The author round-trips onto the card: what the save answered with is what
  /// the page goes back to showing, because that is a fresh read of the files
  /// rather than an echo of what was sent.
  it("shows on the card the author the save came back with", async () => {
    const rewritten: SettingsSaved = {
      settings: {
        ...SAVED.settings,
        git_author: { name: "Ada Lovelace", email: "ada@analytical.engine" },
      },
      verified: null,
    };
    theSettings(TOLD, json(rewritten));
    const { container } = mountBoth();
    await waitFor(() => screen.getByLabelText("Email"));

    fireEvent.input(screen.getByLabelText("Email"), {
      target: { value: "ada@analytical.engine" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(
        container.querySelector(
          `.${styles.githubCard} .${styles.authorEmail}`,
        )!.textContent,
      ).toContain("ada@analytical.engine"),
    );
  });

  /// A write-only field left blank means *leave it alone*: a page that read one
  /// as *clear this* would take the credentials away every time somebody
  /// corrected their own email address.
  it("keeps the configured token when the field was not filled in", async () => {
    const fetching = theSettings(UNSET, json(SAVED));
    mountPane();
    await waitFor(() => screen.getByLabelText("Name"));

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
    mountPane();
    await waitFor(() => screen.getByLabelText(/Token, pasted/));

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

  /// Said in the pane the press was made in, which is still standing to hear the
  /// answer: the modal that used to be gone by now is a pane that stays.
  it("says which account GitHub verified the token as", async () => {
    theSettings(UNSET, json(SAVED));
    const { container } = mountPane();
    await waitFor(() => screen.getByLabelText(/Token, pasted/));

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/GitHub says it is/));
    expect(container.querySelector(`.${styles.login}`)!.textContent).toBe("ada");
  });

  /// A token can be whose it should be and still not do what Verkstead needs of
  /// it. Publishing a share writes a secret gist, which is the `gist` scope, and
  /// a token issued for reading repositories does not carry it — so the pane
  /// says so here, where the human already is, rather than leaving it to be
  /// found by a press on a conversation weeks later.
  it("says which scope a verified token is missing", async () => {
    const unscoped: SettingsSaved = {
      settings: SAVED.settings,
      verified: { Account: { login: "ada", missing: ["gist"] } },
    };
    theSettings(UNSET, json(unscoped));
    const { container } = mountPane();
    await waitFor(() => screen.getByLabelText(/Token, pasted/));

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    // Whose it is, and what it may not do: two lines, because they are two
    // different things to do about it.
    await waitFor(() => screen.getByText(/GitHub says it is/));
    await waitFor(() => screen.getByText(/It cannot publish a share/));
    expect(container.querySelector(`.${styles.scope}`)!.textContent).toBe(
      "gist",
    );
  });

  /// And a token that can do everything asked of it says nothing extra, which
  /// is what most tokens are.
  it("says nothing about scopes on a token that has them all", async () => {
    theSettings(UNSET, json(SAVED));
    mountPane();
    await waitFor(() => screen.getByLabelText(/Token, pasted/));

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/GitHub says it is/));
    expect(screen.queryByText(/It cannot publish a share/)).toBeNull();
  });

  /// A token GitHub would not vouch for is saved all the same — it is pasted
  /// once out of a page that will not show it again — so the pane says both
  /// halves: what is stored, and that nobody could be asked whose it is.
  it("says in words why a token could not be verified, and shows it stored", async () => {
    const unverified: SettingsSaved = {
      settings: SAVED.settings,
      verified: { Refused: { why: "gh: Bad credentials (HTTP 401)" } },
    };
    theSettings(UNSET, json(unverified));
    mountPane();
    await waitFor(() => screen.getByLabelText(/Token, pasted/));

    fireEvent.input(screen.getByLabelText(/Token, pasted/), {
      target: { value: TOKEN },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/Bad credentials/));
    await waitFor(() => screen.getByText(/A token ending/));
  });

  /// A server that could not write the files, which is the one thing here that
  /// is an error rather than an answer — and the form is still standing to say
  /// so, because nothing has been saved to go back to reading.
  it("says so when the settings could not be saved, and keeps the form up", async () => {
    theSettings(TOLD, json({ error: "the disk is full" }, 503));
    mountPane();
    await waitFor(() => screen.getByLabelText("Name"));

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/could not be saved/));
    expect(screen.getByLabelText("Name")).toBeTruthy();
  });
});

describe("replacing and clearing the token", () => {
  /// The field to paste a credential into is one somebody asked for: with a
  /// token already configured, replacing is a press. And the way back from it is
  /// named for what it does.
  it("opens the field on the press, and closes it again on the way back", async () => {
    theSettings(TOLD);
    mountPane();
    await waitFor(() => screen.getByRole("button", { name: "Replace" }));

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

  /// The pane stays up, because a details pane is left by the human rather than
  /// by a save — but the write-only field is spent with the press, which is what
  /// puts Replace back in its place.
  it("sends the replacement, and spends the field", async () => {
    const fetching = theSettings(TOLD, json(SAVED));
    mountPane();
    await waitFor(() => screen.getByRole("button", { name: "Replace" }));

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

    await waitFor(() => screen.getByRole("button", { name: "Replace" }));
    expect(screen.queryByLabelText(/Token, pasted/)).toBeNull();
  });

  /// Clearing is its own press for the reason a blank field is a `Keep`: the two
  /// cannot be the same gesture. The author fields ride along, because the
  /// server writes both files in one request.
  it("clears the token without taking the author with it", async () => {
    const cleared: SettingsSaved = {
      settings: {
        git_author: TOLD.git_author,
        github_token: null,
        // Untouched by this form, and so untouched in what the save answers
        // with — see the sections below it for what does change these.
        rust_build_cache: TOLD.rust_build_cache,
        share_viewer_url: TOLD.share_viewer_url,
        conflict_resolution: TOLD.conflict_resolution,
        paths: TOLD.paths,
      },
      verified: null,
    };
    const fetching = theSettings(TOLD, json(cleared));
    const { container } = mountBoth();
    await waitFor(() => screen.getByRole("button", { name: "Clear" }));

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Clear",
        rust_build_cache: {
          enabled: TOLD.rust_build_cache.enabled,
          size: TOLD.rust_build_cache.size,
        },
        share_viewer_url: TOLD.share_viewer_url,
        conflict_resolution: TOLD.conflict_resolution,
        ...PATHS,
      }),
    );

    // And the card is the unset one again, warning and all, with the author it
    // never sent away.
    await waitFor(() => screen.getByText(/sessions cannot reach GitHub/));
    expect(
      container.querySelector(`.${styles.githubCard} .${styles.authorName}`)!
        .textContent,
    ).toBe(TOLD.git_author.name);
  });
});

/// The page the credentials head, on the routes the app really gives it: the
/// settings pane in the middle, the conversations beside it, and a details pane
/// for whatever the path names.
function thePage(at = "/settings") {
  const queries = client();

  serving(
    whenever("/api/ui/settings", json(TOLD)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever("/api/ui/repos", json(REPOS)),
    // And one of them opened, which is what a card leads to — with the answer
    // for an id nothing is registered under beside it, because a link followed
    // after somebody took a repo away is a path this page has to draw.
    whenever(`/api/ui/repos/${FIRST_REPO.id}`, json(OPENED)),
    whenever("/api/ui/repos/404", json({ error: "there is no Repo 404" }, 404)),
    // The one write this page can make that is not the credentials' own: a
    // registration, which the pane sends to the same path it read the list
    // from.
    whenever("/api/ui/repos", json("Added"), "POST"),
    whenever("/api/ui/update", json("Current")),
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/abandoned-roadmaps", json([])),
  );

  const history = createMemoryHistory();
  history.set({ value: at });

  return {
    ...render(() => (
      <QueryClientProvider client={queries}>
        <MemoryRouter history={history}>
          {/* Nested exactly as `App.tsx` nests them, out of the page's own
              `panes()` rather than written out again: the nesting is what keeps
              the middle pane up while the leaf under it changes, so a mount that
              flattened them would be testing a page the app does not build —
              and a list spelled out here would be a second opinion about where
              a card leads, which is the drift that left the share viewer's card
              opening the catch-all. */}
          <Route path="/settings" component={SettingsPage}>
            {settingsPanes()}
          </Route>
          <Route path="*" component={() => <p>somewhere else</p>} />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
  };
}

/// The three panes of the frame, in the order they are walked through.
function panes(container: ParentNode): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(`.${shell.panes} > section`)];
}

describe("the settings page", () => {
  /// The same frame the workbench stands on, with the settings where the
  /// Timeline is: the conversations ride along, and the third pane is what the
  /// path names.
  it("stands on the three panes, with the conversations beside it", async () => {
    const { container } = thePage();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(panes(container).map((pane) => pane.getAttribute("aria-label"))).toEqual([
      "Conversations",
      "Settings",
      "Details",
    ]);
    // The list is the workbench's own, drawn by the same component: its rows are
    // there, and so is the gear at the head of it.
    await waitFor(() => screen.getByText(SIDEBAR[0]!.branch));
    expect(container.querySelector(`.${shell.panes}`)!.getAttribute("data-pane")).toBe(
      "middle",
    );
  });

  /// One page for everything the human configures: what Verkstead itself was
  /// told, and the two things a Conversation is settled against — in the reading
  /// order a fresh install needs them in.
  it("holds the credentials, the build cache, the paths, the profiles and the repos", async () => {
    const { container } = thePage();

    const settings = panes(container)[1]!;

    // The repo names are on the New conversation menu as well as on this list,
    // so each list is waited for inside the pane it belongs to.
    await drawn(settings, `.${styles.githubCard}`);
    await drawn(settings, `.${buildCache.buildCacheCard}`);
    await drawn(settings, `.${paths.pathsCard}`);
    await drawn(settings, `.${profileList.profiles} .${profileList.profile}`);
    await drawn(settings, `.${repoList.repos} .${repoList.repo}`);

    expect(settings.querySelectorAll("h1")).toHaveLength(1);
    expect(
      settings.querySelectorAll(`.${repoList.repos} .${repoList.repo}`),
    ).toHaveLength(REPOS.length);
    expect(
      settings.querySelectorAll(`.${profileList.profiles} .${profileList.profile}`),
    ).toHaveLength(PROFILES.length);
  });

  /// The gear is what opened this pane, so it reads as the open one — which is
  /// what the cards under it say about themselves.
  it("reads the gear in the conversations as open", async () => {
    const { container } = thePage();

    const gear = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Settings"]',
    );
    expect(gear.getAttribute("aria-pressed")).toBe("true");
    expect(gear.classList).toContain(button.open);
  });

  /// And the list beside the settings is the list: pressing a Conversation on it
  /// opens that Conversation, from here as from anywhere.
  it("opens a conversation from the pane beside it", async () => {
    const { container, history } = thePage();

    const row = await drawn<HTMLElement>(
      container,
      `[data-id="${SIDEBAR[0]!.id}"] [role="button"], [data-id="${SIDEBAR[0]!.id}"] button`,
    );
    fireEvent.click(row);

    await waitFor(() =>
      expect(history.get()).toBe(`/conversations/${SIDEBAR[0]!.id}`),
    );
  });

  /// The way back out of the settings, in the slot every pane keeps for it —
  /// drawn always and hidden by the frame wherever the conversations are already
  /// on screen.
  it("goes back to the conversations", async () => {
    const { container, history } = thePage();

    const settings = panes(container)[1]!;
    const back = await drawn<HTMLButtonElement>(settings, `.${head.back}`);
    expect(back.textContent).toContain("Conversations");

    fireEvent.click(back);
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// The switch about this device came with the Repos it was on, and it keeps
  /// the head's line it had. The banner about this server came with it; where
  /// that sits is `update.test.tsx`'s.
  it("keeps the notifications switch on the pane head's line", async () => {
    const { container } = thePage();

    await waitFor(() => screen.getByText(TOLD.github_token!.last_four));

    expect(
      container.querySelector(`.${head.head} .${notifications.notifications}`),
    ).not.toBeNull();
  });

  /// Every form on this page is a pane of its own now, so what the middle pane
  /// carries is the way into each of them and none of their fields. There is no
  /// modal left anywhere on it.
  it("carries no form at all until a pane is opened", async () => {
    const { container } = thePage();

    await drawn(container, `.${repoList.repos} .${repoList.repo}`);

    const settings = panes(container)[1]!;
    expect(settings.querySelector("dialog")).toBeNull();
    expect(settings.querySelector("form")).toBeNull();
    expect(screen.queryByLabelText("Name")).toBeNull();
    expect(screen.queryByLabelText(/absolute path/i)).toBeNull();
    expect(panes(container)[2]!.textContent).toBe("");
  });
});

describe("the path a details pane stands at", () => {
  /// The card opens the pane by navigating to where that pane stands, so what is
  /// open survives a reload and can be linked to. It replaces rather than
  /// pushes: the details of the settings are places in a page rather than pages.
  it("opens the credentials at /settings/github, replacing", async () => {
    const { container, history } = thePage();

    const face = await drawn<HTMLElement>(container, `.${styles.githubCard}`);
    fireEvent.click(face);

    await waitFor(() => expect(history.get()).toBe("/settings/github"));

    // Replaced rather than pushed: the settings' own entry is the one that was
    // written over, so Back leaves the settings rather than walking out of the
    // pane that was just opened.
    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// The build cache is the other pane a word names, and it opens the way the
  /// credentials do.
  it("opens the build cache at /settings/build-cache, replacing", async () => {
    const { container, history } = thePage();

    const face = await drawn<HTMLElement>(
      container,
      `.${buildCache.buildCacheCard}`,
    );
    fireEvent.click(face);

    await waitFor(() => expect(history.get()).toBe("/settings/build-cache"));

    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// And the third pane a word names: where the share viewer is hosted.
  it("opens the share viewer at /settings/share-viewer, replacing", async () => {
    const { container, history } = thePage();

    const face = await drawn<HTMLElement>(
      container,
      `.${shareViewer.shareViewerCard}`,
    );
    fireEvent.click(face);

    await waitFor(() => expect(history.get()).toBe("/settings/share-viewer"));

    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// And the fourth: the paths Verkstead may work inside, and what a sandbox is
  /// given beyond its worktree.
  it("opens the paths at /settings/paths, replacing", async () => {
    const { container, history } = thePage();

    const face = await drawn<HTMLElement>(container, `.${paths.pathsCard}`);
    fireEvent.click(face);

    await waitFor(() => expect(history.get()).toBe("/settings/paths"));

    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  it("draws the two lists in the details pane, and reads the paths card as open", async () => {
    const { container } = thePage("/settings/paths");

    await waitFor(() => screen.getByLabelText("Add a watched path"));

    const face = await drawn<HTMLElement>(container, `.${paths.pathsCard}`);
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);
  });

  it("draws the viewer's field in the details pane, and reads its card as open", async () => {
    const { container } = thePage("/settings/share-viewer");

    await waitFor(() => screen.getByLabelText(/Where you hosted it/));

    const face = await drawn<HTMLElement>(
      container,
      `.${shareViewer.shareViewerCard}`,
    );
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);
  });

  it("draws the switch in the details pane, and reads the cache card as open", async () => {
    const { container } = thePage("/settings/build-cache");

    await waitFor(() => screen.getByRole("switch", { name: /build cache/i }));

    const face = await drawn<HTMLElement>(
      container,
      `.${buildCache.buildCacheCard}`,
    );
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);
  });

  it("draws the form in the details pane, and reads the card as open", async () => {
    const { container } = thePage("/settings/github");

    await waitFor(() => screen.getByLabelText("Name"));

    const face = await drawn<HTMLElement>(container, `.${styles.githubCard}`);
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);

    // In the third pane rather than over the page: the modal is gone.
    expect(panes(container)[2]!.querySelector("form")).not.toBeNull();
    expect(container.querySelector("dialog")).toBeNull();
  });

  /// A cold load of a details pane — a reload, or a link somebody kept — opens
  /// on that pane, which is the level a narrow window shows.
  it("opens on the details when the path names one", async () => {
    const { container } = thePage("/settings/github");

    await waitFor(() => screen.getByLabelText("Name"));
    expect(container.querySelector(`.${shell.panes}`)!.getAttribute("data-pane")).toBe(
      "details",
    );
  });

  /// And the way back out of it is a change of level rather than a navigation:
  /// what is open stays open, and the URL goes on saying so.
  it("walks back to the settings without closing the pane", async () => {
    const { container, history } = thePage("/settings/github");

    await waitFor(() => screen.getByLabelText("Name"));
    const details = panes(container)[2]!;
    fireEvent.click(details.querySelector<HTMLButtonElement>(`.${head.back}`)!);

    await waitFor(() =>
      expect(
        container.querySelector(`.${shell.panes}`)!.getAttribute("data-pane"),
      ).toBe("middle"),
    );
    expect(history.get()).toBe("/settings/github");
  });

  /// A Profile is the first thing on this page with an id of its own, so it is
  /// the first whose pane stands behind a segment: `profiles/` keeps the ids
  /// apart from the panes a word names beside them.
  it("opens a profile at /settings/profiles/:id, replacing", async () => {
    const { container, history } = thePage();

    const face = await drawn<HTMLElement>(
      container,
      `.${profileList.profiles} .${profileList.profile}`,
    );
    fireEvent.click(face);

    await waitFor(() =>
      expect(history.get()).toBe(`/settings/profiles/${PROFILES[0]!.id}`),
    );

    // Replaced rather than pushed, as every detail of the settings is: Back
    // leaves the settings rather than walking out of the pane just opened.
    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// The blank form stands where a filled one does: it is the same pane asked
  /// about a Profile that does not exist yet.
  it("opens the blank form at /settings/profiles/new, replacing", async () => {
    const { container, history } = thePage();

    const plus = await drawn<HTMLButtonElement>(
      panes(container)[1]!,
      'button[aria-label="Add a profile"]',
    );
    fireEvent.click(plus);

    await waitFor(() => expect(history.get()).toBe("/settings/profiles/new"));

    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  it("draws the filled-in form in the details pane, and reads the card as open", async () => {
    const { container } = thePage(`/settings/profiles/${PROFILES[0]!.id}`);

    await waitFor(() =>
      expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
        PROFILES[0]!.name,
      ),
    );

    const face = await drawn<HTMLElement>(
      container,
      `.${profileList.profiles} .${profileList.profile}`,
    );
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);

    // The paths and the agent type the card gave up, in the pane that has room
    // for them: the account's paths are the form's own fields.
    const details = panes(container)[2]!;
    const account = PROFILES[0]!.account;
    if (account.agent_type !== "Claude") {
      throw new Error("this fixture should be a Claude account");
    }
    expect(
      (screen.getByLabelText(/Claude directory/) as HTMLInputElement).value,
    ).toBe(account.claude_dir);
    expect(details.textContent).toContain(account.agent_type);

    // In the third pane rather than over the page: the modal is gone.
    expect(details.querySelector("form")).not.toBeNull();
    expect(container.querySelector("dialog")).toBeNull();
  });

  /// The plus reads as open while the pane it opens stands, the way a card
  /// does: it is another thing in this pane that has been selected.
  it("draws the blank form at /settings/profiles/new, and reads the plus as open", async () => {
    const { container } = thePage("/settings/profiles/new");

    await waitFor(() =>
      expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(""),
    );

    const plus = await drawn<HTMLButtonElement>(
      panes(container)[1]!,
      'button[aria-label="Add a profile"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("true");
    expect(plus.classList).toContain(button.open);
    expect(container.querySelector("dialog")).toBeNull();
  });

  /// A registered Repo has a pane of its own, behind the `repos/` segment for
  /// the reason a Profile's stands behind `profiles/`.
  it("opens a repo at /settings/repos/:id, replacing", async () => {
    const { container, history } = thePage();

    const face = await drawn<HTMLElement>(
      panes(container)[1]!,
      `.${repoList.repos} .${repoList.repo}`,
    );
    fireEvent.click(face);

    await waitFor(() =>
      expect(history.get()).toBe(`/settings/repos/${FIRST_REPO.id}`),
    );

    // Replaced rather than pushed, as every detail of the settings is: Back
    // leaves the settings rather than walking out of the pane just opened.
    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  /// What the card could not hold, in the pane that has the room for it — and
  /// the card reading as open while that pane stands, like every other card.
  it("draws the repo in the details pane, and reads its card as open", async () => {
    const { container } = thePage(`/settings/repos/${FIRST_REPO.id}`);

    const details = panes(container)[2]!;
    await waitFor(() => expect(details.textContent).toContain(OPENED.path));

    expect(details.textContent).toContain(OPENED.default_branch);
    expect(details.textContent).toContain(`${OPENED.live} live`);
    expect(details.textContent).toContain(`${OPENED.finished} finished`);
    for (const branch of OPENED.branches) {
      expect(details.textContent).toContain(branch);
    }
    for (const roadmap of OPENED.roadmaps) {
      expect(details.textContent).toContain(roadmap.title);
      expect(details.textContent).toContain(roadmap.stage_title);
    }

    const face = await drawn<HTMLElement>(
      panes(container)[1]!,
      `.${repoList.repos} .${repoList.repo}`,
    );
    expect(face.getAttribute("aria-pressed")).toBe("true");
    expect(face.classList).toContain(card.open);
  });

  /// A link followed after somebody took the repo away: said in a line rather
  /// than shown as an error the human is meant to do something about.
  it("says the repo is gone where the server has no such id", async () => {
    const { container } = thePage("/settings/repos/404");

    await waitFor(() =>
      expect(panes(container)[2]!.textContent).toContain("That repo is gone."),
    );
  });

  /// And the other pane under the same segment: the path another Repo is
  /// registered by, standing where an id stands.
  it("opens the repo form at /settings/repos/new, replacing", async () => {
    const { container, history } = thePage();

    const plus = await drawn<HTMLButtonElement>(
      panes(container)[1]!,
      'button[aria-label="Add a repo"]',
    );
    fireEvent.click(plus);

    await waitFor(() => expect(history.get()).toBe("/settings/repos/new"));

    // Replaced rather than pushed, as every detail of the settings is: Back
    // leaves the settings rather than walking out of the pane just opened.
    history.back();
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  it("draws the repo form in the details pane, and reads the plus as open", async () => {
    const { container } = thePage("/settings/repos/new");

    await waitFor(() => screen.getByLabelText(/absolute path/i));

    const plus = await drawn<HTMLButtonElement>(
      panes(container)[1]!,
      'button[aria-label="Add a repo"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("true");
    expect(plus.classList).toContain(button.open);

    // In the third pane rather than over the page: the modal is gone.
    expect(panes(container)[2]!.querySelector("form")).not.toBeNull();
    expect(container.querySelector("dialog")).toBeNull();
  });

  /// A registration that was taken spends the pane, and the repo lands on the
  /// list behind it: that card is the whole of the confirmation.
  it("puts the human back on the settings once a repo is registered", async () => {
    const { container, history } = thePage("/settings/repos/new");

    const field = await waitFor(() => screen.getByLabelText(/absolute path/i));
    fireEvent.input(field, { target: { value: "/srv/repos/verkstead" } });
    fireEvent.click(screen.getByRole("button", { name: "Register" }));

    await waitFor(() => expect(history.get()).toBe("/settings"));
    expect(panes(container)[2]!.textContent).toBe("");
    await drawn(container, `.${repoList.repos} .${repoList.repo}`);
  });

  /// A cold load of a profile's pane — a reload, or a link somebody kept —
  /// opens on that pane, which is the level a narrow window shows.
  it("opens on the details when the path names a profile", async () => {
    const { container } = thePage(`/settings/profiles/${PROFILES[0]!.id}`);

    await waitFor(() => screen.getByLabelText("Name"));
    expect(container.querySelector(`.${shell.panes}`)!.getAttribute("data-pane")).toBe(
      "details",
    );
  });

  /// A segment that is no id the server ever issued names no pane at all, and
  /// leaves the details as bare as they are when nothing is open: the URL is a
  /// record of what was picked rather than a promise that it is still there.
  it("leaves the details bare where the path names nothing", async () => {
    const { container } = thePage("/settings/profiles/nonsense");

    await drawn(container, `.${profileList.profiles} .${profileList.profile}`);
    expect(panes(container)[2]!.textContent).toBe("");
  });
});

/// The arithmetic alone, which knows nothing of the page: a path is a value, and
/// a value is cheaper to hold true here than through a mounted one. What the
/// page does with it is the two suites above.
describe("where a settings details pane stands", () => {
  it("puts an id behind a segment of its own, and a word beside it", () => {
    expect(pathTo("github")).toBe("/settings/github");
    expect(pathTo("build-cache")).toBe("/settings/build-cache");
    expect(pathTo("share-viewer")).toBe("/settings/share-viewer");
    expect(pathTo(opensProfile(7))).toBe("/settings/profiles/7");
    expect(pathTo(opensProfile("new"))).toBe("/settings/profiles/new");
    expect(pathTo(opensRepo(7))).toBe("/settings/repos/7");
    expect(pathTo(opensRepo("new"))).toBe("/settings/repos/new");
  });

  it("reads back everything it writes", () => {
    for (const opening of [
      "github",
      "build-cache",
      "share-viewer",
      opensProfile(7),
      opensProfile("new"),
      opensRepo(7),
      opensRepo("new"),
    ] as const) {
      expect(openingAt(pathTo(opening))).toBe(opening);
    }
  });

  /// Which Profile an opening is about, asked of the opening rather than of the
  /// path — it is what says which card reads as open.
  it("says which profile an opening names, and which names none", () => {
    expect(profileOpened(opensProfile(7))).toBe(7);
    expect(profileOpened(opensProfile("new"))).toBe("new");
    expect(profileOpened("github")).toBeNull();
    expect(profileOpened(opensRepo(7))).toBeNull();
    expect(profileOpened(null)).toBeNull();
  });

  /// And which Repo, read the same way. The two are the same shape and are told
  /// apart by their segment, which is what keeps a Repo's id from reading as a
  /// Profile's.
  it("says which repo an opening names, and which names none", () => {
    expect(repoOpened(opensRepo(7))).toBe(7);
    expect(repoOpened(opensRepo("new"))).toBe("new");
    expect(repoOpened("github")).toBeNull();
    expect(repoOpened(opensProfile(7))).toBeNull();
    expect(repoOpened(null)).toBeNull();
  });

  /// The settings' own path opens nothing, and neither does anything that is
  /// not a path of ours — whatever it starts with.
  it.each([
    ["/settings"],
    ["/settings/"],
    ["/settings/nonsense"],
    ["/settings/github/extra"],
    ["/settings/profiles"],
    ["/settings/profiles/nonsense"],
    ["/settings/profiles/7/extra"],
    ["/settings/profiles/7.5"],
    ["/settings/repos"],
    ["/settings/repos/nonsense"],
    ["/settings/repos/7/extra"],
    ["/settings/repos/new/extra"],
    ["/conversations/3/events/7"],
    ["/"],
  ])("opens nothing at %s", (path) => {
    expect(openingAt(path)).toBeNull();
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
