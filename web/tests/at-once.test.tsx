//! How much Verkstead runs at once, on the settings page: what the card says of
//! it, what the field in its pane puts on the wire, and the one number this page
//! refuses.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying how much runs at once, and the field that changes it in the
//! details pane it opens.
//!
//! One field and one press, the way a duration on the Cleanup pane is saved: the
//! number is typed, so nothing is committed while somebody is halfway through
//! writing it. That save sends the whole of the settings edit — the author as it
//! stands, the token untouched and every other section where the read left it —
//! because the server writes both files in one request, and that is what these
//! check is not lost.
//!
//! **And this is the one field with a floor under it.** Everywhere else what
//! cannot be read is the default asked for back, and that holds here for the
//! cleared box; what does not is a number below one, which asks for a roadmap that
//! never starts anything. So the press is refused and nothing is sent — checked
//! here, because a page that quietly saved three instead would be saving something
//! nobody typed.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with: `settings.json` is the
//! Verkstead that has been told everything — one stage of a roadmap at a time —
//! and `settings-unset.json` is the one nobody has been to, which is three.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import { AtOnceCard, AtOncePane } from "../src/settings/AtOnce";
import styles from "../src/settings/AtOnce.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The rest of `config.yaml` as every save from this pane sends it: what the read
/// said, left exactly where it was.
const REST = {
  git_author: TOLD.git_author,
  github_token: "Keep",
  // The rules ride along as an action rather than a value: nothing this form
  // does says anything about them — see [`IgnoredCommentsEdit`].
  ignored_comments: "Keep",
  // And the declared servers likewise, and for the same reason — see
  // [`McpServersEdit`].
  mcp_servers: "Keep",
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
  // And the text every session is given, likewise: what is sent is what the
  // file holds afterwards, so a save that left it out would clear it.
  instructions: TOLD.instructions,
};

/// The same settings with the limit somewhere else — what a save answers with,
/// and what a fixture of a number nobody typed is drawn from.
function limited(
  standing: SettingsView,
  roadmap_stages: number,
  configured = true,
): SettingsView {
  return {
    ...standing,
    at_once: {
      roadmap_stages,
      roadmap_stages_configured: configured,
    },
  };
}

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

/// The card in the middle pane, and what pressing it asked for.
function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <AtOnceCard open={open} press={press} />),
    press,
  };
}

/// The field in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <AtOncePane back={back} />), back };
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// What a save answers with, which is the settings as they now stand.
function answering(standing: SettingsView): SettingsSaved {
  return {
    settings: standing,
    verified: null,
    refused: [],
    refused_servers: [],
    tried: [],
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

/// Whether anything was saved at all, for the tests about the press that is
/// refused.
function saved(fetching: ReturnType<typeof serving>): boolean {
  return fetching.mock.calls.some(([, init]) => init?.method === "POST");
}

/// The card itself, once it is drawn — waited for, because it stands on a read.
async function theCard(container: ParentNode): Promise<HTMLElement> {
  return await waitFor(() => {
    const face = container.querySelector<HTMLElement>(`.${styles.atOnceCard}`);
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

/// The one field, by the words beside it.
async function theField(): Promise<HTMLInputElement> {
  return (await waitFor(() =>
    screen.getByLabelText(/Stages of one roadmap/),
  )) as HTMLInputElement;
}

const theSave = () => screen.getByRole("button", { name: "Save" });

describe("the card", () => {
  /// What somebody scanning the page is after: how much of their machine this is
  /// helping itself to. Drawn whether or not anybody chose it, because the
  /// default is as much an answer to *how many* as a choice would be.
  it("says how much of one roadmap runs at once", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/of one roadmap at a time/));
    expect(screen.getByText("3 stages")).toBeTruthy();
  });

  /// And the number somebody chose, in the words that agree with it: *one stage*
  /// rather than *1 stages*, this being the number a human is most likely to have
  /// set.
  it("says the number somebody chose, in words that agree with it", async () => {
    theSettings(TOLD);
    mountCard();

    await waitFor(() => screen.getByText(/of one roadmap at a time/));
    expect(screen.getByText("One stage")).toBeTruthy();
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

describe("the limit as the pane draws it", () => {
  /// A number nobody configured is the default drawn as a placeholder rather than
  /// as text somebody typed — the field says what will happen without claiming
  /// anybody chose it.
  it("draws a number nobody configured as the placeholder", async () => {
    theSettings(UNSET);
    mountPane();

    const field = await theField();

    expect(field.value).toBe("");
    expect(field.placeholder).toBe("3");
  });

  it("draws a number somebody configured as the value", async () => {
    theSettings(TOLD);
    mountPane();

    expect((await theField()).value).toBe("1");
  });

  /// The one thing about this number a human cannot work out from it: that a
  /// stage they have been asked a question about is holding a place.
  it("says what holds a place", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => screen.getByText(/waiting on an answer from you/));
  });

  it("says so when the server could not be read at all", async () => {
    serving(() =>
      Promise.resolve(
        new Response("nope", { status: 500, statusText: "Server Error" }),
      ),
    );
    mountPane();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("changing the limit", () => {
  /// The press saves the number, and everything else in the file rides along as
  /// the read left it: one request writes both files, so a save here must not be
  /// able to take the credentials or another section with it.
  it("sends what was typed, with the rest of the file as it stands", async () => {
    const fetching = theSettings(TOLD, json(answering(limited(TOLD, 2))));
    mountPane();

    fireEvent.input(await theField(), { target: { value: "2" } });
    fireEvent.click(theSave());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        at_once: { roadmap_stages: "2" },
      }),
    );

    // And the field follows the answer rather than the press.
    await waitFor(async () => expect((await theField()).value).toBe("2"));
  });

  /// Clearing the box is asking for the default back rather than for a roadmap
  /// with no places, which is what an empty string means to the server.
  it("sends an empty field where the box was cleared", async () => {
    const fetching = theSettings(
      TOLD,
      json(answering(limited(TOLD, 3, false))),
    );
    mountPane();

    fireEvent.input(await theField(), { target: { value: "" } });
    fireEvent.click(theSave());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        at_once: { roadmap_stages: "" },
      }),
    );

    // And the field is drawing the default again, as a placeholder.
    await waitFor(async () => expect((await theField()).placeholder).toBe("3"));
  });

  /// And the one refusal on this page that is the page's own. A limit of nought
  /// is a roadmap that never starts anything: nothing is sent, and the line says
  /// what a number here can be.
  it("refuses a limit that would start nothing", async () => {
    const fetching = theSettings(TOLD);
    mountPane();

    fireEvent.input(await theField(), { target: { value: "0" } });

    await waitFor(() => screen.getByText(/A whole number of stages/));

    fireEvent.click(theSave());

    expect(saved(fetching), "a refused press saves nothing").toBe(false);
  });

  /// And anything else that is not a whole number of stages, for the reason a
  /// nought is: the page has a floor and a grammar, and neither of them is the
  /// server's to guess at afterwards.
  it("refuses a limit that is not a whole number of stages", async () => {
    const fetching = theSettings(TOLD);
    mountPane();

    for (const typed of ["lots", "2.5", "-1"]) {
      fireEvent.input(await theField(), { target: { value: typed } });

      await waitFor(() => screen.getByText(/A whole number of stages/));

      fireEvent.click(theSave());

      expect(saved(fetching), `nothing a roadmap could run in ${typed}`).toBe(
        false,
      );
    }
  });

  it("says so when the save fails", async () => {
    const fetching = theSettings(TOLD, () =>
      Promise.resolve(
        new Response("nope", { status: 503, statusText: "Unavailable" }),
      ),
    );
    mountPane();

    fireEvent.input(await theField(), { target: { value: "2" } });
    fireEvent.click(theSave());

    await waitFor(() => screen.getByText(/could not be saved/));
    expect(saved(fetching)).toBe(true);
  });
});
