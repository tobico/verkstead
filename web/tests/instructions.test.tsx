//! The one text every session is given, on the settings page: what the card
//! says of it, what the box in its pane holds, and what a Save puts on the
//! wire.
//!
//! Two halves mounted apart, because that is what they are: a card in the
//! middle pane saying whether there is a text and what it reaches, and the box
//! that rewrites it in the details pane it opens.
//!
//! One press saves, unlike the checkboxes beside this section and like the
//! durations on the Cleanup pane: the text is typed, so nothing is committed
//! while somebody is halfway through a sentence. That save sends the whole of
//! the settings edit — the author as it stands, the token untouched, the paths
//! and the Cleanup where the read left them — because the server writes both
//! files in one request, and that is what these check is not lost.
//!
//! What the text itself has to survive is the round trip unchanged: the blank
//! line between two paragraphs and the indent of a list are the writing rather
//! than slips in it, and a harness is handed these words.
//!
//! The read is a fixture the server's own tests wrote, so what the page is
//! drawn from is the shape the endpoint really answers with:
//! `settings.json` is the Verkstead that has been told everything, and
//! `settings-unset.json` is the one nobody has been to — which is the empty box
//! a fresh installation draws.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import {
  InstructionsCard,
  InstructionsPane,
} from "../src/settings/Instructions";
import styles from "../src/settings/Instructions.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The rest of `config.yaml` as every save from this pane sends it: what the
/// read said, left exactly where it was.
const REST = {
  git_author: TOLD.git_author,
  github_token: "Keep",
  // The rules ride along as an action rather than a value: nothing this form
  // does says anything about them — see [`IgnoredCommentsEdit`].
  ignored_comments: "Keep",
  rust_build_cache: {
    enabled: TOLD.rust_build_cache.enabled,
    size: TOLD.rust_build_cache.size,
  },
  // And the Cleanup as the read left it, each duration as the string a form
  // holds — see [`heldCleanup`].
  cleanup: {
    trim: { enabled: true, days: "5" },
    delete: { enabled: true, days: "90" },
  },
  conflict_resolution: TOLD.conflict_resolution,
  share_on_done: TOLD.share_on_done,
  sandbox_binds: ["/var/cache/verkstead-node", "/var/cache/verkstead-cargo"],
};

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Whichever half of the section a test is about, over one query client: both
/// halves read the same file, so a test mounting the pair is reading it once,
/// exactly as the page does.
function mounting(what: () => JSX.Element) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <InstructionsCard open={open} press={press} />),
    press,
  };
}

function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <InstructionsPane back={back} />), back };
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// The same settings with another text in them, which is what a save answers
/// with.
function saying(standing: SettingsView, instructions: string): SettingsSaved {
  return {
    settings: { ...standing, instructions },
    verified: null,
    refused: [],
  };
}

function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// The box itself, by the label the pane gives it.
function theBox(): HTMLTextAreaElement {
  return screen.getByLabelText("Instructions") as HTMLTextAreaElement;
}

describe("the card", () => {
  /// The one thing somebody scanning the page is after: whether there is a
  /// text, and what it reaches either way.
  it("says what the text reaches, and where it sits", async () => {
    theSettings(TOLD);
    mountCard();

    await waitFor(() => screen.getByText(/Given to every session/));
    expect(screen.getByText(/Given to every session/).textContent).toContain(
      "above whatever instructions its repository carries",
    );
  });

  /// And a Verkstead nobody has typed one into says so, rather than saying
  /// nothing: an empty section on a settings page reads as one that is broken.
  it("says so where nobody has typed one", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/Nothing is configured/));
    expect(screen.getByText(/Nothing is configured/).textContent).toContain(
      "only what its repository carries",
    );
  });

  it("opens the pane on a press", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    const face = await waitFor(() => {
      const drawn = container.querySelector<HTMLElement>(
        `.${styles.instructionsCard}`,
      );
      expect(drawn, "expected the card to be drawn").not.toBeNull();
      return drawn!;
    });

    fireEvent.click(face);
    expect(press).toHaveBeenCalled();
  });
});

describe("the pane", () => {
  /// The text as it was typed, line breaks and all: this is the whole of what
  /// the setting is for, and a box that reflowed it would be showing the human
  /// something other than what a session will be handed.
  it("draws the text the file holds, verbatim", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => theBox());
    expect(theBox().value).toBe(TOLD.instructions);
  });

  /// And a fresh installation draws the section rather than failing at it: the
  /// box is empty because nobody has typed one, which is a setting nobody has
  /// made and not an error.
  it("draws an empty box where nobody has typed one", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => theBox());
    expect(theBox().value).toBe("");
    expect(screen.queryByText(/Could not read the settings/)).toBeNull();
  });

  /// Both halves of what the section is for, said where somebody is about to
  /// type into it: where the text goes, and what it does not displace.
  it("says what the text reaches and what it leaves alone", async () => {
    theSettings(TOLD);
    mountPane();

    const note = await waitFor(() =>
      screen.getByText(/Given to every session as the agent's own/),
    );
    expect(note.textContent).toContain("whatever agent its profile runs");
    expect(note.textContent).toContain("read as it always was, under this");
  });

  /// The press sends the box, and everything else as the read left it — one
  /// request writes the whole of `config.yaml`, so a section that spoke only
  /// for itself would be a section that emptied the rest.
  it("sends what was typed, with the rest of the file as it stands", async () => {
    const written = "Prefer the smallest change.\n\nAnd:\n  - run the tests\n";
    const fetching = theSettings(TOLD, json(saying(TOLD, written)));
    mountPane();
    await waitFor(() => theBox());

    fireEvent.input(theBox(), { target: { value: written } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({ ...REST, instructions: written }),
    );
  });

  /// And the box follows the answer afterwards rather than going on holding
  /// what was typed: the save landed, so the server is what the field is a view
  /// of again.
  it("lets go of what was typed once the save lands", async () => {
    const written = "Prefer the smallest change.";
    const fetching = theSettings(TOLD, json(saying(TOLD, written)));
    mountPane();
    await waitFor(() => theBox());

    fireEvent.input(theBox(), { target: { value: written } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    await waitFor(() => expect(theBox().value).toBe(written));
  });

  /// Clearing the box is how the text is taken off — there is nothing else to
  /// press, and an empty text is what the server writes the key away over.
  it("sends an empty text where the box was cleared", async () => {
    const fetching = theSettings(TOLD, json(saying(TOLD, "")));
    mountPane();
    await waitFor(() => theBox());

    fireEvent.input(theBox(), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({ ...REST, instructions: "" }),
    );
  });

  /// A save that would not land says so and leaves what was typed where it is:
  /// the words are the human's, and a form that threw them away on a failed
  /// request would be one nobody could retry.
  it("says a save that failed, and keeps what was typed", async () => {
    const written = "Prefer the smallest change.";
    theSettings(TOLD, () =>
      Promise.resolve(new Response("nope", { status: 503 })),
    );
    mountPane();
    await waitFor(() => theBox());

    fireEvent.input(theBox(), { target: { value: written } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/could not be saved/));
    expect(theBox().value).toBe(written);
  });
});
