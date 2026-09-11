//! The wizard's second step: the accounts this machine already has, the press
//! that saves the ticked ones as Agent Profiles, and the form for an account
//! that was not found.
//!
//! Fed from the golden fixtures `cargo test` writes out of the real endpoint, so
//! what the step is drawn over is what the server actually said: the part-way
//! reading carries one account whose harness is on the machine and one whose is
//! not, which is the tick and the greyed row in one payload.
//!
//! Two shapes of test, as the step before it has. **What is drawn** is asked of
//! the step on its own, handed a reading. **What a press does** is asked of the
//! whole page, because the answer involves the server: a Profile is written and
//! the wizard is read again, and it is that second reading that releases
//! Next.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { OnboardingView, ProfileEdit } from "../src/api/types";
import { AGENT_NAME } from "../src/agents";
import { KNOWN_MODELS, prettify } from "../src/models";
import { Accounts } from "../src/setup/Accounts";
import { SetupPage } from "../src/setup/SetupPage";
import { SETUP_STEP } from "../src/setup/steps";
import { json, serving, whenever } from "./serving";
import partWay from "./fixtures/onboarding-part-way.json" with { type: "json" };

/// The one path this step writes to. What it reads is the wizard’s own, served
/// in whatever order the page asks for it.
const PROFILES = "/api/ui/profiles";

/// A machine with what a session needs on it, a Claude Code account in the
/// server's home and a Codex account beside it — and no Claude Code account
/// saved as a Profile yet, which is what this step is for.
const PART_WAY = partWay as OnboardingView;

/// The same machine with nothing ever logged in on it.
const NOTHING_FOUND: OnboardingView = { ...PART_WAY, accounts: [] };

/// And the same machine once a Profile has been saved, which is the step met.
const A_PROFILE: OnboardingView = {
  ...NOTHING_FOUND,
  steps: { ...PART_WAY.steps, accounts: true },
};

/// No retries: a test that asked for a refusal should see it at once.
///
/// Held in a variable by whoever mounts, rather than called in the `client`
/// prop: a call in a JSX attribute is read again on every access, and a second
/// client would be a second cache — the query in one and the invalidation the
/// save makes in the other.
function client(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

/// The step alone, over a machine that stands as `machine` says.
///
/// A query client because saving is a mutation and what it invalidates is the
/// page's own reading; no router, the step reading nothing off the URL.
function mount(machine: OnboardingView) {
  const onwards = vi.fn();
  const queries = client();

  return {
    ...render(() => (
      <QueryClientProvider client={queries}>
        <Accounts reading={machine} onwards={onwards} />
      </QueryClientProvider>
    )),
    onwards,
  };
}

/// And the whole wizard with this step open, which is what a press that talks to
/// the server has to be asked of.
function mountPage() {
  localStorage.setItem(SETUP_STEP, "accounts");
  const queries = client();

  return render(() => (
    <QueryClientProvider client={queries}>
      <SetupPage />
    </QueryClientProvider>
  ));
}

/// One account's row.
function row(container: ParentNode, agent: string): HTMLElement {
  return container.querySelector<HTMLElement>(`[data-account="${agent}"]`)!;
}

/// And its tick.
function tick(container: ParentNode, agent: string): HTMLInputElement {
  return row(container, agent).querySelector("input[type=checkbox]")!;
}

/// The link the form for an account nothing found stands behind, and the press
/// that opens it: it is the section's own heading drawn as a link, so it is
/// found by the words the heading has.
function naming(container: ParentNode): HTMLButtonElement | undefined {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Or name an account yourself",
  );
}

/// The press onwards, found by its own words the way the step before it is.
function onwards(container: ParentNode): HTMLButtonElement {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Next",
  )!;
}

/// What the page put on the wire when it wrote to the profiles.
function sent(fetching: ReturnType<typeof serving>): ProfileEdit {
  const written = fetching.mock.calls.find(
    ([asked, init]) => String(asked) === PROFILES && init?.method === "POST",
  );

  expect(written, "expected a profile to have been written").toBeTruthy();

  return JSON.parse(String(written![1]?.body)) as ProfileEdit;
}

/// Every model this build knows for one harness, which is what an autoconfigured
/// Profile is given.
function models(agent: string): string[] {
  return KNOWN_MODELS.filter((model) => model.agent === agent).map(
    (model) => model.id,
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the accounts that were found", () => {
  /// One per harness at most, each shape being a fixed path under a home — so
  /// the list is short and every row is an account somebody logged in once.
  it("offers each one under its harness, with where it is kept", () => {
    const { container } = mount(PART_WAY);

    expect(
      [...container.querySelectorAll<HTMLElement>("[data-account]")].map(
        (found) => found.dataset.account,
      ),
    ).toEqual(["Claude", "Codex"]);

    const claude = row(container, "Claude");

    expect(claude.textContent).toContain(AGENT_NAME.Claude);
    expect(claude.textContent).toContain("~/.claude");
    expect(claude.textContent).toContain("~/.claude.json");
  });

  /// The harness being there is what says an account is one to take on: a
  /// Profile of a backend that is not installed is one no session could be
  /// launched under.
  it("ticks the one whose harness is on the machine and greys the one that is not", () => {
    const { container } = mount(PART_WAY);

    expect(row(container, "Claude").dataset.harness).toBe("yes");
    expect(tick(container, "Claude").checked).toBe(true);
    expect(tick(container, "Claude").disabled).toBe(false);

    expect(row(container, "Codex").dataset.harness).toBe("no");
    expect(tick(container, "Codex").checked).toBe(false);
    expect(tick(container, "Codex").disabled).toBe(true);
    expect(row(container, "Codex").textContent).toContain(
      `${AGENT_NAME.Codex} is not on this machine.`,
    );
  });
});

describe("taking an account on", () => {
  /// No name because a home holds one account per harness, and every model
  /// because a Profile with none is refused — both the settings page's to
  /// change afterwards.
  it("saves the ticked account with no name and every model of its harness", async () => {
    const fetching = serving(
      whenever(PROFILES, json("Saved"), "POST"),
      json(PART_WAY),
    );
    const { container, onwards: next } = mount(PART_WAY);

    fireEvent.click(onwards(container));

    await waitFor(() => expect(next).toHaveBeenCalled());

    expect(sent(fetching)).toEqual({
      name: null,
      account: PART_WAY.accounts[0]!.account,
      models: models("Claude"),
    });
  });

  /// The greyed row is not saved with the ticked one: it is drawn so that it is
  /// not a surprise later, and there is nothing to launch it with.
  it("writes nothing for the account whose harness is missing", async () => {
    const fetching = serving(
      whenever(PROFILES, json("Saved"), "POST"),
      json(PART_WAY),
    );
    const { container, onwards: next } = mount(PART_WAY);

    fireEvent.click(onwards(container));

    await waitFor(() => expect(next).toHaveBeenCalled());

    expect(
      fetching.mock.calls.filter(
        ([asked, init]) => String(asked) === PROFILES && init?.method === "POST",
      ),
    ).toHaveLength(1);
  });

  /// A step opened again and pressed twice: the harness already has the unnamed
  /// Profile this press would have made, which is the press having already
  /// happened rather than a refusal to put in front of anybody.
  it("takes a harness that already has an unnamed Profile as saved", async () => {
    serving(whenever(PROFILES, json("DefaultTaken"), "POST"), json(PART_WAY));
    const { container, onwards: next } = mount(PART_WAY);

    fireEvent.click(onwards(container));

    await waitFor(() => expect(next).toHaveBeenCalled());
    expect(container.textContent).not.toContain("already has a profile");
  });

  /// Every other refusal is about the account itself, and is said where the
  /// press was made rather than swallowed.
  it("says why an account could not be saved, and stays on the step", async () => {
    serving(whenever(PROFILES, json("DirMissing"), "POST"), json(PART_WAY));
    const { container, onwards: next } = mount(PART_WAY);

    fireEvent.click(onwards(container));

    await waitFor(() =>
      expect(container.textContent).toContain(
        "There is nothing at the claude directory's path.",
      ),
    );
    expect(next).not.toHaveBeenCalled();
  });
});

describe("the press onwards", () => {
  /// Clearing the mode with nothing saved would land somebody on exactly the
  /// empty state a skip was rejected for.
  it("is refused once every account is unticked", () => {
    const { container } = mount(PART_WAY);

    expect(onwards(container).disabled).toBe(false);

    fireEvent.click(tick(container, "Claude"));

    expect(tick(container, "Claude").checked).toBe(false);
    expect(onwards(container).disabled).toBe(true);
    expect(container.textContent).toContain(
      "Verkstead runs nothing without an Agent Profile.",
    );
  });

  /// And a Profile that is already saved is a step there is nothing left to do
  /// on: the press goes straight on rather than writing anything.
  it("goes on without writing anything where a Profile is saved already", async () => {
    const fetching = serving(json(A_PROFILE));
    const { container, onwards: next } = mount(A_PROFILE);

    expect(onwards(container).disabled).toBe(false);

    fireEvent.click(onwards(container));

    await waitFor(() => expect(next).toHaveBeenCalled());
    expect(
      fetching.mock.calls.filter(([, init]) => init?.method === "POST"),
    ).toEqual([]);
  });
});

describe("a home with no account in it", () => {
  it("says what to run to make one, and offers the link under it", () => {
    const { container } = mount(NOTHING_FOUND);

    expect(container.querySelector("[data-account]")).toBeNull();
    expect(container.textContent).toContain(
      "No agent account was found in this server's home.",
    );

    // One line per harness that is on the machine, because those are the ones
    // that can be run at all.
    expect(
      [...container.querySelectorAll<HTMLElement>("[data-login]")].map(
        (login) => login.dataset.login,
      ),
    ).toEqual(["Claude"]);
    expect(container.querySelector("[data-login] pre")!.textContent).toBe(
      "claude",
    );

    // And the way to the same form the settings page saves a Profile with, for
    // an account that is kept somewhere else entirely — a link here as
    // everywhere else, because the empty state already says what to run and
    // this page reads the machine again every ten seconds.
    expect(naming(container)).toBeTruthy();
    expect(screen.queryByLabelText(/^Name/)).toBeNull();
  });
});

/// The form for an account kept somewhere other than this server's home, which
/// is the uncommon way past the step and so is not standing open under the rows.
describe("the link the form stands behind", () => {
  it("keeps the form off the step until it is pressed", () => {
    const { container } = mount(PART_WAY);

    expect(screen.queryByLabelText(/^Name/)).toBeNull();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();

    fireEvent.click(naming(container)!);

    expect(screen.getByLabelText(/^Name/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
  });

  /// The link is the heading, so what is left where it stood is the heading
  /// itself — and the section does not shut again, there being nothing on this
  /// step that wants the room back.
  it("leaves the heading in its place, and no way back", () => {
    const { container } = mount(PART_WAY);

    fireEvent.click(naming(container)!);

    expect(naming(container)).toBeUndefined();
    expect(
      [...container.querySelectorAll("h3")].map((head) => head.textContent),
    ).toContain("Or name an account yourself");
  });
});

/// The two things about this step that are the whole page's: an account that
/// appears while somebody is looking, and a Profile saved from the form
/// releasing the press.
describe("while the step is open", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  /// The frame re-reads while the open step is unmet, which is what makes a
  /// `claude` logged in in another window a row here without anybody touching
  /// the page.
  it("picks an account up within ten seconds of it appearing", async () => {
    serving(json(NOTHING_FOUND), json(PART_WAY));
    const { container } = mountPage();

    await waitFor(() =>
      expect(container.textContent).toContain(
        "No agent account was found in this server's home.",
      ),
    );

    await vi.advanceTimersByTimeAsync(10_000);

    await waitFor(() => expect(row(container, "Claude")).toBeTruthy());
    expect(tick(container, "Claude").checked).toBe(true);
  });

  /// And that re-read is not allowed to shut a section somebody has just
  /// opened. Whether the form is out is this step's own signal rather than
  /// anything read off the machine — the same way a tick somebody has taken off
  /// is — and a page that put the form away every ten seconds would be taking
  /// the step back off whoever was filling it in.
  it("leaves the form open through a re-read", async () => {
    serving(json(NOTHING_FOUND), json(PART_WAY));
    const { container } = mountPage();

    await waitFor(() => expect(naming(container)).toBeTruthy());
    fireEvent.click(naming(container)!);
    expect(screen.getByLabelText(/^Name/)).toBeTruthy();

    await vi.advanceTimersByTimeAsync(10_000);

    // The re-read landed — the account it found is a row now — and the form is
    // still where the press left it.
    await waitFor(() => expect(row(container, "Claude")).toBeTruthy());
    expect(screen.getByLabelText(/^Name/)).toBeTruthy();
    expect(naming(container)).toBeUndefined();
  });
});

/// The other way past this step, which needs no clock at all: a Profile
/// saved from the form is a read of the wizard, and that reading is what
/// releases the press.
describe("the form under the rows", () => {
  /// An account nothing found, named by hand: the way past this step for a
  /// machine whose accounts are kept somewhere other than this server’s home.
  it("releases the press once a Profile is saved from the form", async () => {
    serving(
      whenever(PROFILES, json("Saved"), "POST"),
      json(NOTHING_FOUND),
      json(A_PROFILE),
    );
    const { container } = mountPage();

    await waitFor(() => expect(onwards(container)).toBeTruthy());
    expect(onwards(container).disabled).toBe(true);

    fireEvent.click(naming(container)!);

    fireEvent.click(screen.getByLabelText(prettify("claude-sonnet-5")));
    fireEvent.input(screen.getByLabelText(/Claude directory/), {
      target: { value: "/home/you/accounts/work/.claude" },
    });
    fireEvent.input(screen.getByLabelText(/Config file/), {
      target: { value: "/home/you/accounts/work/.claude.json" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(onwards(container).disabled).toBe(false));
  });
});

