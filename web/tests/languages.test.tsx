//! Language support on the settings page: which languages the card names, what
//! the checkboxes in its pane put on the wire, and the size that hangs off
//! every one with a store of its own.
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

import type {
  LanguageCleared,
  SettingsSaved,
  SettingsView,
} from "../src/api/types";
import card from "../src/CardButton.module.css";
import check from "../src/Check.module.css";
import {
  LanguagesCard,
  LanguagesPane,
  bytesSaid,
} from "../src/settings/Languages";
import styles from "../src/settings/Languages.module.css";
import { type Answer, json, serving, whenever } from "./serving";
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
const JVM = "jvm";
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

/// And how much Verkstead runs at once as a save puts it back: both numbers as
/// the strings a form holds, carried by every section for that reason again —
/// see [`heldAtOnce`].
const AT_ONCE = { roadmap_stages: "1", conversations: "2" };

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
  ...answers: Array<Answer>
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
    refused_sizes: [],
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

/// And the group hanging off one, which holds the size — Rust's, the first
/// drawn, unless another is named.
function theGroup(container: ParentNode, name = RUST): HTMLFieldSetElement {
  const group = container
    .querySelector(`#language-size-${name}`)
    ?.closest<HTMLFieldSetElement>(`.${check.nested}`);
  expect(group, "expected the nested group to be drawn").toBeTruthy();
  return group!;
}

/// And the size field in it, by the language it belongs to.
function theSize(name = RUST): HTMLInputElement {
  return screen.getByLabelText(/How large/, {
    selector: `#language-size-${name}`,
  }) as HTMLInputElement;
}

/// And the Save beside that field.
function theSave(name = RUST): HTMLButtonElement {
  return theSize(name)
    .closest("form")!
    .querySelector<HTMLButtonElement>("button[type=submit]")!;
}

describe("the card", () => {
  /// The summary is the list of languages that have build support on, in the
  /// labels the descriptors gave them.
  it("names every language that is on", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountCard();

    await waitFor(() =>
      expect(container.querySelector(`.${styles.standing}`)?.textContent).toBe(
        "Rust, Go, Node, Python, .NET, C/C++, JVM, Gleam",
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
        "Rust, Go, Node, Python, .NET, C/C++, JVM",
      ),
    );
  });

  /// And nothing at all under the heading where none are: a card saying a
  /// machine builds nothing would be a line nobody needs.
  it("says nothing under the heading while they are all off", async () => {
    theSettings(
      off(
        off(off(off(off(off(off(off(TOLD), GO), NODE), PYTHON), DOTNET), CPP), JVM),
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
    const warning = container.querySelector(`.${styles.warning}`);
    expect(warning).not.toBeNull();
    expect(
      warning!.textContent,
      "C/C++ compiles through it too, and has no crates or downloads",
    ).not.toMatch(/crate|download|dependenc/i);
  });

  /// And nothing about it while the languages that want one are switched off,
  /// because a warning about a language's compiles is only true while there is
  /// a language compiling. Both of them: C/C++
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
      screen.getByText("Rust, Go, Node, Python, .NET, C/C++, JVM"),
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
    expect(theCheck("JVM").checked).toBe(true);
    expect(theCheck("Gleam").checked).toBe(true);
    expect(screen.getAllByRole("checkbox")).toHaveLength(8);
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
    expect(theSize().matches(":disabled")).toBe(true);
    expect(
      theSave().matches(":disabled"),
    ).toBe(true);
  });

  /// And greyed again while the box is unticked, which is the other half of the
  /// pattern: the configuration hanging off a checkbox means nothing while the
  /// checkbox is off. Both boxes naming the Compile Server, because with C/C++
  /// on the size is still live under Rust's — see the two tests below.
  it("greys the size while the box is unticked", async () => {
    theSettings(off(off(compiling(TOLD)), CPP));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(false));

    expect(theGroup(container).disabled).toBe(true);
    expect(theSize().matches(":disabled")).toBe(true);
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

    fireEvent.click(theSave());

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
    expect(theSize().matches(":disabled")).toBe(false);
  });

  /// Every language with a store of its own has a size — an installer's own
  /// among them — and C/C++, which has none, has no field.
  it("draws a size under every language with a store, and none under C/C++", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck("Gleam").checked).toBe(true));

    const fields = screen
      .getAllByLabelText(/How large/)
      .map((field) => field.id);

    expect(fields).toEqual(
      [RUST, GO, NODE, PYTHON, DOTNET, JVM, GLEAM].map(
        (name) => `language-size-${name}`,
      ),
    );
    expect(container.querySelectorAll(`.${check.nested}`)).toHaveLength(7);
  });

  /// And a store nothing but Verkstead bounds takes its size whether or not
  /// there is an sccache: only the Compile Server's size is sccache's word.
  it("lets a package store's size be typed where there is no sccache", async () => {
    theSettings(UNSET);
    const { container } = mountPane();

    await waitFor(() => screen.getByText(/No sccache is installed/));

    expect(theGroup(container, GO).disabled).toBe(false);
    expect(theSize(GO).matches(":disabled")).toBe(false);
  });

  /// And greys it while its own box is unticked, like Rust's.
  it("greys a package store's size while its box is unticked", async () => {
    theSettings(off(compiling(TOLD), GO));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck("Go").checked).toBe(false));

    expect(theGroup(container, GO).disabled).toBe(true);
    expect(theGroup(container, NODE).disabled).toBe(false);
  });

  /// One Compile Server between the two languages naming it, so one size: the
  /// field is on the language that sizes it, and the other says whose it is.
  it("draws the one size on Rust and says C/C++ shares its Compile Server", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await waitFor(() => expect(theCheck("C/C++").checked).toBe(true));

    expect(
      screen.queryByLabelText(/How large/, {
        selector: `#language-size-${CPP}`,
      }),
    ).toBeNull();
    expect(theSize().id).toBe(`language-size-${RUST}`);
    expect(
      screen.getByText(/Shares the Compile Server with Rust/),
    ).toBeTruthy();
  });

  /// And with Rust off the size stays Rust's, live while C/C++ still compiles
  /// through the server — the same answer the server gives about which size it
  /// starts at. Moved to C/C++, it would be the store shrinking to C/C++'s
  /// default on a press about something else.
  it("keeps the size on Rust, live, when Rust is off and C/C++ is on", async () => {
    theSettings(off(compiling(TOLD)));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(false));

    expect(theSize().id).toBe(`language-size-${RUST}`);
    expect(theGroup(container).disabled).toBe(false);
    expect(
      screen.getByText(/Shares the Compile Server with Rust/),
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
      theSize(),
    )) as HTMLInputElement;

    expect(field.value).toBe("");
    expect(field.placeholder).toBe("30G");
    expect(
      theSize(GO).placeholder,
      "and every store but Rust's starts at 10G",
    ).toBe("10G");
  });

  /// And the placeholder is the default even where a size is configured, so
  /// that clearing the field shows what clearing it asks for.
  it("draws the default as the placeholder under a configured size", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await waitFor(() => expect(theSize().value).toBe("50G"));

    expect(theSize().placeholder).toBe("30G");
    expect(theSize(GLEAM).placeholder).toBe("10G");
  });

  /// A size a hand-edit wrote that is not one is drawn as written, with the
  /// server's reason and the default the store is held to meanwhile.
  it("says why a hand-written size is not the one in force", async () => {
    const standing = compiling(TOLD);
    theSettings({
      ...standing,
      languages: standing.languages.map((language) =>
        language.name === GO
          ? {
              ...language,
              size: "lots",
              size_configured: true,
              size_unread: "is not a size: write 10G",
            }
          : language,
      ),
    });
    const { container } = mountPane();

    await waitFor(() => expect(theSize(GO).value).toBe("lots"));

    expect(
      theGroup(container, GO).querySelector(`.${styles.warning}`)?.textContent,
    ).toBe(
      "Its size in config.yaml, lots, is not a size: write 10G, so its store " +
        "is held to 10G.",
    );
  });

  it("draws a size somebody configured as the value", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    const field = (await waitFor(() =>
      theSize(),
    )) as HTMLInputElement;

    expect(field.value).toBe("50G");
  });

  /// What each store holds, beside its size: the last figure the server
  /// measured, with the part of it the sweep holds to the size, that it has
  /// not measured one yet, or — for a descriptor naming no store directory —
  /// that there is nothing to measure.
  it("draws each store's disk use beside its size", async () => {
    theSettings({
      ...TOLD,
      languages: TOLD.languages.map((language) =>
        language.name === GO
          ? {
              ...language,
              disk_use: {
                Measured: { bytes: 9.6 * 2 ** 30, in_units: 4 * 2 ** 30 },
              },
            }
          : language,
      ),
    });
    mountPane();

    const said =
      "Holds 9.6G on disk, 4.0G of it in the packages the sweep holds to this size.";

    await waitFor(() => screen.getByText(said));

    const under = (name: string) =>
      theSize(name).closest("form")?.querySelector(`.${styles.held}`)
        ?.textContent;

    expect(under(GO)).toBe(said);
    expect(under(NODE)).toBe("Not measured yet.");
    expect(under(GLEAM)).toMatch(/names no store directory/);
  });
});

describe("saying how a store is held to its size", () => {
  /// Each directory of a store by name, with how it is kept under the size —
  /// by its tool, by the sweep, or not at all — and Rust's two halves said to
  /// be held to it separately.
  it("draws how each directory of a store is bounded", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    await waitFor(() => theSize(RUST));

    const lines = (name: string) =>
      Array.from(
        theGroup(container, name).querySelectorAll(`.${styles.bounds} li`),
      ).map((line) => line.textContent);

    expect(lines(RUST)).toEqual([
      "cargo is swept to this size by whole packages, oldest first.",
      "sccache is handed this size and evicts for itself.",
    ]);
    expect(theGroup(container, RUST).textContent).toContain(
      "Each is held to it separately, so together they may hold up to twice it.",
    );

    expect(lines(DOTNET)).toContain(
      "scratch is never swept: its descriptor names no unit, so it grows as " +
        "its tool fills it.",
    );
    expect(theGroup(container, DOTNET).textContent).not.toContain(
      "twice it",
    );

    expect(lines(GLEAM)).toEqual([]);
  });

  /// When the sweep last brought a store under its size, or that it has not
  /// since the server started and why — and nothing for a language whose
  /// stores the sweep never touches.
  it("says when each swept store was last swept", async () => {
    theSettings({
      ...TOLD,
      languages: TOLD.languages.map((language) =>
        language.name === GO
          ? { ...language, swept: "2026-10-01T09:30:00Z" }
          : language,
      ),
    });
    const { container } = mountPane();

    await waitFor(() => screen.getByText("Last swept 2026-10-01 09:30 UTC."));

    expect(theGroup(container, NODE).textContent).toContain(
      "Not swept since the server started: a sweep waits until no session " +
        "or terminal is running.",
    );
    expect(theGroup(container, GLEAM).textContent).not.toMatch(/swept since/);
  });
});

describe("saying how much a store holds", () => {
  /// In the size grammar's own units, so a figure reads against the size
  /// beside it without a conversion.
  it("says bytes in K, M, G and T", () => {
    expect(bytesSaid(0)).toBe("0 bytes");
    expect(bytesSaid(1023)).toBe("1023 bytes");
    expect(bytesSaid(1536)).toBe("1.5K");
    expect(bytesSaid(20 * 2 ** 20)).toBe("20M");
    expect(bytesSaid(30 * 2 ** 30)).toBe("30G");
    expect(bytesSaid(2 * 2 ** 40)).toBe("2.0T");
  });
});

/// The Clear under a language's size, which stands outside the group the size
/// hangs in — a language switched off still holds what it fetched.
function theClear(container: ParentNode, name = RUST): HTMLButtonElement {
  const clearing = theGroup(container, name).nextElementSibling;
  expect(clearing?.className).toBe(styles.clearing);
  return clearing!.querySelector("button")!;
}

/// The same settings with Go's store holding `bytes`.
function holding(standing: SettingsView, bytes: number): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) =>
      language.name === GO
        ? { ...language, disk_use: { Measured: { bytes, in_units: null } } }
        : language,
    ),
  };
}

describe("clearing a store", () => {
  /// A press empties that language's stores, and what the answer measured is
  /// what is drawn.
  it("clears one language and draws what its store holds after", async () => {
    const cleared: LanguageCleared = {
      Cleared: { settings: holding(TOLD, 0) },
    };
    const fetching = theSettings(
      holding(TOLD, 5 * 2 ** 30),
      whenever("/api/ui/languages/go/clear", json(cleared), "POST"),
    );
    const { container } = mountPane();

    await waitFor(() =>
      expect(theGroup(container, GO).textContent).toContain(
        "Holds 5.0G on disk.",
      ),
    );
    expect(theClear(container, GO).disabled).toBe(false);

    fireEvent.click(theClear(container, GO));

    await waitFor(() =>
      expect(theGroup(container, GO).textContent).toContain(
        "Holds 0 bytes on disk.",
      ),
    );
    expect(
      fetching.mock.calls.some(
        ([asked, init]) =>
          String(asked) === "/api/ui/languages/go/clear" &&
          init?.method === "POST",
      ),
    ).toBe(true);
  });

  /// Off while anything runs, saying what it waits on.
  it("is off while a session or terminal runs, and says how many", async () => {
    theSettings({ ...TOLD, running: { sessions: 2, terminals: 1 } });
    const { container } = mountPane();

    await waitFor(() => expect(theClear(container, GO).disabled).toBe(true));
    expect(theClear(container, GO).parentElement!.textContent).toContain(
      "Clear waits until no session or terminal is running: 2 sessions and " +
        "1 terminal are running.",
    );
  });

  /// And a Clear the server refused, something having started since the read,
  /// goes off the same way.
  it("draws a refusal as what it waits on", async () => {
    const refused: LanguageCleared = {
      Running: { sessions: 1, terminals: 0 },
    };
    theSettings(
      TOLD,
      whenever("/api/ui/languages/go/clear", json(refused), "POST"),
    );
    const { container } = mountPane();

    await waitFor(() => expect(theClear(container, GO).disabled).toBe(false));
    fireEvent.click(theClear(container, GO));

    await waitFor(() =>
      expect(theClear(container, GO).parentElement!.textContent).toContain(
        "1 session is running.",
      ),
    );
    expect(theClear(container, GO).disabled).toBe(true);
  });

  /// C/C++ has no store of its own, so nothing to clear.
  it("draws no Clear for a language with no store", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => theSize(RUST));

    expect(screen.getAllByRole("button", { name: "Clear" }).length).toBe(
      TOLD.languages.filter(
        (language) => language.disk_use !== "NoStore" && language.store,
      ).length,
    );
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
          { name: JVM, enabled: true, size: "" },
          { name: GLEAM, enabled: true, size: "8G" },
        ],
        // Untouched by this form, and sent back as it stands: one request
        // writes the whole of `config.yaml`.
        cleanup: CLEANUP,
        at_once: AT_ONCE,
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
        { name: JVM, enabled: true, size: "" },
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
    const fetching = theSettings(compiling(TOLD), json(answering(off(TOLD))));
    mountPane();

    const field = await waitFor(() => theSize());
    fireEvent.input(field, { target: { value: "5" } });

    fireEvent.click(theCheck());

    await waitFor(() => expect(languagesSent(fetching)[0]?.size).toBe("50G"));

    // And what was typed is still there to finish typing: the tick did not
    // commit it, so the field did not let go of it either.
    await waitFor(() => expect(theCheck().checked).toBe(false));
    expect((theSize() as HTMLInputElement).value).toBe(
      "5",
    );
  });

  /// The size is typed, so it waits for a press: nothing is committed while
  /// somebody is halfway through writing `30`.
  it("sends a size only when it is saved", async () => {
    const bigger = compiling(TOLD);
    const fetching = theSettings(compiling(TOLD), json(answering(bigger)));
    mountPane();

    const field = await waitFor(() => theSize());
    fireEvent.input(field, { target: { value: "80G" } });

    expect(
      fetching.mock.calls.some(([, init]) => init?.method === "POST"),
      "typing is not saving",
    ).toBe(false);

    fireEvent.click(theSave());

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
          { name: JVM, enabled: true, size: "" },
          { name: GLEAM, enabled: true, size: "8G" },
        ],
        // Untouched by this form, and sent back as it stands: one request
        // writes the whole of `config.yaml`.
        cleanup: CLEANUP,
        at_once: AT_ONCE,
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

    const field = await waitFor(() => theSize());
    fireEvent.input(field, { target: { value: "" } });
    fireEvent.click(theSave());

    await waitFor(() => expect(languagesSent(fetching)[0]?.size).toBe(""));
  });

  /// And a size the pane drew no field for is sent all the same.
  ///
  /// One request writes the whole of `config.yaml`, and `size` is a key of every
  /// language's entry — C/C++'s too, which has no store and no box for it here.
  /// A tick that sent what it had drawn would write the file with that size
  /// gone, which is a key nobody was ever shown being emptied by a press about
  /// something else.
  it("sends the size of a language it drew no field for", async () => {
    const fetching = theSettings(compiling(TOLD), json(answering(off(TOLD))));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
    expect(
      screen.queryByLabelText(/How large/, {
        selector: `#language-size-${CPP}`,
      }),
      "C/C++ has no store, so no field",
    ).toBeNull();

    fireEvent.click(theCheck());

    await waitFor(() =>
      expect(languagesSent(fetching)).toEqual([
        { name: RUST, enabled: false, size: "50G" },
        { name: GO, enabled: true, size: "" },
        { name: NODE, enabled: true, size: "" },
        { name: PYTHON, enabled: true, size: "" },
        { name: DOTNET, enabled: true, size: "" },
        { name: CPP, enabled: true, size: "" },
        { name: JVM, enabled: true, size: "" },
        { name: GLEAM, enabled: true, size: "8G" },
      ]),
    );
  });

  /// A size the server cannot read is turned down with the save, and the
  /// reason drawn under the field still holding what was typed.
  it("draws a refused size's reason and keeps what was typed", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json({
        ...answering(compiling(TOLD)),
        refused_sizes: [{ language: GO, why: "is not a size: write 10G" }],
      }),
    );
    mountPane();

    const field = await waitFor(() => theSize(GO));
    fireEvent.input(field, { target: { value: "1.5G" } });
    fireEvent.click(theSave(GO));

    await waitFor(() => screen.getByText(/is not a size: write 10G/));

    expect(languagesSent(fetching)[1]).toEqual({
      name: GO,
      enabled: true,
      size: "1.5G",
    });
    expect(screen.getByText(/is not a size/).textContent).toBe(
      "1.5G is not a size: write 10G.",
    );
    expect(theSize(GO).value, "what was typed is still there").toBe("1.5G");
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
      "Rust, Go, Node, Python, .NET, C/C++, JVM, Gleam",
    );

    fireEvent.click(theCheck());

    await waitFor(() =>
      expect(container.querySelector(`.${styles.standing}`)?.textContent).toBe(
        "Go, Node, Python, .NET, C/C++, JVM, Gleam",
      ),
    );
  });

  /// A save that would not land keeps the page honest about it: a settings page
  /// that quietly saved nothing is how a machine ends up not being what it says.
  it("says so when the save fails", async () => {
    serving(whenever("/api/ui/settings", json(TOLD)), () =>
      Promise.resolve(
        new Response("nope", {
          status: 503,
          statusText: "Service Unavailable",
        }),
      ),
    );
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
    fireEvent.click(theCheck());

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});
