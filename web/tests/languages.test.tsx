//! Language support on the settings page: which languages the card names, what
//! the checkboxes in its pane put on the wire, and the link beside each one —
//! what its cache holds — with the panel behind it holding the size and the
//! rest.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane naming the languages that have build support on, and the controls that
//! change them in the details pane it opens. Each is mounted on its own, and the
//! pair together only where the round trip is what is being asked about.
//!
//! **The page is drawn from what the server loaded**, so the fixture carries the
//! descriptors Verkstead ships and one an installer wrote into `config.yaml`. A
//! box under a label nothing in the viewer knows is the whole of what this
//! section is for, and a suite that only ever saw Rust would prove only that the
//! page can draw the language it was written around.
//!
//! Two saves and one endpoint. A checkbox is its own press, because a box that
//! needed a second one is not a box; the size is typed, so it saves on Enter or
//! on leaving the field — and not on leaving one nobody changed, which is every
//! close of its panel. Both send the whole of the settings edit — the author as
//! it stands and the token untouched — because the server writes both files in
//! one request, and that is what these check is not lost.
//!
//! The size field is in a panel built when it opens, so every test reaching it
//! presses the link first. It is greyed and refuses input while the box is
//! unticked, and again while there is no sccache to read it. Both of those are
//! here, because the second is the one nobody would think to try.
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
import {
  LanguagesCard,
  LanguagesPane,
  bytesSaid,
  usageSaid,
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

function theSettings(standing: SettingsView, ...answers: Array<Answer>) {
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

/// The link beside a language's box, by the label the server gave it — or
/// `undefined` where the row has none.
function linkOf(label: string): HTMLButtonElement | undefined {
  return screen
    .queryAllByRole("button")
    .find((button) =>
      button.getAttribute("aria-label")?.startsWith(`${label}: `),
    ) as HTMLButtonElement | undefined;
}

/// The words a link reads, which is what stands to the right of the box.
function linkSaid(label: string): string | undefined {
  return linkOf(label)?.textContent ?? undefined;
}

/// A language's panel, opened by pressing its link — waited for, because the
/// row stands on a read.
async function opened(label = "Rust"): Promise<HTMLElement> {
  const link = await waitFor(() => {
    const found = linkOf(label);
    expect(found, `expected ${label} to have a link`).toBeTruthy();
    return found!;
  });

  if (link.getAttribute("aria-expanded") !== "true") {
    fireEvent.click(link);
  }

  return screen.getByRole("group", { name: `${label} cache` });
}

/// And the size field in an open panel, by the language it belongs to.
function theSize(name = RUST): HTMLInputElement {
  return screen.getByLabelText(/How large/, {
    selector: `#language-size-${name}`,
  }) as HTMLInputElement;
}

/// Let whatever a press set going reach the network: a mutation calls `fetch`
/// a few microtasks after it is asked, so a count taken straight after a
/// press would miss a save that is on its way.
async function settled(): Promise<void> {
  await new Promise((done) => setTimeout(done, 20));
}

/// How many saves the page has sent.
function saves(fetching: ReturnType<typeof serving>): number {
  return fetching.mock.calls.filter(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  ).length;
}

/// The same settings with one language's store measured at `bytes`, of which
/// `in_units` are whole packages.
function measured(
  standing: SettingsView,
  name: string,
  bytes: number,
  in_units: number | null = null,
): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) =>
      language.name === name
        ? { ...language, disk_use: { Measured: { bytes, in_units } } }
        : language,
    ),
  };
}

const GB = 2 ** 30;

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
        off(
          off(off(off(off(off(off(TOLD), GO), NODE), PYTHON), DOTNET), CPP),
          JVM,
        ),
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

  /// The link stands beside the box rather than inside its label: a press
  /// anywhere in the label ticks the box, and pressing the link must not.
  it("opens a panel from the link without ticking the box", async () => {
    const fetching = theSettings(TOLD);
    mountPane();

    const panel = await opened("Go");

    expect(theCheck("Go").closest("label")!.contains(linkOf("Go")!)).toBe(
      false,
    );
    expect(panel.contains(theSize(GO))).toBe(true);
    await settled();
    expect(saves(fetching), "opening a panel is not a tick").toBe(0);
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
    await opened();
    expect(theSize().matches(":disabled")).toBe(true);
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

  /// Nothing about sccache at the foot of the pane: it is in the panels of the
  /// languages it is about, and on the card.
  it("says nothing about sccache at the foot of the pane", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));

    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });
});

describe("the link beside each language", () => {
  /// Everything the store holds against the size in force, in the size's unit
  /// as a human writes it, with a decimal under ten — and Rust's own size,
  /// not twice it.
  it("says what a ticked store holds against its size", async () => {
    theSettings(measured(measured(TOLD, RUST, 3.2 * GB), GO, 0.4 * GB));
    mountPane();

    await waitFor(() => expect(linkSaid("Rust")).toBe("Cache usage: 3.2/50GB"));
    expect(linkSaid("Go"), "one decimal, so a small cache is not 0").toBe(
      "Cache usage: 0.4/10GB",
    );
  });

  /// The default where a size written by hand is not one, since that is what
  /// the store is held to.
  it("measures against the default where the size in the file is not one", async () => {
    const standing = measured(TOLD, GO, 2 * GB);
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
    mountPane();

    await waitFor(() => expect(linkSaid("Go")).toBe("Cache usage: 2.0/10GB"));
  });

  it("says a store not measured yet is not", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() =>
      expect(linkSaid("Node")).toBe("Cache usage: not measured yet"),
    );
  });

  /// An unticked language still holds what it fetched, so its row keeps a
  /// way to it — quieter, and saying only what it holds.
  it("draws a quieter Holds link on an unticked row that holds something", async () => {
    theSettings(off(measured(TOLD, PYTHON, 1.8 * GB), PYTHON));
    mountPane();

    await waitFor(() => expect(linkSaid("Python")).toBe("Holds 1.8GB"));
    expect(linkOf("Python")!.parentElement!.classList).toContain(styles.quiet);
    expect(linkOf("Go")!.parentElement!.classList).not.toContain(styles.quiet);
  });

  /// And one not measured yet may hold something, so it keeps its link; one
  /// measured at nothing has nothing to read and nothing to clear.
  it("draws no link on an unticked row measured at nothing", async () => {
    theSettings(off(off(measured(TOLD, GO, 0), GO), NODE));
    mountPane();

    await waitFor(() => expect(theCheck("Go").checked).toBe(false));
    expect(linkOf("Go")).toBeUndefined();
    expect(linkSaid("Node")).toBe("Store not measured yet");
  });

  /// Except where its size is still live: Rust's sizes the Compile Server
  /// C/C++ is still compiling through.
  it("keeps Rust's link while its size is live for C/C++", async () => {
    theSettings(off(measured(compiling(TOLD), RUST, 0)));
    mountPane();

    await waitFor(() => expect(linkSaid("Rust")).toBe("Holds 0 bytes"));
  });

  /// A language with no store directory and no Compile Server has nothing to
  /// measure and nothing its size holds, so nothing stands beside its box.
  it("draws nothing beside a language with no store", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => expect(theCheck("Gleam").checked).toBe(true));
    expect(linkOf("Gleam")).toBeUndefined();
  });

  /// C/C++ has no store of its own: its link says whose cache it compiles
  /// through, and its panel says so with no field of its own.
  it("says C/C++ compiles through Rust's cache", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await waitFor(() =>
      expect(linkSaid("C/C++")).toBe("Compiles through Rust's cache"),
    );

    const panel = await opened("C/C++");
    expect(panel.textContent).toContain("Compiles through the Compile Server");
    expect(panel.querySelector("input")).toBeNull();
    expect(screen.queryByRole("button", { name: "Clear" })).toBeNull();
  });

  /// No sentence said per language outside a panel: the row is the box and
  /// the link.
  it("says nothing per language outside a panel", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
    expect(container.querySelectorAll("p")).toHaveLength(0);
    expect(container.querySelectorAll("input[type=text]")).toHaveLength(0);
  });
});

describe("the size in a language's panel", () => {
  /// With no sccache the size is still read, by the trimming holding Rust's
  /// cargo half to it, so the field stays live while Rust is on — beside the
  /// warning that its compiling is not cached.
  it("keeps the size live where there is no sccache, for the cargo half", async () => {
    theSettings(UNSET);
    mountPane();

    const panel = await opened();

    expect(panel.textContent).toMatch(/No sccache is installed/);
    expect(theSize().matches(":disabled")).toBe(false);
  });

  /// And greyed where Rust is off with no sccache: C/C++ on compiles through
  /// nothing that reads it, and Rust's own store is the box's that is off.
  it("greys the size where there is no sccache and Rust is off", async () => {
    theSettings(off(UNSET));
    mountPane();

    await opened();
    expect(theSize().matches(":disabled")).toBe(true);
  });

  /// And greyed again while the box is unticked. Both boxes naming the Compile
  /// Server, because with C/C++ on the size is still live under Rust's.
  it("greys the size while the box is unticked", async () => {
    theSettings(off(off(compiling(TOLD)), CPP));
    mountPane();

    await opened();
    expect(theSize().matches(":disabled")).toBe(true);
  });

  /// And it takes input where both halves say it means something.
  it("lets the size be typed where the box is on and sccache is there", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await opened();
    expect(theSize().matches(":disabled")).toBe(false);
  });

  /// Every language with a store directory has a size — and only Rust's
  /// panel has the Compile Server's.
  it("draws a size in the panel of every language with a store", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    for (const [label, name] of [
      ["Rust", RUST],
      ["Go", GO],
      ["Node", NODE],
      ["Python", PYTHON],
      [".NET", DOTNET],
      ["JVM", JVM],
    ] as const) {
      const panel = await opened(label);
      expect(panel.contains(theSize(name))).toBe(true);
      fireEvent.keyDown(document, { key: "Escape" });
    }
  });

  /// And a store nothing but Verkstead bounds takes its size whether or not
  /// there is an sccache: only the Compile Server's size is sccache's word.
  it("lets a package store's size be typed where there is no sccache", async () => {
    theSettings(UNSET);
    mountPane();

    await opened("Go");
    expect(theSize(GO).matches(":disabled")).toBe(false);
  });

  /// And greys it while its own box is unticked, like Rust's.
  it("greys a package store's size while its box is unticked", async () => {
    theSettings(off(compiling(TOLD), GO));
    mountPane();

    await opened("Go");
    expect(theSize(GO).matches(":disabled")).toBe(true);
  });

  /// And with Rust off the size stays Rust's, live while C/C++ still compiles
  /// through the server — the same answer the server gives about which size it
  /// starts at.
  it("keeps the size on Rust, live, when Rust is off and C/C++ is on", async () => {
    theSettings(off(compiling(TOLD)));
    mountPane();

    await opened();
    expect(theSize().matches(":disabled")).toBe(false);
  });

  /// An unconfigured size is the default drawn as a placeholder rather than as
  /// text somebody typed.
  it("draws a size nobody configured as the placeholder", async () => {
    theSettings(compiling(UNSET));
    mountPane();

    await opened();
    expect(theSize().value).toBe("");
    expect(theSize().placeholder).toBe("30G");

    await opened("Go");
    expect(
      theSize(GO).placeholder,
      "and every store but Rust's starts at 10G",
    ).toBe("10G");
  });

  /// And the placeholder is the default even where a size is configured, so
  /// that clearing the field shows what clearing it asks for.
  it("draws a configured size as the value, over the default", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await opened();
    expect(theSize().value).toBe("50G");
    expect(theSize().placeholder).toBe("30G");
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
    mountPane();

    const panel = await opened("Go");

    expect(theSize(GO).value).toBe("lots");
    expect(panel.querySelector(`.${styles.warning}`)?.textContent).toBe(
      "Its size in config.yaml, lots, is not a size: write 10G, so its store " +
        "is held to 10G.",
    );
  });
});

describe("the rest of a language's panel", () => {
  /// What the store holds on disk and how much of it is packages, said once in
  /// the panel rather than under every box.
  it("says what the store holds and how much of it is packages", async () => {
    theSettings(measured(TOLD, GO, 9.6 * GB, 4 * GB));
    mountPane();

    const panel = await opened("Go");
    expect(panel.textContent).toContain(
      "Holds 9.6GB on disk, 4.0GB of it in packages.",
    );

    const node = await opened("Node");
    expect(node.textContent).toContain("Not measured yet");
  });

  /// The directories named once, on one line, in the server's order.
  it("names the store's directories on one line", async () => {
    theSettings(TOLD);
    mountPane();

    const panel = await opened();
    expect(panel.textContent).toContain("Kept in cargo, sccache.");
  });

  /// The exceptions, a sentence each: sccache holding itself to the size, and
  /// a directory nothing trims. The ordinary case says nothing.
  it("says the exceptions and only the exceptions", async () => {
    theSettings(TOLD);
    mountPane();

    const rust = await opened();
    expect(rust.textContent).toContain("up to twice it");
    fireEvent.keyDown(document, { key: "Escape" });

    const dotnet = await opened(".NET");
    expect(dotnet.textContent).toContain(
      "scratch is never trimmed, so it grows as its tool fills it.",
    );
    expect(dotnet.textContent).not.toContain("twice it");
    fireEvent.keyDown(document, { key: "Escape" });

    const node = await opened("Node");
    expect(node.textContent).not.toMatch(/never trimmed|twice it/);
  });

  /// None of Verkstead's own words for how it does it: nothing per directory
  /// claiming the size, which read as the size seven times over.
  it("says nothing about a sweep, a unit or a descriptor", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    for (const label of ["Rust", "Go", "Node", ".NET", "C/C++"]) {
      await opened(label);
      expect(container.textContent).not.toMatch(/swept|sweep|unit|descriptor/i);
      fireEvent.keyDown(document, { key: "Escape" });
    }
  });

  /// When the store was last trimmed, as an age with the exact stamp behind
  /// it — or that it has not been since the server started, and why.
  it("says when a store was last trimmed", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date("2026-10-01T11:30:00Z"));

    try {
      theSettings({
        ...TOLD,
        languages: TOLD.languages.map((language) =>
          language.name === GO
            ? { ...language, swept: "2026-10-01T09:30:00Z" }
            : language,
        ),
      });
      mountPane();

      await opened("Go");
      const said = screen.getByText("Last trimmed 2h ago.");
      expect(said.getAttribute("title")).toBe("2026-10-01 09:30 UTC");
      fireEvent.keyDown(document, { key: "Escape" });

      const node = await opened("Node");
      expect(node.textContent).toContain(
        "Not trimmed since the server started: trimming waits until no " +
          "session or terminal is running.",
      );
    } finally {
      vi.useRealTimers();
    }
  });

  /// The sccache warning is in the panels of the two languages compiling
  /// through it, and nowhere else in the pane.
  it("warns about sccache in Rust's and C/C++'s panels only", async () => {
    theSettings(UNSET);
    mountPane();

    const rust = await opened();
    expect(rust.textContent).toMatch(/No sccache is installed/);
    fireEvent.keyDown(document, { key: "Escape" });

    const cpp = await opened("C/C++");
    expect(cpp.textContent).toMatch(/No sccache is installed/);
    fireEvent.keyDown(document, { key: "Escape" });

    const go = await opened("Go");
    expect(go.textContent).not.toMatch(/sccache/);
  });

  /// And only for a language that is on: off, nothing of it is compiled.
  it("says nothing about sccache in the panel of a language that is off", async () => {
    theSettings(off(UNSET, CPP));
    mountPane();

    const cpp = await opened("C/C++");
    expect(cpp.textContent).not.toMatch(/No sccache is installed/);
  });

  it("says nothing about sccache where the server found one", async () => {
    theSettings(compiling(UNSET));
    mountPane();

    const rust = await opened();
    expect(rust.textContent).not.toMatch(/No sccache is installed/);
  });
});

describe("saying how much a store holds", () => {
  /// In the units a human writes a size in, so a figure reads against the
  /// size beside it without a conversion.
  it("says bytes in KB, MB, GB and TB", () => {
    expect(bytesSaid(0)).toBe("0 bytes");
    expect(bytesSaid(1023)).toBe("1023 bytes");
    expect(bytesSaid(1536)).toBe("1.5KB");
    expect(bytesSaid(20 * 2 ** 20)).toBe("20MB");
    expect(bytesSaid(30 * GB)).toBe("30GB");
    expect(bytesSaid(2 * 2 ** 40)).toBe("2.0TB");
  });

  /// And what is held against the size, in the size's unit — or its own,
  /// where it would round to nothing in that one.
  it("says usage against a size in the size's unit", () => {
    expect(usageSaid(3.2 * GB, "30G")).toBe("3.2/30GB");
    expect(usageSaid(0.4 * GB, "10G")).toBe("0.4/10GB");
    expect(usageSaid(12.4 * GB, "30G")).toBe("12/30GB");
    expect(usageSaid(0, "10G")).toBe("0.0/10GB");
    expect(usageSaid(20 * 2 ** 20, "10G")).toBe("20MB/10GB");
    expect(usageSaid(300 * 2 ** 20, "500M")).toBe("300/500MB");
    expect(usageSaid(GB, "lots")).toBe("1.0GB/lots");
  });
});

/// The Clear at the foot of a language's panel.
function theClear(panel: HTMLElement): HTMLButtonElement {
  return panel.querySelector<HTMLButtonElement>(`.${styles.clearing} button`)!;
}

describe("clearing a store", () => {
  /// A press empties that language's stores, and what the answer measured is
  /// what is drawn — in the panel, which stays open over the answer.
  it("clears one language and draws what its store holds after", async () => {
    const cleared: LanguageCleared = {
      Cleared: { settings: measured(TOLD, GO, 0) },
    };
    const fetching = theSettings(
      measured(TOLD, GO, 5 * GB),
      whenever("/api/ui/languages/go/clear", json(cleared), "POST"),
    );
    mountPane();

    const panel = await opened("Go");
    expect(panel.textContent).toContain("Holds 5.0GB on disk.");
    expect(theClear(panel).disabled).toBe(false);

    fireEvent.click(theClear(panel));

    await waitFor(() =>
      expect(panel.textContent).toContain("Holds 0 bytes on disk."),
    );
    expect(panel.isConnected, "the panel is still open").toBe(true);
    expect(linkSaid("Go")).toBe("Cache usage: 0.0/10GB");
    expect(
      fetching.mock.calls.some(
        ([asked, init]) =>
          String(asked) === "/api/ui/languages/go/clear" &&
          init?.method === "POST",
      ),
    ).toBe(true);
  });

  /// And a language switched off still holds what it fetched, so its panel
  /// clears it all the same.
  it("clears a language that is switched off", async () => {
    theSettings(off(measured(TOLD, PYTHON, 1.8 * GB), PYTHON));
    mountPane();

    const panel = await opened("Python");
    expect(theClear(panel).disabled).toBe(false);
  });

  /// Off while anything runs, saying what it waits on.
  it("is off while a session or terminal runs, and says how many", async () => {
    theSettings({ ...TOLD, running: { sessions: 2, terminals: 1 } });
    mountPane();

    const panel = await opened("Go");
    expect(theClear(panel).disabled).toBe(true);
    expect(theClear(panel).parentElement!.textContent).toContain(
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
    mountPane();

    const panel = await opened("Go");
    fireEvent.click(theClear(panel));

    await waitFor(() =>
      expect(theClear(panel).parentElement!.textContent).toContain(
        "1 session is running.",
      ),
    );
    expect(theClear(panel).disabled).toBe(true);
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
  /// a `50` as the store size, which is a number nobody finished. The Cleanup
  /// pane's two durations hold the same rule.
  it("sends the server's size when the box is ticked, not what is typed", async () => {
    const fetching = theSettings(compiling(TOLD), json(answering(off(TOLD))));
    mountPane();

    await opened();
    fireEvent.input(theSize(), { target: { value: "5" } });

    fireEvent.click(theCheck());

    await waitFor(() => expect(languagesSent(fetching)[0]?.size).toBe("50G"));

    // And what was typed is still there to finish typing: the tick did not
    // commit it, so the field did not let go of it either.
    await waitFor(() => expect(theCheck().checked).toBe(false));
    expect(theSize().value).toBe("5");
  });

  /// The size is typed, so typing is not saving; Enter is. And the panel
  /// stays open over the answer.
  it("saves a size on Enter, and only then", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(compiling(TOLD))),
    );
    mountPane();

    const panel = await opened();
    fireEvent.input(theSize(), { target: { value: "80G" } });

    await settled();
    expect(saves(fetching), "typing is not saving").toBe(0);

    fireEvent.submit(theSize().form!);
    // And the field left straight after, as Enter then Escape would: the save
    // already on its way is the one save.
    fireEvent.blur(theSize());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Keep",
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
        cleanup: CLEANUP,
        at_once: AT_ONCE,
        conflict_resolution: TOLD.conflict_resolution,
        share_on_done: TOLD.share_on_done,
        ...PATHS,
        instructions: TOLD.instructions,
      }),
    );

    // And what was typed lets go once the answer is in, so the field follows
    // the server again — in the same panel, still open.
    await waitFor(() => expect(theSize().value).toBe("50G"));
    expect(panel.isConnected).toBe(true);
    expect(saves(fetching)).toBe(1);
  });

  /// And on leaving the field, which is how a panel is usually left.
  it("saves a size on leaving the field", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(compiling(TOLD))),
    );
    mountPane();

    await opened("Go");
    fireEvent.input(theSize(GO), { target: { value: "20G" } });
    fireEvent.blur(theSize(GO));

    await waitFor(() =>
      expect(languagesSent(fetching)[1]).toEqual({
        name: GO,
        enabled: true,
        size: "20G",
      }),
    );
  });

  /// And on closing the panel by Escape, which takes the field out of the page
  /// with the focus still in it — no blur — and is leaving it all the same.
  it("saves a size when the panel is closed by Escape", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(compiling(TOLD))),
    );
    mountPane();

    const panel = await opened("Go");
    fireEvent.input(theSize(GO), { target: { value: "20G" } });
    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() => expect(panel.isConnected).toBe(false));
    await waitFor(() =>
      expect(languagesSent(fetching)[1]).toEqual({
        name: GO,
        enabled: true,
        size: "20G",
      }),
    );
    await settled();
    expect(saves(fetching)).toBe(1);
  });

  /// But not on leaving one nobody typed in, nor one put back to what it was
  /// showing: every close of the panel is a blur, and a close is not a save.
  it("does not save on leaving a field nothing was changed in", async () => {
    const fetching = theSettings(compiling(TOLD));
    mountPane();

    await opened();
    fireEvent.focus(theSize());
    fireEvent.blur(theSize());

    fireEvent.input(theSize(), { target: { value: "5" } });
    fireEvent.input(theSize(), { target: { value: "50G" } });
    fireEvent.blur(theSize());

    fireEvent.keyDown(document, { key: "Escape" });

    await settled();
    expect(saves(fetching)).toBe(0);
  });

  /// Clearing the field asks for the default back, which is what an empty size
  /// means to the server — and the placeholder is what says so.
  it("sends an empty size for a field the human cleared", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(compiling(UNSET))),
    );
    mountPane();

    await opened();
    fireEvent.input(theSize(), { target: { value: "" } });
    fireEvent.blur(theSize());

    await waitFor(() => expect(languagesSent(fetching)[0]?.size).toBe(""));
  });

  /// And a size the pane drew no field for is sent all the same.
  ///
  /// One request writes the whole of `config.yaml`, and `size` is a key of every
  /// language's entry — C/C++'s too, which has no store and no field for it.
  /// A tick that sent what it had drawn would write the file with that size
  /// gone, which is a key nobody was ever shown being emptied by a press about
  /// something else.
  it("sends the size of a language it drew no field for", async () => {
    const fetching = theSettings(compiling(TOLD), json(answering(off(TOLD))));
    mountPane();

    await waitFor(() => expect(theCheck().checked).toBe(true));
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
  /// reason drawn under the field still holding what was typed — in a panel
  /// left open, and not sent again by leaving the field.
  it("draws a refused size's reason and keeps what was typed", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json({
        ...answering(compiling(TOLD)),
        refused_sizes: [{ language: GO, why: "is not a size: write 10G" }],
      }),
    );
    mountPane();

    const panel = await opened("Go");
    fireEvent.input(theSize(GO), { target: { value: "1.5G" } });
    fireEvent.submit(theSize(GO).form!);

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
    expect(panel.isConnected).toBe(true);

    await settled();
    fireEvent.blur(theSize(GO));
    await settled();
    expect(saves(fetching), "the same refused size is not sent twice").toBe(1);

    // And it is still there when the panel is opened again.
    fireEvent.keyDown(document, { key: "Escape" });
    await opened("Go");
    expect(screen.getByText(/is not a size/)).toBeTruthy();
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
