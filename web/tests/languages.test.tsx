//! Language support on the settings page: which languages the card names, what
//! the checkboxes in its pane put on the wire, and the size that hangs off the
//! one whose store an sccache bounds.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane naming the languages that have build support on, and the controls that
//! change them in the details pane it opens. Each is mounted on its own, and the
//! pair together only where the round trip is what is being asked about.
//!
//! **The page is drawn from what the server loaded**, so the fixture carries the
//! descriptors Verkstead ships and one an installer wrote into `config.yaml`. A box under a label nothing in the viewer knows is the whole
//! of what this section is for, and a suite that only ever saw Rust would prove
//! only that the page can draw the language it was written around.
//!
//! Two saves and one endpoint. A checkbox is its own press, because a box that
//! needed a second one is not a box; the size is typed, so it waits for a Save.
//! Both send the whole of the settings edit — the author as it stands and the
//! token untouched — because the server writes both files in one request, and
//! that is what these check is not lost.
//!
//! And the size is nested under its checkbox, which is the page's pattern for
//! configuration that only means something while something else is on: it is
//! greyed and refuses input while the box is unticked, and again while there is
//! no sccache to read it. Both of those are here, because the second is the one
//! nobody would think to try.
//!
//! The read is a fixture the server's own tests wrote, so what the page is
//! drawn from is the shape the endpoint really answers with.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import check from "../src/Check.module.css";
import { LanguagesCard, LanguagesPane } from "../src/settings/Languages";
import styles from "../src/settings/Languages.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The built-in descriptor, and the one an installer wrote — by the names
/// `config.yaml` keys them under, which is what a save names back.
const RUST = "rust";
const GO = "go";
const NODE = "node";
const PYTHON = "python";
const DOTNET = "dotnet";
const CPP = "cpp";
const GLEAM = "gleam";

/// The binds the fixture holds, as a save puts them back on the wire: the
/// settings' own entries, each as the path it names. Every section's save
/// carries them, because one request writes the whole of `config.yaml` — a list
/// left out would be a list emptied.
const PATHS = {
  sandbox_binds: ["/var/cache/verkstead-node", "/var/cache/verkstead-cargo"],
};

/// And the Cleanup as a save puts it back: the switches where the read left
/// them, and each duration as the string a form holds. Carried by every
/// section for the reason the paths are — see [`heldCleanup`].
const CLEANUP = {
  trim: { enabled: true, days: "5" },
  delete: { enabled: true, days: "90" },
};

/// The same settings with an sccache the server did find, which no fixture
/// carries: the routers those are written from run no sessions, so they have
/// none to hand out.
function compiling(standing: SettingsView): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) => ({
      ...language,
      compiling: language.compiling ? "Cached" : null,
    })),
  };
}

/// And the same settings with one language switched off.
function off(standing: SettingsView, name = RUST): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) =>
      language.name === name ? { ...language, enabled: false } : language,
    ),
  };
}

/// And the same settings with one language's entry in `config.yaml` one the
/// server could not read.
///
/// `running_on` is the server's answer rather than this helper's arithmetic:
/// what says a language has something to fall back to is that Verkstead ships a
/// descriptor of that name, which is a fact about the binary.
function unread(
  standing: SettingsView,
  name: string,
  running_on: "BuiltIn" | "Nothing",
  why = "sets RUSTUP_HOME, which is a variable the Sandbox sets itself",
): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) =>
      language.name === name
        ? {
            ...language,
            enabled: running_on === "BuiltIn",
            unread: { why, running_on },
          }
        : language,
    ),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Whatever half of the section a test is about, over one query client: both
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
    ...mounting(() => <LanguagesCard open={open} press={press} />),
    press,
  };
}

/// The controls in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <LanguagesPane back={back} />), back };
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

/// The languages a save carried, which is what every press on this pane sends.
function languagesSent(fetching: ReturnType<typeof serving>) {
  return (sent(fetching) as { languages: Array<Record<string, unknown>> })
    .languages;
}

/// The card itself, once it is drawn — waited for, because it stands on a read.
async function theCard(container: ParentNode): Promise<HTMLElement> {
  return await waitFor(() => {
    const face = container.querySelector<HTMLElement>(
      `.${styles.languagesCard}`,
    );
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

/// One of the boxes on the pane, by the label the server gave its language.
function theCheck(label = "Rust"): HTMLInputElement {
  return screen.getByRole("checkbox", { name: label }) as HTMLInputElement;
}

/// And the group hanging off one, which holds the size. There is one on this
/// fixture, which is Rust's: nothing bounds the installer's own store.
function theGroup(container: ParentNode): HTMLFieldSetElement {
  const group = container.querySelector<HTMLFieldSetElement>(
    `.${check.nested}`,
  );
  expect(group, "expected the nested group to be drawn").not.toBeNull();
  return group!;
}

describe("the card", () => {
  /// The summary is the list of languages that have build support on, in the
  /// labels the descriptors gave them.
  it("names every language that is on", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountCard();

    await waitFor(() =>
      expect(container.querySelector(`.${styles.standing}`)?.textContent).toBe(
        "Rust, Go, Node, Python, .NET, C/C++, Gleam",
      ),
    );
  });

  /// And the one an installer wrote goes when it is switched off, the same way
  /// the built-in does: there is nothing about either that the card knows.
  it("drops a language that is switched off", async () => {
    theSettings(compiling(off(TOLD, GLEAM)));
    const { container } = mountCard();

    await waitFor(() =>
      expect(container.querySelector(`.${styles.standing}`)?.textContent).toBe(
        "Rust, Go, Node, Python, .NET, C/C++",
      ),
    );
  });

  /// And nothing at all under the heading where none are: a card saying a
  /// machine builds nothing would be a line nobody needs.
  it("says nothing under the heading while they are all off", async () => {
    theSettings(
      off(
        off(off(off(off(off(off(TOLD), GO), NODE), PYTHON), DOTNET), CPP),
        GLEAM,
      ),
    );
    const { container } = mountCard();

    const face = await theCard(container);
    expect(face.textContent).toBe("Language support");
    expect(container.querySelector(`.${styles.standing}`)).toBeNull();
  });

  /// What the human cannot fix from the browser, said where they would
  /// otherwise wonder why nothing got faster — and on the card rather than
  /// only in the pane, because whoever needs to read it is whoever is not
  /// editing.
  it("warns when there is no sccache for the server to compile through", async () => {
    theSettings(UNSET);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/No sccache is installed/));
    expect(container.querySelector(`.${styles.warning}`)).not.toBeNull();
  });

  /// And nothing about it while the languages that want one are switched off,
  /// because the half of that warning that says the downloads are still shared
  /// is only true while there is a cache to share them. Both of them: C/C++
  /// compiles through the same Compile Server, so it is uncached too.
  it("says nothing about sccache while the languages that want one are off", async () => {
    theSettings(off(off(UNSET), CPP));
    const { container } = mountCard();

    await theCard(container);
    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  it("says nothing about sccache where the server found one", async () => {
    theSettings(compiling(UNSET));
    mountCard();

    await waitFor(() =>
      screen.getByText("Rust, Go, Node, Python, .NET, C/C++"),
    );
    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  /// And a language whose entry the server could not read, said here too: the
  /// card is what somebody scanning the page sees, and a file that wants
  /// editing is exactly the sort of thing they would want telling about.
  it("names a language whose entry could not be read", async () => {
    theSettings(unread(compiling(TOLD), GLEAM, "Nothing"));
    const { container } = mountCard();

    await waitFor(() =>
      expect(container.querySelector(`.${styles.warning}`)?.textContent).toBe(
        "Verkstead could not read what config.yaml says about Gleam.",
      ),
    );
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

describe("the languages as the pane draws them", () => {
  /// A box per descriptor the server loaded, under the label it gave — the one
  /// an installer wrote included, which is the whole of what *the file only*
  /// comes to: they write `config.yaml` and the box is here next time.
  it("draws a checkbox per language the server lists", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => expect(theCheck("Rust").checked).toBe(true));
    expect(theCheck("Go").checked).toBe(true);
    expect(theCheck("Node").checked).toBe(true);
    expect(theCheck("Python").checked).toBe(true);
    expect(theCheck(".NET").checked).toBe(true);
    expect(theCheck("C/C++").checked).toBe(true);
    expect(theCheck("Gleam").checked).toBe(true);
    expect(screen.getAllByRole("checkbox")).toHaveLength(7);
  });

  /// The box says where its language stands rather than whether anybody has
  /// touched it, which is what an unconfigured language being on means.
  it("reads as on where nothing has been configured", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
  });

  /// The size is sccache's own word, so a server without one has nothing to
  /// read it — and the field is greyed rather than taken away, because the
  /// setting has not gone anywhere.
  it("greys the size where there is no sccache to read it", async () => {
    theSettings(UNSET);
    const { container } = mountPane();

    await waitFor(() => screen.getByText(/No sccache is installed/));

    expect(theCheck().checked).toBe(true);
    expect(theGroup(container).disabled).toBe(true);
    expect(screen.getByLabelText(/How large/).matches(":disabled")).toBe(true);
    expect(
      screen.getByRole("button", { name: "Save" }).matches(":disabled"),
    ).toBe(true);
  });

  /// And greyed again while the box is unticked, which is the other half of the
  /// pattern: the configuration hanging off a checkbox means nothing while the
  /// checkbox is off. Both boxes naming the Compile Server, because with one of
  /// them on the size is that one's — see the two tests below.
  it("greys the size while the box is unticked", async () => {
    theSettings(off(off(compiling(TOLD)), CPP));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(false));

    expect(theGroup(container).disabled).toBe(true);
    expect(screen.getByLabelText(/How large/).matches(":disabled")).toBe(true);
  });

  /// A language whose entry would not load says so, with the reason the server
  /// gave and what it is running on meanwhile — and its controls are off, the
  /// two keys they write being keys of the entry nothing could read.
  it("says why a language's entry was not used and what it is running on", async () => {
    theSettings(unread(compiling(TOLD), RUST, "BuiltIn"));
    const { container } = mountPane();

    await waitFor(() =>
      expect(
        container.querySelector(`.${styles.warning}`)?.textContent,
      ).toContain(
        "Its entry in config.yaml sets RUSTUP_HOME, which is a variable the " +
          "Sandbox sets itself. Verkstead is using the descriptor it ships",
      ),
    );

    expect(theCheck().matches(":disabled")).toBe(true);
    expect(theGroup(container).disabled).toBe(true);
  });

  /// And where there is no built-in of that name, the other half of the
  /// sentence: the language is off until the entry is fixed, which is a
  /// different thing to know from a cache still working.
  it("says a language with nothing behind it is off until the entry is fixed", async () => {
    theSettings(unread(compiling(TOLD), GLEAM, "Nothing"));
    const { container } = mountPane();

    await waitFor(() =>
      expect(
        container.querySelector(`.${styles.warning}`)?.textContent,
      ).toContain(
        "Verkstead ships no descriptor of that name, so this language is off",
      ),
    );

    expect(theCheck("Gleam").checked).toBe(false);
    expect(theCheck("Gleam").matches(":disabled")).toBe(true);
    expect(
      theCheck("Rust").matches(":disabled"),
      "while the language beside it is left alone",
    ).toBe(false);
  });

  /// A greyed group refuses input rather than only looking as though it would:
  /// the browser will not take the press at all, so nothing is saved.
  it("refuses a save from the greyed size", async () => {
    const fetching = theSettings(off(compiling(TOLD)));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(false));

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(
      fetching.mock.calls.some(([, init]) => init?.method === "POST"),
      "a disabled press is no press",
    ).toBe(false);
  });

  /// And it takes input where both halves say it means something.
  it("lets the size be typed where the box is on and sccache is there", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));

    expect(theGroup(container).disabled).toBe(false);
    expect(screen.getByLabelText(/How large/).matches(":disabled")).toBe(false);
  });

  /// And a language whose store nothing bounds has no size at all — there is
  /// one field on this pane, which is the one the sccache belongs to.
  it("draws no size under a language whose store nothing bounds", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck("Gleam").checked).toBe(true));

    expect(
      container.querySelectorAll(`.${check.nested}`),
      "one group, which is the one with an sccache behind it",
    ).toHaveLength(1);
    expect(screen.getAllByLabelText(/How large/)).toHaveLength(1);
  });

  /// One Compile Server between the two languages naming it, so one size: the
  /// field is on the language that sizes it, and the other says whose it is.
  it("draws the one size on Rust and says C/C++ shares its Compile Server", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await waitFor(() => expect(theCheck("C/C++").checked).toBe(true));

    expect(screen.getAllByLabelText(/How large/)).toHaveLength(1);
    expect(screen.getByLabelText(/How large/).id).toBe(`language-size-${RUST}`);
    expect(
      screen.getByText(/Shares the Compile Server with Rust/),
    ).toBeTruthy();
  });

  /// And with Rust off, C/C++ is what the server is sized by, so the field is
  /// its — the same answer the server gives about which size it starts at.
  it("moves the size to C/C++ when Rust is switched off", async () => {
    theSettings(off(compiling(TOLD)));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(false));

    expect(screen.getAllByLabelText(/How large/)).toHaveLength(1);
    expect(screen.getByLabelText(/How large/).id).toBe(`language-size-${CPP}`);
    expect(
      screen.getByText(/Shares the Compile Server with C\/C\+\+/),
    ).toBeTruthy();
  });

  it("says nothing about sccache while the languages are switched off", async () => {
    theSettings(off(off(UNSET), CPP));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(false));

    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  /// An unconfigured size is the default drawn as a placeholder rather than as
  /// text somebody typed — the field says what will happen without claiming
  /// anybody chose it.
  it("draws a size nobody configured as the placeholder", async () => {
    theSettings(compiling(UNSET));
    mountPane();

    const field = (await waitFor(() =>
      screen.getByLabelText(/How large/),
    )) as HTMLInputElement;

    expect(field.value).toBe("");
    expect(field.placeholder).toBe("30G");
  });

  it("draws a size somebody configured as the value", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    const field = (await waitFor(() =>
      screen.getByLabelText(/How large/),
    )) as HTMLInputElement;

    expect(field.value).toBe("50G");
  });
});

describe("changing the languages", () => {
  /// A checkbox is its own save. What goes with it is the author as it stands
  /// and `Keep` for the token: one request writes both files, so a tick here
  /// must not be able to take the credentials with it.
  it("saves the moment the box is ticked, and leaves the credentials alone", async () => {
    const fetching = theSettings(TOLD, json(answering(off(TOLD))));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
    fireEvent.click(theCheck());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Keep",
        // The rules ride along as an action rather than a value: nothing this
        // form does says anything about them — see [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
        mcp_servers: "Keep",
        // Every language, because one request writes the whole file — the one
        // that was pressed as it is now to stand, and the rest as they were,
        // sizes included, whether this pane drew a field for one or not.
        languages: [
          { name: RUST, enabled: false, size: "50G" },
          { name: GO, enabled: true, size: "" },
          { name: NODE, enabled: true, size: "" },
          { name: PYTHON, enabled: true, size: "" },
          { name: DOTNET, enabled: true, size: "" },
          { name: CPP, enabled: true, size: "" },
          { name: GLEAM, enabled: true, size: "8G" },
        ],
        // Untouched by this form, and sent back as it stands: one request
        // writes the whole of `config.yaml`.
        cleanup: CLEANUP,
        conflict_resolution: TOLD.conflict_resolution,
        share_on_done: TOLD.share_on_done,
        ...PATHS,
        // And the text every session is given, likewise: what is sent is what the
        // file holds afterwards, so a save that left it out would clear it.
        instructions: TOLD.instructions,
      }),
    );

    // And the box follows the answer rather than the press.
    await waitFor(() => expect(theCheck().checked).toBe(false));
  });

  /// And a box the installer's own language draws saves the same way: the
  /// entry it names is the one that moves, and the built-in stands.
  it("saves an installer's own language under its own name", async () => {
    const fetching = theSettings(TOLD, json(answering(off(TOLD, GLEAM))));
    mountPane();

    await waitFor(() => expect(theCheck("Gleam").checked).toBe(true));
    fireEvent.click(theCheck("Gleam"));

    await waitFor(() =>
      expect(languagesSent(fetching)).toEqual([
        { name: RUST, enabled: true, size: "50G" },
        { name: GO, enabled: true, size: "" },
        { name: NODE, enabled: true, size: "" },
        { name: PYTHON, enabled: true, size: "" },
        { name: DOTNET, enabled: true, size: "" },
        { name: CPP, enabled: true, size: "" },
        { name: GLEAM, enabled: false, size: "8G" },
      ]),
    );
  });

  /// A tick has to say something about the sizes, because one request writes
  /// the whole file — and what it says is what the server holds rather than
  /// what is in the box. Otherwise unticking the box mid-edit writes the `5` of
  /// a `50` as the store size, which is a number nobody pressed Save on. The
  /// Cleanup pane's two durations hold the same rule.
  it("sends the server's size when the box is ticked, not what is typed", async () => {
    // Answered with C/C++ off too, so the field stays on Rust's box: with
    // C/C++ on it would be C/C++'s size the pane drew instead.
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(off(off(TOLD), CPP))),
    );
    mountPane();

    const field = await waitFor(() => screen.getByLabelText(/How large/));
    fireEvent.input(field, { target: { value: "5" } });

    fireEvent.click(theCheck());

    await waitFor(() =>
      expect(languagesSent(fetching)[0]?.size).toBe("50G"),
    );

    // And what was typed is still there to finish typing: the tick did not
    // commit it, so the field did not let go of it either.
    await waitFor(() => expect(theCheck().checked).toBe(false));
    expect((screen.getByLabelText(/How large/) as HTMLInputElement).value).toBe(
      "5",
    );
  });

  /// The size is typed, so it waits for a press: nothing is committed while
  /// somebody is halfway through writing `30`.
  it("sends a size only when it is saved", async () => {
    const bigger = compiling(TOLD);
    const fetching = theSettings(compiling(TOLD), json(answering(bigger)));
    mountPane();

    const field = await waitFor(() => screen.getByLabelText(/How large/));
    fireEvent.input(field, { target: { value: "80G" } });

    expect(
      fetching.mock.calls.some(([, init]) => init?.method === "POST"),
      "typing is not saving",
    ).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Keep",
        // The rules ride along as an action rather than a value: nothing this
        // form does says anything about them — see [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
        mcp_servers: "Keep",
        languages: [
          { name: RUST, enabled: true, size: "80G" },
          { name: GO, enabled: true, size: "" },
          { name: NODE, enabled: true, size: "" },
          { name: PYTHON, enabled: true, size: "" },
          { name: DOTNET, enabled: true, size: "" },
          { name: CPP, enabled: true, size: "" },
          { name: GLEAM, enabled: true, size: "8G" },
        ],
        // Untouched by this form, and sent back as it stands: one request
        // writes the whole of `config.yaml`.
        cleanup: CLEANUP,
        conflict_resolution: TOLD.conflict_resolution,
        share_on_done: TOLD.share_on_done,
        ...PATHS,
        // And the text every session is given, likewise: what is sent is what the
        // file holds afterwards, so a save that left it out would clear it.
        instructions: TOLD.instructions,
      }),
    );
  });

  /// Clearing the field asks for the default back, which is what an empty size
  /// means to the server — and the placeholder is what says so.
  it("sends an empty size for a field the human cleared", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(compiling(UNSET))),
    );
    mountPane();

    const field = await waitFor(() => screen.getByLabelText(/How large/));
    fireEvent.input(field, { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(languagesSent(fetching)[0]?.size).toBe(""));
  });

  /// And a size the pane drew no field for is sent all the same.
  ///
  /// One request writes the whole of `config.yaml`, and `size` is a key of every
  /// language's entry — not only of the one whose store an sccache bounds, which
  /// is the only one with a box for it here. A tick that sent what it had drawn
  /// would write the file with the installer's own size gone, which is a key
  /// nobody was ever shown being emptied by a press about something else.
  it("sends the size of a language it drew no field for", async () => {
    const fetching = theSettings(compiling(TOLD), json(answering(off(TOLD))));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
    expect(
      screen.queryAllByLabelText(/How large/),
      "only the language whose store an sccache bounds has a field",
    ).toHaveLength(1);

    fireEvent.click(theCheck());

    await waitFor(() =>
      expect(languagesSent(fetching)).toEqual([
        { name: RUST, enabled: false, size: "50G" },
        { name: GO, enabled: true, size: "" },
        { name: NODE, enabled: true, size: "" },
        { name: PYTHON, enabled: true, size: "" },
        { name: DOTNET, enabled: true, size: "" },
        { name: CPP, enabled: true, size: "" },
        { name: GLEAM, enabled: true, size: "8G" },
      ]),
    );
  });

  /// What the pane saved is what the card goes back to saying, because the
  /// answer is a fresh read of the files that both halves are drawn from.
  it("takes a language off the card when the pane unticks it", async () => {
    theSettings(TOLD, json(answering(off(TOLD))));
    const { container } = mounting(() => (
      <>
        <LanguagesCard open press={() => {}} />
        <LanguagesPane back={() => {}} />
      </>
    ));

    await waitFor(() => expect(theCheck().checked).toBe(true));
    expect(container.querySelector(`.${styles.standing}`)?.textContent).toBe(
      "Rust, Go, Node, Python, .NET, C/C++, Gleam",
    );

    fireEvent.click(theCheck());

    await waitFor(() =>
      expect(container.querySelector(`.${styles.standing}`)?.textContent).toBe(
        "Go, Node, Python, .NET, C/C++, Gleam",
      ),
    );
  });

  /// A save that would not land keeps the page honest about it: a settings page
  /// that quietly saved nothing is how a machine ends up not being what it says.
  it("says so when the save fails", async () => {
    serving(whenever("/api/ui/settings", json(TOLD)), () =>
      Promise.resolve(
        new Response("nope", { status: 503, statusText: "Service Unavailable" }),
      ),
    );
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
    fireEvent.click(theCheck());

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});
