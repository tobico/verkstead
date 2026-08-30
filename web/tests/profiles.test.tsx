//! The Agent Profiles section of the settings page: the cards for the accounts a
//! session can be run under, and the pane that adds or rewrites one.
//!
//! The two halves are mounted apart, because that is what they are now: cards in
//! the middle pane carrying what a list is scanned for, and the form that
//! rewrites one in the details pane beside them. The page that puts the two
//! together — which path a card opens, and which card reads as open while a pane
//! stands — is `settings.test.tsx`'s, along with the arithmetic behind it.
//!
//! So this suite is in four parts: what is readable on the cards without opening
//! anything, what the pane does with a Profile that exists, what it does with
//! one that does not yet, and removing.
//!
//! `tests/fixtures/profiles.json` is a golden fixture like the workbench's:
//! `cargo test` renders the real endpoint and writes the file, so what these
//! assertions read is what the server actually said.
//!
//! Whether a pair is really there, and whether it is inside the watched paths,
//! are the server's to decide — the tests over in `crates/server` are what say
//! so. This side's job is to send what was typed and say in words what came
//! back.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ProfileEdit, ProfileEntry } from "../src/api/types";
import card from "../src/CardButton.module.css";
import button from "../src/IconButton.module.css";
import { ProfileList, ProfilePane } from "../src/profiles/ProfileList";
import styles from "../src/profiles/ProfileList.module.css";
import head from "../src/workbench/PaneHead.module.css";
import { drawn } from "./bench";
import { json, serving, whenever } from "./serving";
import profiles from "./fixtures/profiles.json" with { type: "json" };

const SAVED = profiles as ProfileEntry[];
const FABLE = SAVED[0]!;

/// The fixture's other account, which lists more than one model — a profile
/// says everything it can launch, and the card is where that is read.
const OPUS = SAVED[1]!;

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
/// as the page does.
function mounting(what: () => JSX.Element) {
  const queries = client();

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

/// The cards in the middle pane, and what pressing one of them asked for.
function mountCards(opening: number | "new" | null = null) {
  const open = vi.fn();
  const add = vi.fn();

  return {
    ...mounting(() => (
      <ProfileList opening={opening} open={open} add={add} />
    )),
    open,
    add,
  };
}

/// The form in the details pane, and what its two ways out asked for.
function mountPane(profile: number | "new") {
  const back = vi.fn();
  const done = vi.fn();

  return {
    ...mounting(() => (
      <ProfilePane profile={profile} back={back} done={done} />
    )),
    back,
    done,
  };
}

/// The list as it stands, with whatever the pane's writes are answered by.
function theProfiles(...answers: Array<() => Promise<Response>>) {
  return serving(whenever("/api/ui/profiles", json(SAVED)), ...answers);
}

/// The body the pane put on the wire when it wrote to `path`.
function sent(fetching: ReturnType<typeof serving>, path: string): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
  expect(written, `expected the page to have written to ${path}`).toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// One profile's card, by the name on it.
function theCard(name: string): HTMLElement {
  return screen.getByText(name).closest(`.${styles.profile}`)!;
}

/// Fill the form in, whichever profile it is about.
///
/// The account's fields are the ones its agent type has, which is a Claude
/// pair here — the form draws them off the type, and this fills in what it
/// drew.
function fillIn(profile: ProfileEdit) {
  fireEvent.input(screen.getByLabelText("Name"), {
    target: { value: profile.name },
  });
  fireEvent.input(screen.getByLabelText("Models, one per line"), {
    target: { value: profile.models.join("\n") },
  });
  fireEvent.input(screen.getByLabelText(/Claude directory/), {
    target: { value: profile.account.claude_dir },
  });
  fireEvent.input(screen.getByLabelText(/Config file/), {
    target: { value: profile.account.config_file },
  });
}

const NEW: ProfileEdit = {
  name: "personal",
  models: ["claude-sonnet-5"],
  account: {
    agent_type: "Claude",
    claude_dir: "/home/you/accounts/personal/.claude",
    config_file: "/home/you/accounts/personal/.claude.json",
  },
};

describe("the cards", () => {
  it("lists what the server gave it, in that order", async () => {
    const fetching = theProfiles();
    const { container } = mountCards();

    await waitFor(() => screen.getByText(FABLE.name));

    expect(fetching).toHaveBeenCalledWith(
      "/api/ui/profiles",
      expect.anything(),
    );
    expect(
      [...container.querySelectorAll(`.${styles.profile} .${styles.title}`)].map(
        (name) => name.textContent,
      ),
    ).toEqual(SAVED.map((profile) => profile.name));
  });

  /// The list is the whole of what the account can launch, so the card shows all
  /// of it: one entry drawn out of several would be a preference the profile
  /// does not have.
  it("shows every model a profile lists", async () => {
    theProfiles();
    mountCards();

    await waitFor(() => screen.getByText(OPUS.name));

    expect(OPUS.models.length).toBeGreaterThan(1);
    expect(
      [...theCard(OPUS.name).querySelectorAll(`.${styles.model}`)].map(
        (model) => model.textContent,
      ),
    ).toEqual(OPUS.models);
  });

  /// The mounted paths and the agent type came off the card and into the pane,
  /// which has the room for them: a list is scanned before it is read, and two
  /// absolute paths a card deep are neither.
  it("keeps the paths and the agent type off the card", async () => {
    theProfiles();
    mountCards();

    await waitFor(() => screen.getByText(FABLE.name));

    const face = theCard(FABLE.name);
    expect(face.textContent).not.toContain(FABLE.account.claude_dir);
    expect(face.textContent).not.toContain(FABLE.account.config_file);
    expect(face.textContent).not.toContain(FABLE.account.agent_type);
  });

  /// It is a card like every other card in the app: pressed to open the pane
  /// beside it, and drawn as the open one while that pane is what is being read.
  /// An `article` rather than a button, because it holds a paragraph.
  it("opens the pane when it is pressed", async () => {
    theProfiles();
    const { open } = mountCards();

    await waitFor(() => screen.getByText(FABLE.name));

    const face = theCard(FABLE.name);
    expect(face.getAttribute("role")).toBe("button");
    expect(face.getAttribute("aria-pressed")).toBe("false");
    expect(face.classList).not.toContain(card.open);

    fireEvent.click(face);
    expect(open).toHaveBeenCalledWith(FABLE.id);
  });

  it("reads as open while its own pane is, and no other card does", async () => {
    theProfiles();
    mountCards(FABLE.id);

    await waitFor(() => screen.getByText(FABLE.name));

    expect(theCard(FABLE.name).getAttribute("aria-pressed")).toBe("true");
    expect(theCard(FABLE.name).classList).toContain(card.open);
    expect(theCard(OPUS.name).classList).not.toContain(card.open);
  });

  /// The list is what stays in the pane. There is no form on it at all: adding
  /// one is a pane of its own now, and so is rewriting one.
  it("keeps no form beside the cards", async () => {
    theProfiles();
    const { container } = mountCards();

    await waitFor(() => screen.getByText(FABLE.name));

    expect(container.querySelector("form")).toBeNull();
    expect(container.querySelector("dialog")).toBeNull();
    expect(screen.queryByLabelText("Name")).toBeNull();
  });

  it("says so plainly when nothing is saved yet", async () => {
    serving(whenever("/api/ui/profiles", json([])));
    mountCards();

    await waitFor(() => screen.getByText("No agent profiles are saved yet."));
  });
});

describe("the plus that adds one", () => {
  /// An `IconButton`, for the reason the gear at the head of the conversations
  /// is one: it is another thing in the pane that is selected and opened into
  /// the pane beside it, rather than a quiet text button of its own kind.
  it("asks for the blank form when it is pressed", async () => {
    theProfiles();
    const { container, add } = mountCards();

    const plus = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Add a profile"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("false");
    expect(plus.classList).not.toContain(button.open);

    fireEvent.click(plus);
    expect(add).toHaveBeenCalled();
  });

  it("reads as open while the blank form is", async () => {
    theProfiles();
    const { container } = mountCards("new");

    const plus = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Add a profile"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("true");
    expect(plus.classList).toContain(button.open);
  });

  /// The plus is about the blank form and nothing else: a card being open says
  /// nothing about it.
  it("reads as shut while a saved profile's pane is open", async () => {
    theProfiles();
    const { container } = mountCards(FABLE.id);

    const plus = await drawn<HTMLButtonElement>(
      container,
      'button[aria-label="Add a profile"]',
    );
    expect(plus.getAttribute("aria-pressed")).toBe("false");
  });
});

describe("the pane a card opens", () => {
  /// Editing one is filling the same form in with what it already says: a
  /// profile is a name, a list of models and an account, with nothing built from
  /// them yet.
  it("fills the form in with what the profile already says", async () => {
    theProfiles();
    mountPane(FABLE.id);

    await waitFor(() =>
      expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
        FABLE.name,
      ),
    );
    expect(
      (screen.getByLabelText("Models, one per line") as HTMLTextAreaElement)
        .value,
    ).toBe(FABLE.models.join("\n"));
    expect(
      (screen.getByLabelText(/Claude directory/) as HTMLInputElement).value,
    ).toBe(FABLE.account.claude_dir);
    expect(
      (screen.getByLabelText(/Config file/) as HTMLInputElement).value,
    ).toBe(FABLE.account.config_file);
  });

  /// The form asks for what this profile's agent type keeps an account in, and
  /// nothing beside it: the fields come off the discriminator, so a profile of
  /// another type would be asked for that type's paths instead — and the stage
  /// adding one adds fields rather than restructuring this.
  it("asks only for the fields its agent type's account has", async () => {
    theProfiles();
    const { container } = mountPane(FABLE.id);

    await waitFor(() => screen.getByLabelText(/Claude directory/));

    expect(
      [...container.querySelectorAll("form label")].map(
        (label) => label.getAttribute("for"),
      ),
    ).toEqual([
      "profile-name",
      "profile-models",
      "profile-claude_dir",
      "profile-config_file",
    ]);
  });

  /// The paths shown are the resolved ones the server recorded rather than
  /// whatever was typed to save them — those are what will be bind-mounted, and
  /// the point of showing them is that they can be checked. The agent type is
  /// said beside them rather than offered, there being one of it.
  it("says what is on the record but not in the form", async () => {
    theProfiles();
    const { container } = mountPane(FABLE.id);

    const standing = await drawn(container, `.${styles.standing}`);
    expect(standing.textContent).toContain(FABLE.account.agent_type);
    expect(container.querySelector("select")).toBeNull();
  });

  /// The way back out of it, in the slot every pane keeps for it: a change of
  /// level rather than a navigation, which is the page's to make.
  it("goes back to the settings", async () => {
    theProfiles();
    const { container, back } = mountPane(FABLE.id);

    const out = await drawn<HTMLButtonElement>(container, `.${head.back}`);
    expect(out.textContent).toContain("Settings");

    fireEvent.click(out);
    expect(back).toHaveBeenCalled();
  });

  it("sends the rewrite to the profile it is about", async () => {
    const fetching = theProfiles(json("Saved"));
    mountPane(FABLE.id);

    await waitFor(() => screen.getByLabelText("Models, one per line"));

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
        account: FABLE.account,
      }),
    );
  });

  /// A pane that has been spent: what says the save landed is the card behind
  /// it, which is where the human is put back.
  it("spends the pane once the server took the rewrite", async () => {
    theProfiles(json("Saved"));
    const { done } = mountPane(FABLE.id);

    await waitFor(() => screen.getByLabelText("Name"));
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(done).toHaveBeenCalled());
  });

  /// There is no Cancel and there is no dialog: a details pane is left by
  /// opening something else or by the way back its head draws, and a button
  /// saying the same thing again would be a second way out of a pane with one.
  it("is a pane rather than a modal, with no second way out", async () => {
    theProfiles();
    const { container } = mountPane(FABLE.id);

    await waitFor(() => screen.getByLabelText("Name"));

    expect(container.querySelector("dialog")).toBeNull();
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });

  /// A path naming a Profile the list has not got — a link somebody kept, or one
  /// removed on another device. Said rather than drawn as a blank form that
  /// would write to an id nothing is saved under.
  it("says so when the path names a profile that is gone", async () => {
    theProfiles();
    mountPane(404);

    await waitFor(() => screen.getByText("That profile is gone."));
    expect(screen.queryByLabelText("Name")).toBeNull();
  });

  it("says so when the server could not be read at all", async () => {
    serving(whenever("/api/ui/profiles", json({ error: "gone" }, 500)));
    mountPane(FABLE.id);

    await waitFor(() => screen.getByText(/Could not read the agent profiles/));
  });
});

describe("the pane the plus opens", () => {
  /// Blank, and standing on nothing the server has said: adding a Profile is
  /// asking about one that does not exist yet, so the pane does not wait on a
  /// read it has no use for.
  it("opens empty", async () => {
    theProfiles();
    const { container } = mountPane("new");

    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("");
    expect(
      (screen.getByLabelText("Models, one per line") as HTMLTextAreaElement)
        .value,
    ).toBe("");
    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();

    // Nothing on the record to say, and nothing to remove: both belong to a
    // Profile that is saved.
    expect(container.querySelector(`.${styles.standing}`)).toBeNull();
    expect(screen.queryByRole("button", { name: "Remove" })).toBeNull();
  });

  /// The account among them, saying which type it is: what the server is sent
  /// is a profile of a type with that type's own fields, rather than a pair
  /// every profile is assumed to have.
  it("sends the name, the models and the account that were typed", async () => {
    const fetching = theProfiles(json("Saved"));
    mountPane("new");

    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/profiles")).toEqual(NEW),
    );
  });

  it("spends the pane once the server took it", async () => {
    theProfiles(json("Saved"));
    const { done } = mountPane("new");

    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(done).toHaveBeenCalled());
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
    const { done } = mountPane("new");

    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    // Beside the form, and the pane still standing: what was typed is still
    // there to be corrected, which is the whole use of being told what was
    // wrong with it.
    await waitFor(() => screen.getByText(said));
    expect(done).not.toHaveBeenCalled();
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
      NEW.name,
    );
  });

  /// A server that could not answer at all, which is the one thing here that is
  /// an error rather than an outcome.
  it("says so when the server could not answer", async () => {
    theProfiles(json({ error: "the database is gone" }, 500));
    mountPane("new");

    fillIn(NEW);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});

describe("removing a profile", () => {
  /// In the pane beside the form rather than on the card: it was a second
  /// control on every row, which put a destructive press beside a list somebody
  /// was only reading.
  it("asks the server to remove the profile the pane is about", async () => {
    const fetching = theProfiles(json("Removed"));
    const { done } = mountPane(FABLE.id);

    fireEvent.click(
      await waitFor(() => screen.getByRole("button", { name: "Remove" })),
    );

    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        `/api/ui/profiles/${FABLE.id}/delete`,
        expect.objectContaining({ method: "POST" }),
      ),
    );
    await waitFor(() => expect(done).toHaveBeenCalled());
  });

  /// Refused rather than taken away from the conversation that chose it: one
  /// pointing at a profile that is not there is a session that fails to start
  /// with nobody watching. Said in the pane, which is where the press was made.
  it("says why a profile in use could not be removed", async () => {
    theProfiles(json("InUse"));
    const { done } = mountPane(FABLE.id);

    fireEvent.click(
      await waitFor(() => screen.getByRole("button", { name: "Remove" })),
    );

    await waitFor(() =>
      screen.getByText(/A conversation is set to run under it/),
    );
    expect(done).not.toHaveBeenCalled();
  });

  it("says so when the server could not answer", async () => {
    theProfiles(json({ error: "the database is gone" }, 500));
    mountPane(FABLE.id);

    fireEvent.click(
      await waitFor(() => screen.getByRole("button", { name: "Remove" })),
    );

    await waitFor(() => screen.getByText(/could not be removed/));
  });
});

describe("a profile whose pair has gone", () => {
  /// The pair was there when it was saved, and a directory can be moved
  /// afterwards. The server reports it on every read; the card is where it is
  /// said, because a list is scanned before it is read.
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
    mountCards();

    await waitFor(() => screen.getByText(FABLE.name));

    const face = theCard(FABLE.name);
    expect(face.classList).toContain(styles.broken);
    expect(face.querySelector(`.${styles.broken}`)!.textContent).toBe(said);
  });

  it("says nothing about a profile whose pair is where it was left", async () => {
    theProfiles();
    mountCards();

    await waitFor(() => screen.getByText(FABLE.name));

    const face = theCard(FABLE.name);
    expect(FABLE.broken).toBeNull();
    expect(face.classList).not.toContain(styles.broken);
    expect(face.querySelector(`.${styles.broken}`)).toBeNull();
  });
});
