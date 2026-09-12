//! The wizard's last step: the fields prefilled from the machine, the one save
//! that writes them, and the press that closes the wizard.
//!
//! Fed from the golden fixtures `cargo test` writes out of the real endpoints —
//! the git step's own read in both shapes it comes in, and the settings before
//! and after a save — so what the step is drawn over is what the server said.
//!
//! Two shapes of test, as the two steps before it have. **What is drawn** is
//! asked of the step on its own. **What a press does** is asked of the whole
//! app, because the answer is which app there is: the last Next takes
//! onboarding mode off, and what that means is the workbench standing at
//! `/compose` where the wizard was.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Gate } from "../src/App";
import type {
  OnboardingView,
  PrefillView,
  SettingsEdit,
  SettingsSaved,
  SettingsView,
} from "../src/api/types";
import { Git } from "../src/setup/Git";
import { SETUP_STEP } from "../src/setup/steps";
import { SET_UP, theWorkbench } from "./bench";
import { json, serving, whenever } from "./serving";
import fresh from "./fixtures/onboarding-fresh.json" with { type: "json" };
import found from "./fixtures/onboarding-git.json" with { type: "json" };
import nothing from "./fixtures/onboarding-git-nothing.json" with { type: "json" };
import saved from "./fixtures/settings-saved.json" with { type: "json" };
import settings from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

/// The three paths this step talks to.
const PREFILL = "/api/ui/onboarding/git";
const SETTINGS = "/api/ui/settings";
const FINISHED = "/api/ui/onboarding/finished";
const ONBOARDING = "/api/ui/onboarding";

/// A machine that could answer every field: an author in its own git config,
/// and a token in the server's environment.
const FOUND = found as PrefillView;

/// And one that could answer none, which is three empty fields.
const NOTHING = nothing as PrefillView;

/// A Verkstead that has been told nothing, which is what the wizard runs over.
const UNSET = unset as SettingsView;

/// And one that has been told an author already, which is what a field shows
/// instead of anything found.
const TOLD = settings as SettingsView;

/// What the save answers: the files as they now stand, and GitHub naming the
/// account the token authenticates as.
const SAVED = saved as SettingsSaved;

/// The same with no token to verify, which is a save that has nothing to be
/// read and goes straight on.
const UNVERIFIED: SettingsSaved = { ...SAVED, verified: null };

/// And one whose token GitHub would not answer for.
const REFUSED: SettingsSaved = {
  ...SAVED,
  verified: { Refused: { why: "gh said: HTTP 401: Bad credentials" } },
};

/// A bare machine, which is the reading the wizard is a page under.
const FRESH = fresh as OnboardingView;

/// No retries: a test that asked for a refusal should see it at once.
///
/// Held in a variable by whoever mounts rather than called in the `client`
/// prop, for the reason the accounts step's own suite holds one: a call in a
/// JSX attribute is read again on every access, and a second client would be a
/// second cache — the query in one and what the press wrote in the other.
function client(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

/// The step alone under one client, which is what every mount here is.
function drawn(queries: QueryClient) {
  return render(() => (
    <QueryClientProvider client={queries}>
      <Git />
    </QueryClientProvider>
  ));
}

/// The step alone, over a machine that offers `prefill` and a Verkstead that
/// has been told `told`.
function mount(prefill: PrefillView, told: SettingsView = UNSET) {
  const fetching = serving(
    whenever(PREFILL, json(prefill)),
    whenever(SETTINGS, json(told)),
    whenever(SETTINGS, json(SAVED), "POST"),
    whenever(FINISHED, json(SET_UP), "POST"),
  );

  return { ...drawn(client()), fetching };
}

/// One of the three boxes.
function field(container: ParentNode, id: string): HTMLInputElement {
  return container.querySelector<HTMLInputElement>(`#${id}`)!;
}

/// And what is said under it about where its value came from.
function source(container: ParentNode, id: string): string | undefined {
  return (
    field(container, id).parentElement?.querySelector("p")?.textContent ??
    undefined
  );
}

/// The press onwards, found by its own words the way the steps before it are.
function onwards(container: ParentNode): HTMLButtonElement {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Next",
  )!;
}

/// What the page put on the wire when it saved the settings.
function sent(fetching: ReturnType<typeof serving>): SettingsEdit {
  const written = fetching.mock.calls.find(
    ([asked, init]) => String(asked) === SETTINGS && init?.method === "POST",
  );

  expect(written, "expected the settings to have been saved").toBeTruthy();

  return JSON.parse(String(written![1]?.body)) as SettingsEdit;
}

/// Whether anything was written to a path at all.
function wrote(fetching: ReturnType<typeof serving>, path: string): boolean {
  return fetching.mock.calls.some(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
  window.history.replaceState({}, "", "/");
});

describe("what the fields open with", () => {
  /// Each prefill is labelled with where it was found: somebody is being asked
  /// to confirm a value they did not type, and a box that only showed the value
  /// would be asking them to trust it blind.
  it("prefills each field from the machine and says where each came from", async () => {
    const { container } = mount(FOUND);

    await waitFor(() =>
      expect(field(container, "setup-author-name").value).toBe("Ada Lovelace"),
    );

    expect(source(container, "setup-author-name")).toContain(
      "git config --global",
    );
    expect(field(container, "setup-author-email").value).toBe(
      "ada@example.com",
    );
    expect(field(container, "setup-github-token").value).toBe(
      "ghp_intheenvironment",
    );
    expect(source(container, "setup-github-token")).toContain("GH_TOKEN");
  });

  /// A machine that could say nothing leaves them empty: a field to type in
  /// rather than a wrong one to correct.
  it("leaves a field the server found nothing for empty", async () => {
    const { container } = mount(NOTHING);

    await waitFor(() => expect(onwards(container)).toBeTruthy());

    expect(field(container, "setup-author-name").value).toBe("");
    expect(field(container, "setup-author-email").value).toBe("");
    expect(field(container, "setup-github-token").value).toBe("");
    expect(source(container, "setup-author-name")).toBeUndefined();
  });

  /// And a value Verkstead already holds is the one shown: the server sends no
  /// prefill for a field it has been told, and what is in front of the human is
  /// then what is written down.
  it("shows what Verkstead was told rather than anything found", async () => {
    const { container } = mount(NOTHING, TOLD);

    await waitFor(() =>
      expect(field(container, "setup-author-name").value).toBe("Ada Lovelace"),
    );

    expect(source(container, "setup-author-name")).toBeUndefined();
  });

  /// git refuses a commit with no author, so both halves are required — which
  /// is the one thing that holds this step's Next.
  it("refuses Next until both halves of the author are there", async () => {
    const { container } = mount(NOTHING);

    await waitFor(() => expect(onwards(container).disabled).toBe(true));

    fireEvent.input(field(container, "setup-author-name"), {
      target: { value: "Ada Lovelace" },
    });
    expect(onwards(container).disabled).toBe(true);

    fireEvent.input(field(container, "setup-author-email"), {
      target: { value: "ada@example.com" },
    });
    expect(onwards(container).disabled).toBe(false);
  });
});

describe("what Next saves", () => {
  /// Nothing is written until it is pressed: a prefill is a proposal, and the
  /// press is the human telling Verkstead it is right.
  it("writes nothing until it is pressed", async () => {
    const { container, fetching } = mount(FOUND);

    await waitFor(() =>
      expect(field(container, "setup-author-name").value).toBe("Ada Lovelace"),
    );

    expect(wrote(fetching, SETTINGS)).toBe(false);
    expect(wrote(fetching, FINISHED)).toBe(false);
  });

  /// And the press is the settings page's own save: one request, both files,
  /// and everything in `config.yaml` this step is not about riding along as it
  /// stands rather than being emptied.
  it("saves the three fields through the whole-settings save", async () => {
    const { container, fetching } = mount(FOUND, TOLD);

    await waitFor(() =>
      expect(field(container, "setup-author-email").value).toBe(
        "ada@example.com",
      ),
    );

    fireEvent.input(field(container, "setup-author-name"), {
      target: { value: "Ada King" },
    });
    fireEvent.click(onwards(container));

    await waitFor(() => expect(wrote(fetching, SETTINGS)).toBe(true));

    expect(sent(fetching)).toEqual({
      git_author: { name: "Ada King", email: "ada@example.com" },
      github_token: { Set: { token: "ghp_intheenvironment" } },
      share_on_done: TOLD.share_on_done,
      // The one thing a save can be refused over, left alone: a wizard turned
      // down over a pattern somebody hand-edited into the file weeks ago would
      // be a wizard nobody could finish.
      ignored_comments: "Keep",
      rust_build_cache: { enabled: true, size: "50G" },
      cleanup: {
        trim: { enabled: true, days: "5" },
        delete: { enabled: true, days: "90" },
      },
      conflict_resolution: TOLD.conflict_resolution,
      sandbox_binds: [
        "/var/cache/verkstead-node",
        "verkstead=/var/cache/verkstead-cargo",
      ],
    });
  });

  /// An empty token field is the credentials left alone rather than taken away,
  /// which is the same thing it means on the settings page.
  it("leaves the token alone where the field is empty", async () => {
    const { container, fetching } = mount(NOTHING);

    await waitFor(() => expect(onwards(container)).toBeTruthy());

    fireEvent.input(field(container, "setup-author-name"), {
      target: { value: "Ada Lovelace" },
    });
    fireEvent.input(field(container, "setup-author-email"), {
      target: { value: "ada@example.com" },
    });
    fireEvent.click(onwards(container));

    await waitFor(() => expect(wrote(fetching, SETTINGS)).toBe(true));
    expect(sent(fetching).github_token).toBe("Keep");
  });
});

describe("what the save answers", () => {
  /// The login is shown rather than typed, and the wizard waits one more press
  /// for it to be read: a token that authenticates as the *wrong* account is
  /// the mistake nothing else here would catch.
  it("shows the account the token authenticates as, and does not close yet", async () => {
    const { container, fetching } = mount(FOUND);

    await waitFor(() =>
      expect(field(container, "setup-author-name").value).toBe("Ada Lovelace"),
    );

    fireEvent.click(onwards(container));

    await waitFor(() => screen.getByText("ada"));
    expect(wrote(fetching, FINISHED)).toBe(false);

    // The press that follows the save, once the save is over: the button is
    // refused while a request of its own is out, the way every press in this
    // app is.
    await waitFor(() => expect(onwards(container).disabled).toBe(false));
    fireEvent.click(onwards(container));
    await waitFor(() => expect(wrote(fetching, FINISHED)).toBe(true));
  });

  /// A save with no token to verify has nothing to be read, so the press that
  /// made it is the press that closes the wizard.
  it("goes straight on where there was no token to verify", async () => {
    const fetching = serving(
      whenever(PREFILL, json(NOTHING)),
      whenever(SETTINGS, json(UNSET)),
      whenever(SETTINGS, json(UNVERIFIED), "POST"),
      whenever(FINISHED, json(SET_UP), "POST"),
    );

    const { container } = drawn(client());

    await waitFor(() => expect(onwards(container)).toBeTruthy());

    fireEvent.input(field(container, "setup-author-name"), {
      target: { value: "Ada Lovelace" },
    });
    fireEvent.input(field(container, "setup-author-email"), {
      target: { value: "ada@example.com" },
    });
    fireEvent.click(onwards(container));

    await waitFor(() => expect(wrote(fetching, FINISHED)).toBe(true));
  });

  /// And a token GitHub would not answer for says so, with everything the human
  /// typed still in front of them.
  it("says why a token could not be verified, and keeps what was typed", async () => {
    const fetching = serving(
      whenever(PREFILL, json(NOTHING)),
      whenever(SETTINGS, json(UNSET)),
      whenever(SETTINGS, json(REFUSED), "POST"),
      whenever(FINISHED, json(SET_UP), "POST"),
    );

    const { container } = drawn(client());

    await waitFor(() => expect(onwards(container)).toBeTruthy());

    for (const [id, value] of [
      ["setup-author-name", "Ada Lovelace"],
      ["setup-author-email", "ada@example.com"],
      ["setup-github-token", "ghp_thewrongone"],
    ] as const) {
      fireEvent.input(field(container, id), { target: { value } });
    }

    fireEvent.click(onwards(container));

    await waitFor(() =>
      expect(container.textContent).toContain("HTTP 401: Bad credentials"),
    );

    expect(wrote(fetching, FINISHED)).toBe(false);
    expect(field(container, "setup-author-name").value).toBe("Ada Lovelace");
    expect(field(container, "setup-github-token").value).toBe(
      "ghp_thewrongone",
    );
  });
});

describe("the last Next", () => {
  /// Which is the whole wizard ending: the mode goes off for this run and the
  /// app lands on the compose page. Driven through the gate, because what is
  /// being asked is which of the two apps the browser is holding — see
  /// `setup-routes.test.tsx`, whose subject is what each of them answers.
  it("takes the mode off and lands the app on /compose", async () => {
    localStorage.setItem(SETUP_STEP, "git");
    window.history.replaceState({}, "", "/setup");

    // The verdict as the server would answer it either side of the press: the
    // mode is on until the wizard says it has finished, and off from then on.
    let over = false;

    const fetching = theWorkbench(
      whenever(ONBOARDING, () => json(over ? SET_UP : FRESH)()),
      whenever(PREFILL, json(NOTHING)),
      whenever(SETTINGS, json(UNSET)),
      whenever(SETTINGS, json(UNVERIFIED), "POST"),
      whenever(
        FINISHED,
        () => {
          over = true;
          return json(SET_UP)();
        },
        "POST",
      ),
    );

    const queries = client();

    render(() => (
      <QueryClientProvider client={queries}>
        <Gate />
      </QueryClientProvider>
    ));

    const container = document.body;

    await waitFor(() =>
      expect(field(container, "setup-author-name")).toBeTruthy(),
    );

    fireEvent.input(field(container, "setup-author-name"), {
      target: { value: "Ada Lovelace" },
    });
    fireEvent.input(field(container, "setup-author-email"), {
      target: { value: "ada@example.com" },
    });
    fireEvent.click(onwards(container));

    await waitFor(() => expect(wrote(fetching, FINISHED)).toBe(true));

    // The workbench, at the page the wizard hands over to.
    await waitFor(() => screen.getByText("Save as draft"));
    expect(window.location.pathname).toBe("/compose");
    expect(screen.queryByText("Set Verkstead up")).toBeNull();
  });
});
