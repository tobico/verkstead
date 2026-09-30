//! How much Verkstead runs at once, on the settings page: what the card says of
//! the two limits, what the fields in its pane put on the wire, how many of the
//! server's places the pane says are taken, and the numbers this page refuses.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying how much runs at once, and the fields that change it in the
//! details pane it opens.
//!
//! Two fields and a press each, the way a duration on the Cleanup pane is saved:
//! the numbers are typed, so nothing is committed while somebody is halfway
//! through writing one — and either press sends both, the page having one request
//! to write the whole file with. That save sends the whole of the settings edit —
//! the author as it stands, the token untouched and every other section where the
//! read left it — because the server writes both files in one request, and that is
//! what these check is not lost.
//!
//! **And these are the one pair of fields with a floor under them.** Everywhere
//! else what cannot be read is the default asked for back, and that holds here for
//! the cleared box; what does not is a number below one, which asks for a roadmap
//! — or a server — that never starts anything. So the press is refused and nothing
//! is sent — checked here, because a page that quietly saved three instead would be
//! saving something nobody typed.
//!
//! **And how many places are taken is a reading rather than a setting**: it is
//! drawn beside the limit it is measured against and goes back nowhere, because a
//! server holding every place starts nothing more until one comes free and that
//! has to read as held rather than as stalled.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with: `settings.json` is the
//! Verkstead that has been told everything — one stage of a roadmap at a time and
//! two Conversations across the server — and `settings-unset.json` is the one
//! nobody has been to, which is three and four.

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
  // And the languages as the read left them — see [`heldLanguages`].
  languages: [
    { name: "rust", enabled: true, size: "50G" },
    { name: "go", enabled: true, size: "" },
    { name: "node", enabled: true, size: "" },
    { name: "python", enabled: true, size: "" },
    { name: "dotnet", enabled: true, size: "" },
    { name: "cpp", enabled: true, size: "" },
    { name: "jvm", enabled: true, size: "" },
    { name: "gleam", enabled: true, size: "8G" },
  ],
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

/// The same settings with the limits somewhere else — what a save answers with,
/// and what a fixture of a number nobody typed is drawn from.
///
/// `taken` is how many of the server's places are held, which rides back with
/// every read and is nothing a save sets.
function limited(
  standing: SettingsView,
  limits: {
    roadmap_stages?: number;
    roadmap_stages_configured?: boolean;
    conversations?: number;
    conversations_configured?: boolean;
    places_taken?: number;
  },
): SettingsView {
  return { ...standing, at_once: { ...standing.at_once, ...limits } };
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

/// The roadmap's own field, by the words beside it.
async function theField(): Promise<HTMLInputElement> {
  return (await waitFor(() =>
    screen.getByLabelText(/Stages of one roadmap/),
  )) as HTMLInputElement;
}

/// And the server's, beside it.
async function theServerField(): Promise<HTMLInputElement> {
  return (await waitFor(() =>
    screen.getByLabelText(/Conversations across the whole server/),
  )) as HTMLInputElement;
}

/// A press per field, in the order the fields stand in — the same shape the
/// Cleanup pane's two durations are pressed by.
const theSaves = () => screen.getAllByRole("button", { name: "Save" });
const theSave = () => theSaves()[0]!;
const theServerSave = () => theSaves()[1]!;

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

  /// And the other limit beside it, because there are two and the card is what
  /// somebody scanning the page reads both off.
  it("says how many conversations the whole server runs at once", async () => {
    theSettings(UNSET);
    mountCard();

    await waitFor(() => screen.getByText(/across the whole server/));
    expect(screen.getByText("4 conversations")).toBeTruthy();
  });

  /// And the numbers somebody chose, in the words that agree with them: *one
  /// stage* rather than *1 stages*, this being the number a human is most likely
  /// to have set.
  it("says the numbers somebody chose, in words that agree with them", async () => {
    theSettings(TOLD);
    mountCard();

    await waitFor(() => screen.getByText(/of one roadmap at a time/));
    expect(screen.getByText("One stage")).toBeTruthy();
    expect(screen.getByText("2 conversations")).toBeTruthy();
  });

  /// And the singular on the server's line too, which is the number somebody
  /// reaches for on a machine that is doing too much.
  it("says one conversation in the singular", async () => {
    theSettings(limited(TOLD, { conversations: 1 }));
    mountCard();

    await waitFor(() => screen.getByText(/across the whole server/));
    expect(screen.getByText("One conversation")).toBeTruthy();
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

    const server = await theServerField();

    expect(server.value).toBe("");
    expect(server.placeholder).toBe("4");
  });

  it("draws a number somebody configured as the value", async () => {
    theSettings(TOLD);
    mountPane();

    expect((await theField()).value).toBe("1");
    expect((await theServerField()).value).toBe("2");
  });

  /// The one thing about these numbers a human cannot work out from them: that a
  /// stage they have been asked a question about is holding a place.
  it("says what holds a place", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => screen.getByText(/waiting on an answer from you/));
  });

  /// And what the server's limit is measured against, beside it: a server
  /// holding every place starts nothing more until one comes free, and a number
  /// nobody could see would leave that looking like a stall.
  it("says how many of the server's places are taken", async () => {
    theSettings(limited(TOLD, { conversations: 4, places_taken: 4 }));
    mountPane();

    await waitFor(() => screen.getByText(/taken when this was read/));
    expect(screen.getByText("4 of 4")).toBeTruthy();
  });

  /// And it is drawn as it came back rather than clamped to the limit: a press
  /// goes ahead over the limit and is counted from then on, so more places can be
  /// held than there are.
  it("says a count standing above the limit as it came back", async () => {
    theSettings(limited(TOLD, { conversations: 2, places_taken: 3 }));
    mountPane();

    await waitFor(() => screen.getByText("3 of 2"));
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

describe("changing the limits", () => {
  /// The press saves the number, and everything else in the file rides along as
  /// the read left it: one request writes both files, so a save here must not be
  /// able to take the credentials or another section with it.
  ///
  /// The field this press is not about rides along as it stands too, which is
  /// what keeps one press from clearing the limit beside it.
  it("sends what was typed, with the rest of the file as it stands", async () => {
    const fetching = theSettings(
      TOLD,
      json(answering(limited(TOLD, { roadmap_stages: 2 }))),
    );
    mountPane();

    fireEvent.input(await theField(), { target: { value: "2" } });
    fireEvent.click(theSave());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        at_once: { roadmap_stages: "2", conversations: "2" },
      }),
    );

    // And the field follows the answer rather than the press.
    await waitFor(async () => expect((await theField()).value).toBe("2"));
  });

  /// And the server's own field, by its own press — which sends both for that
  /// same reason.
  it("sends the server's limit from its own press", async () => {
    const fetching = theSettings(
      TOLD,
      json(answering(limited(TOLD, { conversations: 6 }))),
    );
    mountPane();

    fireEvent.input(await theServerField(), { target: { value: "6" } });
    fireEvent.click(theServerSave());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        at_once: { roadmap_stages: "1", conversations: "6" },
      }),
    );

    await waitFor(async () => expect((await theServerField()).value).toBe("6"));
  });

  /// Clearing a box is asking for the default back rather than for a roadmap — or
  /// a server — with no places, which is what an empty string means to the server.
  it("sends an empty field where the box was cleared", async () => {
    const fetching = theSettings(
      TOLD,
      json(
        answering(
          limited(TOLD, {
            roadmap_stages: 3,
            roadmap_stages_configured: false,
            conversations: 4,
            conversations_configured: false,
          }),
        ),
      ),
    );
    mountPane();

    fireEvent.input(await theField(), { target: { value: "" } });
    fireEvent.input(await theServerField(), { target: { value: "" } });
    fireEvent.click(theSave());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        at_once: { roadmap_stages: "", conversations: "" },
      }),
    );

    // And the fields are drawing the defaults again, as placeholders.
    await waitFor(async () => expect((await theField()).placeholder).toBe("3"));
    await waitFor(async () =>
      expect((await theServerField()).placeholder).toBe("4"),
    );
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

  /// And a server with no places, for the reason a roadmap with none is refused:
  /// what it asks for is a Verkstead that starts nothing at all, ever. In the
  /// server's own words, because the line says back what the number counts.
  it("refuses a server with no places at all", async () => {
    const fetching = theSettings(TOLD);
    mountPane();

    fireEvent.input(await theServerField(), { target: { value: "0" } });

    await waitFor(() => screen.getByText(/A whole number of conversations/));

    fireEvent.click(theServerSave());

    expect(saved(fetching), "a refused press saves nothing").toBe(false);
  });

  /// And **either** press is refused while either field is bad, because one press
  /// sends both: a save let through here would write the number the page had
  /// just turned down.
  it("refuses the other field's press while one is bad", async () => {
    const fetching = theSettings(TOLD);
    mountPane();

    fireEvent.input(await theServerField(), { target: { value: "0" } });

    await waitFor(() => screen.getByText(/A whole number of conversations/));

    fireEvent.click(theSave());

    expect(saved(fetching), "the press that sends both saves neither").toBe(
      false,
    );
  });

  /// And anything else that is not a whole number of places, for the reason a
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
