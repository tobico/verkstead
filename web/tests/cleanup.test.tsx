//! What becomes of an archived conversation, on the settings page: what the
//! card says of it, what the switches in its pane put on the wire, and the two
//! durations beside them.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying what happens and when, and the controls that change it in the
//! details pane it opens.
//!
//! Two rows and two kinds of press. A switch is its own save, because a switch
//! that needed a second one is not a switch; a duration is typed, so it waits
//! for a Save. Every one of those saves sends the whole of the settings edit —
//! the author as it stands, the token untouched and the other row where the read
//! left it — because the server writes both files in one request, and that is
//! what these check is not lost.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with: `settings.json` is the
//! Verkstead that has been told everything — trimming at five days and deleting
//! at ninety — and `settings-unset.json` is the one nobody has been to, which is
//! the trim on at three and the delete off at thirty.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CleanupView, SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import { CleanupCard, CleanupPane } from "../src/settings/Cleanup";
import styles from "../src/settings/Cleanup.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The paths the fixture holds, as a save puts them back on the wire — every
/// section's save carries them, because one request writes the whole of
/// `config.yaml`.
const PATHS = {
  watched_paths: ["/home/ada/src"],
  sandbox_binds: [
    "/var/cache/verkstead-node",
    "verkstead=/var/cache/verkstead-cargo",
  ],
};

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
  conflict_resolution: TOLD.conflict_resolution,
  share_on_done: TOLD.share_on_done,
  ...PATHS,
};

/// The same settings with the Cleanup somewhere else — what a save answers with,
/// and what a fixture of a switch flipped is drawn from.
function cleaning(standing: SettingsView, cleanup: CleanupView): SettingsView {
  return { ...standing, cleanup };
}

/// One row of it moved, the other left where it was.
function step(
  standing: SettingsView,
  which: "trim" | "delete",
  moved: Partial<CleanupView["trim"]>,
): SettingsView {
  return cleaning(standing, {
    ...standing.cleanup,
    [which]: { ...standing.cleanup[which], ...moved },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Whichever half of the section a test is about, over one query client: both
/// halves read the same two files, so a test mounting the pair is reading them
/// once, exactly as the page does.
function mounting(what: () => JSX.Element) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

/// The card in the middle pane, and what pressing it asked for.
function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <CleanupCard open={open} press={press} />),
    press,
  };
}

/// The controls in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <CleanupPane back={back} />), back };
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// What a save answers with, which is the settings as they now stand.
function answering(standing: SettingsView): SettingsSaved {
  return { settings: standing, verified: null, refused: [] };
}

function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// The card itself, once it is drawn — waited for, because it stands on a read.
async function theCard(container: ParentNode): Promise<HTMLElement> {
  return await waitFor(() => {
    const face = container.querySelector<HTMLElement>(`.${styles.cleanupCard}`);
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

/// The two switches, in the order the pane draws them.
function switches(): HTMLInputElement[] {
  return screen.getAllByRole("switch") as HTMLInputElement[];
}

const theTrim = () => switches()[0]!;
const theDelete = () => switches()[1]!;

describe("the card", () => {
  /// The shape of the whole section, said in the two lines somebody scanning
  /// the page reads: nobody has to open anything for a trim to happen, and
  /// nothing is ever deleted for somebody who has not asked.
  it("says what happens to an archived conversation and when", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/trimmed/));
    expect(screen.getByText(/trimmed/).textContent).toContain("3 days");
    expect(screen.getByText(/never deleted/)).toBeTruthy();
  });

  it("says the durations somebody chose", async () => {
    theSettings(TOLD);
    mountCard();

    await waitFor(() => screen.getByText(/trimmed/));
    expect(screen.getByText(/trimmed/).textContent).toContain("5 days");
    expect(screen.getByText(/deleted for good/).textContent).toContain(
      "90 days",
    );
  });

  it("says so where the trim is switched off", async () => {
    theSettings(step(TOLD, "trim", { enabled: false }));
    mountCard();

    await waitFor(() => screen.getByText(/never trimmed/));
  });

  /// The one thing the two numbers can say that is worth pointing out, and it
  /// is a reading rather than a fault: the clocks are independent, so a delete
  /// sooner than the trim simply arrives first.
  it("says when the delete lands before the trim ever would", async () => {
    theSettings(step(TOLD, "delete", { days: 2 }));
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/The delete comes first/));
    expect(container.querySelector(`.${styles.ordering}`)).not.toBeNull();
  });

  it("says nothing about the ordering while the delete is off", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/never deleted/));
    expect(screen.queryByText(/The delete comes first/)).toBeNull();
  });

  it("opens the pane when it is pressed", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    const face = await theCard(container);
    fireEvent.click(face);

    expect(press).toHaveBeenCalled();
  });

  it("reads as open while its pane is", async () => {
    theSettings(TOLD);
    const { container } = mountCard(true);

    const face = await theCard(container);
    expect(face.classList).toContain(card.open);
    expect(face.getAttribute("aria-pressed")).toBe("true");
  });

  it("says so when the server could not be read at all", async () => {
    serving(() =>
      Promise.resolve(
        new Response("nope", { status: 500, statusText: "Server Error" }),
      ),
    );
    mountCard();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("the cleanup as the pane draws it", () => {
  /// Where the switches sit rather than whether anybody has touched them, which
  /// is what the two defaults falling the two different ways means.
  it("reads as trimming and not deleting where nothing has been configured", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => expect(theTrim().checked).toBe(true));
    expect(theDelete().checked).toBe(false);
  });

  /// A duration nobody configured is the default drawn as a placeholder rather
  /// than as text somebody typed — the field says what will happen without
  /// claiming anybody chose it.
  it("draws a duration nobody configured as the placeholder", async () => {
    theSettings(UNSET);
    mountPane();

    const field = (await waitFor(() =>
      screen.getByLabelText(/before trimming/),
    )) as HTMLInputElement;

    expect(field.value).toBe("");
    expect(field.placeholder).toBe("3");
  });

  it("draws a duration somebody configured as the value", async () => {
    theSettings(TOLD);
    mountPane();

    const field = (await waitFor(() =>
      screen.getByLabelText(/before trimming/),
    )) as HTMLInputElement;

    expect(field.value).toBe("5");
  });

  /// The duration of a step that never happens is nothing to ask for: the
  /// switch is what says whether there is a clock at all.
  it("asks for no duration while a step is switched off", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByLabelText(/before trimming/));
    expect(screen.queryByLabelText(/before deleting/)).toBeNull();
  });
});

describe("changing the cleanup", () => {
  /// A switch is its own save. What goes with it is everything else in the
  /// file as it stands: one request writes both files, so a flip here must not
  /// be able to take the credentials or the other row with it.
  it("saves the moment the trim is switched off, and leaves the rest alone", async () => {
    const off = step(TOLD, "trim", { enabled: false });
    const fetching = theSettings(TOLD, json(answering(off)));
    mountPane();

    await waitFor(() => expect(theTrim().checked).toBe(true));
    fireEvent.click(theTrim());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        cleanup: {
          trim: { enabled: false, days: "5" },
          // The row this press was not about, exactly as the read left it.
          delete: { enabled: true, days: "90" },
        },
      }),
    );

    // And the switch follows the answer rather than the press.
    await waitFor(() => expect(theTrim().checked).toBe(false));
  });

  it("saves the moment the delete is switched on", async () => {
    const on = step(UNSET, "delete", { enabled: true });
    const fetching = theSettings(UNSET, json(answering(on)));
    mountPane();

    await waitFor(() => expect(theDelete().checked).toBe(false));
    fireEvent.click(theDelete());

    await waitFor(() =>
      expect(
        (sent(fetching) as { cleanup: { delete: unknown } }).cleanup.delete,
      ).toEqual({ enabled: true, days: "" }),
    );

    await waitFor(() => expect(theDelete().checked).toBe(true));
  });

  /// A duration is typed, so it waits for a press: nothing is committed while
  /// somebody is halfway through writing `30`.
  it("sends a duration only when it is saved", async () => {
    const sooner = step(TOLD, "trim", { days: 2, days_configured: true });
    const fetching = theSettings(TOLD, json(answering(sooner)));
    mountPane();

    const field = await waitFor(() => screen.getByLabelText(/before trimming/));
    fireEvent.input(field, { target: { value: "2" } });

    expect(
      fetching.mock.calls.some(([, init]) => init?.method === "POST"),
      "typing is not saving",
    ).toBe(false);

    fireEvent.click(screen.getAllByRole("button", { name: "Save" })[0]!);

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        cleanup: {
          trim: { enabled: true, days: "2" },
          delete: { enabled: true, days: "90" },
        },
      }),
    );
  });

  /// Clearing a field asks for the default back, which is what an empty
  /// duration means to the server — and the placeholder is what says so.
  it("sends an empty duration for a field the human cleared", async () => {
    const fetching = theSettings(
      TOLD,
      json(answering(step(TOLD, "trim", { days: 3, days_configured: false }))),
    );
    mountPane();

    const field = await waitFor(() => screen.getByLabelText(/before trimming/));
    fireEvent.input(field, { target: { value: "" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Save" })[0]!);

    await waitFor(() =>
      expect(
        (sent(fetching) as { cleanup: { trim: { days: string } } }).cleanup.trim
          .days,
      ).toBe(""),
    );

    // And the field is the placeholder again, which is the answer being drawn
    // rather than what was typed.
    await waitFor(() =>
      expect((screen.getByLabelText(/before trimming/) as HTMLInputElement).placeholder).toBe("3"),
    );
  });

  /// A delete sooner than the trim is saved as it was typed and said back in
  /// words: the pane reports the ordering rather than refusing it.
  it("saves a delete sooner than the trim, and says what it means", async () => {
    const sooner = step(TOLD, "delete", { days: 2, days_configured: true });
    const fetching = theSettings(TOLD, json(answering(sooner)));
    mountPane();

    const field = await waitFor(() => screen.getByLabelText(/before deleting/));
    fireEvent.input(field, { target: { value: "2" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Save" })[1]!);

    await waitFor(() =>
      expect(
        (sent(fetching) as { cleanup: { delete: { days: string } } }).cleanup
          .delete.days,
      ).toBe("2"),
    );

    await waitFor(() => screen.getByText(/The delete comes first/));
  });

  it("says so when the save fails", async () => {
    const fetching = theSettings(TOLD, () =>
      Promise.resolve(
        new Response("nope", { status: 503, statusText: "Unavailable" }),
      ),
    );
    mountPane();

    await waitFor(() => expect(theTrim().checked).toBe(true));
    fireEvent.click(theTrim());

    await waitFor(() => screen.getByText(/could not be saved/));
    expect(sent(fetching)).toBeTruthy();
  });
});
