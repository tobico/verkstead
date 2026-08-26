//! The agent profiles page: the accounts a session can be run under, and the
//! one form that adds or rewrites one, which is a modal drawn over the list.
//!
//! What a modal *is* — the dialog, Escape, a press away from the card — belongs
//! to `Modal` and is read in `modals.test.tsx`. What is here is this page's half
//! of it: which press opens the form, what it is filled in with when it opens,
//! and that a refusal keeps it up.
//!
//! `tests/fixtures/profiles.json` is a golden fixture like the workbench's:
//! `cargo test` renders the real endpoint and writes the file, so what these
//! assertions read is what the server actually said.
//!
//! Whether a pair is really there, and whether it is inside the watched paths,
//! are the server's to decide — the tests over in `crates/server` are what say
//! so. This side's job is to send what was typed and say in words what came
//! back.

import { MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ProfileEntry } from "../src/api/types";
import { ProfileList } from "../src/profiles/ProfileList";
import styles from "../src/profiles/ProfileList.module.css";
import { json, serving, whenever } from "./serving";
import profiles from "./fixtures/profiles.json" with { type: "json" };

const SAVED = profiles as ProfileEntry[];
const FABLE = SAVED[0]!;

/// The fixture's other account, which lists more than one model — a profile
/// says everything it can launch, and the row is where that is read.
const OPUS = SAVED[1]!;

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
        <Route path="/" component={ProfileList} />
      </MemoryRouter>
    </QueryClientProvider>
  ));
}

/// The list as it stands, with whatever the page's writes are answered by.
function theProfiles(...answers: Array<() => Promise<Response>>) {
  return serving(whenever("/api/ui/profiles", json(SAVED)), ...answers);
}

/// The body the page put on the wire when it wrote to `path`.
function sent(fetching: ReturnType<typeof serving>, path: string): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
  expect(written, `expected the page to have written to ${path}`).toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// Open the empty form, which is what the button on the heading does.
function addProfile() {
  fireEvent.click(screen.getByRole("button", { name: "Add a profile" }));
}

/// Open it filled in with what a profile already says, which is what a row's
/// own edit does.
function editProfile(name: string) {
  fireEvent.click(
    screen
      .getByText(name)
      .closest("li")!
      .querySelector(`.${styles.actions} button`)!,
  );
}

/// The form, or nothing at all where it has not been opened.
function theForm(container: ParentNode): HTMLDialogElement | null {
  return container.querySelector<HTMLDialogElement>(`dialog.${styles.form}`);
}

/// Fill the form in, whichever profile it is about.
function fillIn(profile: {
  name: string;
  models: string[];
  claude_dir: string;
  config_file: string;
}) {
  fireEvent.input(screen.getByLabelText("Name"), {
    target: { value: profile.name },
  });
  fireEvent.input(screen.getByLabelText("Models, one per line"), {
    target: { value: profile.models.join("\n") },
  });
  fireEvent.input(screen.getByLabelText(/Claude directory/), {
    target: { value: profile.claude_dir },
  });
  fireEvent.input(screen.getByLabelText(/Config file/), {
    target: { value: profile.config_file },
  });
}

const NEW = {
  name: "personal",
  models: ["claude-sonnet-5"],
  claude_dir: "/home/you/accounts/personal/.claude",
  config_file: "/home/you/accounts/personal/.claude.json",
};

describe("the agent profiles page", () => {
  it("lists what the server gave it, in that order", async () => {
    const fetching = theProfiles();
    const { container } = mount();

    await waitFor(() => screen.getByText(FABLE.name));

    expect(fetching).toHaveBeenCalledWith(
      "/api/ui/profiles",
      expect.anything(),
    );
    expect(
      [...container.querySelectorAll(`.${styles.row} .${styles.title}`)].map(
        (row) => row.textContent,
      ),
    ).toEqual(SAVED.map((profile) => profile.name));
  });

  /// The paths shown are the resolved ones the server recorded rather than
  /// whatever was typed, and the point of showing them is that they can be
  /// checked.
  it("says of each profile what it runs and what is mounted for it", async () => {
    theProfiles();
    mount();

    const row = (await waitFor(() => screen.getByText(FABLE.name))).closest(
      "li",
    )!;

    expect(
      [...row.querySelectorAll(`.${styles.model}`)].map((it) => it.textContent),
    ).toEqual(FABLE.models);
    // The agent type closes the meta line the models are on: it has no paint of
    // its own, so it is read as the last thing on that line rather than by a
    // class that would exist only to be queried here.
    expect(
      row.querySelector(`.${styles.meta}`)!.lastElementChild!.textContent,
    ).toBe(FABLE.agent_type);

    const paths = [...row.querySelectorAll(`.${styles.path}`)].map(
      (it) => it.textContent,
    );
    expect(paths).toEqual([FABLE.claude_dir, FABLE.config_file]);
  });

  /// The list is the whole of what the account can launch, so the row shows all
  /// of it: one entry drawn out of several would be a preference the profile
  /// does not have.
  it("shows every model a profile lists", async () => {
    theProfiles();
    mount();

    const row = (await waitFor(() => screen.getByText(OPUS.name))).closest(
      "li",
    )!;

    expect(OPUS.models.length).toBeGreaterThan(1);
    expect(
      [...row.querySelectorAll(`.${styles.model}`)].map((it) => it.textContent),
    ).toEqual(OPUS.models);
  });

  /// The list is what stays on the page. Adding one is a form, and the form is
  /// drawn over the page when it is asked for — so until then there is none of
  /// it, and the list is not pushed down under one.
  it("keeps no form on the page until one is asked for", async () => {
    theProfiles();
    const { container } = mount();
    await waitFor(() => screen.getByText(FABLE.name));

    expect(theForm(container)).toBeNull();
    expect(screen.queryByLabelText("Name")).toBeNull();

    addProfile();

    expect(theForm(container)!.open, "opened as a modal").toBe(true);
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("");
  });

  it("says so plainly when nothing is saved yet", async () => {
    serving(whenever("/api/ui/profiles", json([])));
    mount();

    await waitFor(() => screen.getByText("No agent profiles are saved yet."));
  });
});

describe("saving a profile", () => {
  it("sends the four fields that were typed", async () => {
    const fetching = theProfiles(json("Saved"));
    mount();
    await waitFor(() => screen.getByText(FABLE.name));

    addProfile();
    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/profiles")).toEqual(NEW),
    );
  });

  /// The form is spent once it is taken: the profile appearing on the list
  /// underneath is the whole of the confirmation.
  it("takes the form away once the server took it", async () => {
    theProfiles(json("Saved"));
    const { container } = mount();
    await waitFor(() => screen.getByText(FABLE.name));

    addProfile();
    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(theForm(container)).toBeNull());
  });

  /// Each refusal is a different sentence, and each names which of the two paths
  /// it is about: pointing the config field at a directory is an easy mistake,
  /// and "that path is wrong" would not say which one.
  it.each([
    ["DirMissing", /nothing at the claude directory/i],
    ["ConfigMissing", /nothing at the config file/i],
    ["DirOutsideWatchedPaths", /claude directory is outside the watched paths/i],
    ["ConfigOutsideWatchedPaths", /config file is outside the watched paths/i],
    ["NotADirectory", /not a directory/i],
    ["NotAFile", /not a file/i],
    ["DirNotAbsolute", /claude directory's absolute path/i],
    ["ConfigNotAbsolute", /config file's absolute path/i],
    ["Nameless", /give the profile a name/i],
    ["Modelless", /at least one model/i],
    ["NameTaken", /another profile is called that already/i],
  ])("says why %s was refused, in words", async (outcome, said) => {
    theProfiles(json(outcome));
    const { container } = mount();
    await waitFor(() => screen.getByText(FABLE.name));

    addProfile();
    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    // Inside the form, and the form still up: what was typed is still there to
    // be corrected, which is the whole use of being told what was wrong with it.
    const refusal = await waitFor(() => screen.getByText(said));
    expect(theForm(container)!.contains(refusal)).toBe(true);
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
      NEW.name,
    );
  });

  /// A server that could not answer at all, which is the one thing here that is
  /// an error rather than an outcome.
  it("says so when the server could not answer", async () => {
    theProfiles(json({ error: "the database is gone" }, 500));
    mount();
    await waitFor(() => screen.getByText(FABLE.name));

    addProfile();
    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});

describe("rewriting a profile", () => {
  /// Editing one is filling the same form in with what it already says: a
  /// profile is four fields with nothing built from them yet.
  it("fills the form in with what the profile already says", async () => {
    theProfiles();
    mount();
    await waitFor(() => screen.getByText(FABLE.name));

    editProfile(FABLE.name);

    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
      FABLE.name,
    );
    expect(
      (screen.getByLabelText("Models, one per line") as HTMLTextAreaElement)
        .value,
    ).toBe(FABLE.models.join("\n"));
    expect(
      (screen.getByLabelText(/Claude directory/) as HTMLInputElement).value,
    ).toBe(FABLE.claude_dir);
    expect(
      (screen.getByLabelText(/Config file/) as HTMLInputElement).value,
    ).toBe(FABLE.config_file);
  });

  it("sends the rewrite to the profile it is about", async () => {
    const fetching = theProfiles(json("Saved"));
    mount();
    await waitFor(() => screen.getByText(FABLE.name));

    editProfile(FABLE.name);

    // A line apiece, which is how the whole list is retyped: the models go over
    // as the lines they were written on.
    fireEvent.input(screen.getByLabelText("Models, one per line"), {
      target: { value: "claude-haiku-4-5-20251001\nclaude-opus-5" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/profiles/${FABLE.id}`)).toEqual({
        name: FABLE.name,
        models: ["claude-haiku-4-5-20251001", "claude-opus-5"],
        claude_dir: FABLE.claude_dir,
        config_file: FABLE.config_file,
      }),
    );
  });

  /// Three ways out, and none of them saves: the button a thumb has, the key a
  /// keyboard has, and a press away from the card. What each of them leaves is
  /// the list, with the profile as it was.
  it.each([
    ["Cancel is pressed", () => fireEvent.click(screen.getByRole("button", { name: "Cancel" }))],
    ["Escape is pressed", () => fireEvent.keyDown(document, { key: "Escape" })],
  ])("takes the form away, unsaved, when %s", async (_, back) => {
    const fetching = theProfiles();
    const { container } = mount();
    await waitFor(() => screen.getByText(FABLE.name));

    editProfile(FABLE.name);
    fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "something else" },
    });

    back();

    await waitFor(() => expect(theForm(container)).toBeNull());
    expect(
      fetching.mock.calls.filter(([, init]) => init?.method === "POST"),
      "nothing was sent",
    ).toHaveLength(0);
  });

  /// The press away from the card, which lands on the dialog rather than on
  /// anything of the page behind it.
  it("takes the form away, unsaved, on a press away from it", async () => {
    const fetching = theProfiles();
    const { container } = mount();
    await waitFor(() => screen.getByText(FABLE.name));

    editProfile(FABLE.name);
    fireEvent.click(theForm(container)!);

    await waitFor(() => expect(theForm(container)).toBeNull());
    expect(
      fetching.mock.calls.filter(([, init]) => init?.method === "POST"),
      "nothing was sent",
    ).toHaveLength(0);
  });

  /// Opened again, it is about whatever it was opened at: what was typed into it
  /// last time went away with it, and nothing here promised to keep a draft.
  it("opens empty again after an edit was abandoned", async () => {
    theProfiles();
    mount();
    await waitFor(() => screen.getByText(FABLE.name));

    editProfile(FABLE.name);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    addProfile();

    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("");
    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
  });
});

describe("removing a profile", () => {
  it("asks the server to remove the one it is beside", async () => {
    const fetching = theProfiles(json("Removed"));
    mount();

    fireEvent.click(
      (await waitFor(() => screen.getByText(FABLE.name)))
        .closest("li")!
        .querySelector(`.${styles.actions} .${styles.remove}`)!,
    );

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        `/api/ui/profiles/${FABLE.id}/delete`,
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  /// Refused rather than taken away from the conversation that chose it: one
  /// pointing at a profile that is not there is a session that fails to start
  /// with nobody watching.
  it("says why a profile in use could not be removed", async () => {
    theProfiles(json("InUse"));
    mount();

    fireEvent.click(
      (await waitFor(() => screen.getByText(FABLE.name)))
        .closest("li")!
        .querySelector(`.${styles.actions} .${styles.remove}`)!,
    );

    await waitFor(() => screen.getByText(/A conversation is set to run under it/));
  });
});

describe("a profile whose pair has gone", () => {
  /// The pair was there when it was saved, and a directory can be moved
  /// afterwards. The server reports it on every read; this is where it is said.
  ///
  /// Built from the real payload rather than served as one of its own: whether
  /// the server reports it, and when, is `crates/server/tests/profiles.rs`'s
  /// subject — what is being read here is that the page says which half it was.
  it.each([
    ["DirMissing", "Its claude directory is gone."],
    ["ConfigMissing", "Its config file is gone."],
    ["OutsideWatchedPaths", "Its pair now points outside the watched paths."],
  ])("says of %s what is wrong with it", async (broken, said) => {
    const gone: ProfileEntry[] = [
      { ...FABLE, broken: broken as ProfileEntry["broken"] },
    ];
    serving(whenever("/api/ui/profiles", json(gone)));
    mount();

    const row = (await waitFor(() => screen.getByText(FABLE.name))).closest(
      "li",
    )!;

    expect(row.classList).toContain(styles.broken);
    expect(row.querySelector(`.${styles.broken}`)!.textContent).toBe(said);
  });

  it("says nothing about a profile whose pair is where it was left", async () => {
    theProfiles();
    mount();

    const row = (await waitFor(() => screen.getByText(FABLE.name))).closest(
      "li",
    )!;

    expect(FABLE.broken).toBeNull();
    expect(row.classList).not.toContain(styles.broken);
    expect(row.querySelector(`.${styles.broken}`)).toBeNull();
  });
});
