//! The Sandbox on the settings page: what its card says of the build caches and
//! the binds together, and the pane it opens holding the two as parts.
//!
//! One section where there were two — Language support and the Sandbox binds —
//! because they answer one question: what the sandbox a session starts in holds
//! beyond its own worktree. What each part draws and saves is its own file's:
//! `languages.test.tsx` for the build caches and `sandbox-binds.test.tsx` for
//! the binds, each mounting its part alone. What is asked here is what only the
//! section as a whole does:
//!
//! - **The card's one line.** Whatever is waiting on the human in either part
//!   comes first — an entry `config.yaml` holds that could not be read, compiles
//!   that nothing caches, a bind the server cannot see — each in the fewest
//!   words that name it, joined on the one line and in the warning colour. Where
//!   nothing is, the line counts: how many build caches are on and how many
//!   binds there are, in the quiet colour, in English rather than as `1 binds`.
//! - **The pane.** One head, titled for the section, with the way back; under it
//!   the two parts, each under a heading of its own.
//! - **The round trip.** What either part saves is what the card goes back to
//!   saying, because the answer is a fresh read of the files both are drawn
//!   from.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with. Its build caches have no
//! sccache to compile through — the routers those are written from run no
//! sessions — and its binds name directories that are not on whatever wrote it,
//! so every warning the card can say is in it as it stands; the helpers below
//! take each away.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import rowStyles from "../src/settings/PathEditor.module.css";
import { SandboxCard, SandboxPane } from "../src/settings/Sandbox";
import styles from "../src/settings/Sandbox.module.css";
import line from "../src/settings/SettingsCard.module.css";
import head from "../src/workbench/PaneHead.module.css";
import { type Answer, json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The descriptors the fixture holds, by the names `config.yaml` keys them
/// under. Gleam is the one an installer wrote; the rest Verkstead ships.
const RUST = "rust";
const GO = "go";
const NODE = "node";
const PYTHON = "python";
const DOTNET = "dotnet";
const CPP = "cpp";
const JVM = "jvm";
const GLEAM = "gleam";

/// The same settings with an sccache the server did find, which no fixture
/// carries — so the caches have nothing to warn about.
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

/// And with every language but the one named switched off.
function only(standing: SettingsView, name: string): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) => ({
      ...language,
      enabled: language.name === name,
    })),
  };
}

/// And the same settings with one language's entry in `config.yaml` one the
/// server could not read, with nothing shipped of that name to fall back to.
function unread(standing: SettingsView, name: string): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) =>
      language.name === name
        ? {
            ...language,
            enabled: false,
            unread: {
              why: "sets RUSTUP_HOME, which is a variable the Sandbox sets itself",
              running_on: "Nothing" as const,
            },
          }
        : language,
    ),
  };
}

/// And the same settings with every bind resolving, which is the ordinary
/// machine: the fixture's own name directories that are not on whatever wrote
/// it.
function seen(standing: SettingsView): SettingsView {
  return {
    ...standing,
    paths: {
      binds: standing.paths.binds.map((entry) => ({
        ...entry,
        resolution: "Resolves" as const,
      })),
    },
  };
}

/// And with only the first of the binds left, as it stood.
function firstBind(standing: SettingsView): SettingsView {
  return {
    ...standing,
    paths: { binds: standing.paths.binds.slice(0, 1) },
  };
}

/// Every warning the card can say taken away: an sccache found and every bind
/// resolving. Which the fixture's entries all read, so nothing else needs it.
function quiet(standing: SettingsView): SettingsView {
  return compiling(seen(standing));
}

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Whatever of the section a test is about, over one query client: the card and
/// both parts read the same two files, so a test mounting them together is
/// reading them once, exactly as the page does.
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
    ...mounting(() => <SandboxCard open={open} press={press} />),
    press,
  };
}

/// The pane it opens, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <SandboxPane back={back} />), back };
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

/// The card itself, once it is drawn — waited for, because it stands on a read.
async function theCard(container: ParentNode): Promise<HTMLElement> {
  return await waitFor(() => {
    const face = container.querySelector<HTMLElement>(`.${styles.sandboxCard}`);
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

/// The status on a drawn card — the one line at its far end saying how the
/// section stands — or `null` where there is no card yet.
function statusOf(container: ParentNode): HTMLElement | null {
  return container.querySelector<HTMLElement>(
    `.${styles.sandboxCard} .${line.status}`,
  );
}

/// The card's line once it reads `said`, and the line itself.
async function reads(
  container: ParentNode,
  said: string,
): Promise<HTMLElement> {
  await waitFor(() => expect(statusOf(container)?.textContent).toBe(said));
  return statusOf(container)!;
}

describe("the card", () => {
  /// What somebody scanning the page is after where nothing is waiting on
  /// them: what the section is, and how much is in it — how many languages a
  /// session builds with a cache, and how many extra paths its sandbox gets —
  /// in the quiet colour.
  it("counts the build caches on and the binds", async () => {
    theSettings(quiet(TOLD));
    const { container } = mountCard();

    const face = await theCard(container);
    expect(face.querySelector("h2")?.textContent).toBe("Sandbox");

    const status = await reads(container, "8 build caches · 2 binds");
    expect(status.classList).not.toContain(line.warn);
  });

  /// A language switched off is a cache the count no longer holds — the one an
  /// installer wrote the same as one Verkstead ships, because there is nothing
  /// about either that the card knows.
  it("leaves a language that is switched off out of the count", async () => {
    theSettings(quiet(off(TOLD, GLEAM)));
    const { container } = mountCard();

    await reads(container, "7 build caches · 2 binds");
  });

  /// And where none are on and nobody has added a bind, the line still counts
  /// rather than going blank: a card is one line, title and status, and a
  /// status missing would read as one still loading. A machine nobody has
  /// touched is the ordinary state rather than one half set up, so it is quiet.
  it("counts nothing where nothing is on", async () => {
    theSettings(
      quiet({
        ...off(
          off(
            off(off(off(off(off(off(TOLD), GO), NODE), PYTHON), DOTNET), CPP),
            JVM,
          ),
          GLEAM,
        ),
        paths: { binds: [] },
      }),
    );
    const { container } = mountCard();

    const status = await reads(container, "0 build caches · 0 binds");
    expect(status.classList).not.toContain(line.warn);
  });

  /// One of each in the singular, so the line reads as English.
  it("says one build cache and one bind in the singular", async () => {
    theSettings(quiet(firstBind(only(TOLD, GO))));
    const { container } = mountCard();

    await reads(container, "1 build cache · 1 bind");
  });

  /// What the human cannot fix from the browser, said where they would
  /// otherwise wonder why nothing got faster — and on the card rather than only
  /// in the pane, because whoever needs to read it is whoever is not editing.
  /// In the fewest words that name it, in the warning colour, and taking the
  /// line the counts would have had: the pane says why.
  it("warns when there is no sccache for the server to compile through", async () => {
    theSettings(seen(TOLD));
    const { container } = mountCard();

    const status = await reads(container, "No sccache");
    expect(status.classList).toContain(line.warn);
  });

  /// And nothing about it while the languages that want one are switched off,
  /// because a warning about a language's compiles is only true while there is
  /// a language compiling. Both of them: C/C++ compiles through the same
  /// Compile Server, so it is uncached too.
  it("says nothing about sccache while the languages that want one are off", async () => {
    theSettings(off(off(UNSET), CPP));
    const { container } = mountCard();

    const status = await reads(container, "5 build caches · 0 binds");
    expect(status.classList).not.toContain(line.warn);
  });

  /// And a language whose entry the server could not read, said here too: a
  /// file that wants editing is exactly the sort of thing somebody scanning the
  /// page would want telling about. The card says only that there is one;
  /// which language and what the file says is the pane's to tell.
  it("says when a language's entry could not be read", async () => {
    theSettings(unread(quiet(TOLD), GLEAM));
    const { container } = mountCard();

    const status = await reads(container, "Unreadable config entry");
    expect(status.classList).toContain(line.warn);
  });

  /// And the thing the browser can see and the human cannot about a bind: an
  /// entry that is saved, is in the file, and does nothing. Counted, in the
  /// warning colour, which is what sends the human to the pane to read why.
  it("counts every bind the server cannot see", async () => {
    theSettings(compiling(TOLD));
    const { container } = mountCard();

    const status = await reads(container, "2 binds unseen");
    expect(status.classList).toContain(line.warn);
  });

  it("says one bind unseen in the singular", async () => {
    theSettings(compiling(firstBind(TOLD)));
    const { container } = mountCard();

    await reads(container, "1 bind unseen");
  });

  /// And every warning at once where every one is so, one after the other on
  /// the one line — the caches' first, as the pane draws them first — because
  /// none is the less waiting on the human for the others.
  it("says every warning where they all hold", async () => {
    theSettings(unread(TOLD, GLEAM));
    const { container } = mountCard();

    const status = await reads(
      container,
      "Unreadable config entry · No sccache · 2 binds unseen",
    );
    expect(status.classList).toContain(line.warn);
  });

  /// And a warning from each part beside the other, with neither's counts.
  it("says a cache warning and a bind warning together", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    const status = await reads(container, "No sccache · 2 binds unseen");
    expect(status.classList).toContain(line.warn);
  });

  it("opens the pane when it is pressed", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    fireEvent.click(await theCard(container));

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

    await waitFor(() => screen.getByText(/The settings couldn't be read/));
  });
});

describe("the pane", () => {
  /// One head for the section, titled as its card is, and under it the two
  /// parts in the order the card's line reads them — each under a heading of
  /// its own, which is how somebody tells where the caches stop and the binds
  /// begin.
  it("heads the pane Sandbox, over the build caches and then the binds", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    await waitFor(() => screen.getByRole("checkbox", { name: "Rust" }));
    await waitFor(() => screen.getByLabelText("Add a path"));

    expect(
      [...container.querySelectorAll("h1")].map((title) => title.textContent),
    ).toEqual(["Sandbox"]);
    expect(
      [...container.querySelectorAll("h3")].map((part) => part.textContent),
    ).toEqual(["Build Caches", "Sandbox Binds"]);
  });

  /// The way back is to the settings, which is the pane this one was entered
  /// from.
  it("goes back to the settings", async () => {
    theSettings(TOLD);
    const { container, back } = mountPane();

    const way = await waitFor(() => {
      const found = container.querySelector<HTMLButtonElement>(`.${head.back}`);
      expect(found, "expected the way back to be drawn").not.toBeNull();
      return found!;
    });
    expect(way.textContent).toContain("Settings");

    fireEvent.click(way);
    expect(back).toHaveBeenCalled();
  });
});

describe("the round trip", () => {
  /// What the caches part saved is what the card goes back to saying, because
  /// the answer is a fresh read of the files both are drawn from. With every
  /// warning taken away, so the line is the counts rather than a warning that
  /// would take it.
  it("takes a build cache off the card's count when the pane unticks it", async () => {
    theSettings(quiet(TOLD), json(answering(off(quiet(TOLD)))));
    const { container } = mounting(() => (
      <>
        <SandboxCard open press={() => {}} />
        <SandboxPane back={() => {}} />
      </>
    ));

    const rust = await waitFor(() => {
      const box = screen.getByRole("checkbox", {
        name: "Rust",
      }) as HTMLInputElement;
      expect(box.checked).toBe(true);
      return box;
    });
    await reads(container, "8 build caches · 2 binds");

    fireEvent.click(rust);

    await reads(container, "7 build caches · 2 binds");
  });

  /// And the binds part the same way: a bind removed is one the card no longer
  /// counts.
  it("takes a bind off the card's count when the pane removes it", async () => {
    theSettings(quiet(TOLD), json(answering(firstBind(quiet(TOLD)))));
    const { container } = mounting(() => (
      <>
        <SandboxCard open press={() => {}} />
        <SandboxPane back={() => {}} />
      </>
    ));

    await reads(container, "8 build caches · 2 binds");

    const remove = await waitFor(() => {
      const found = container.querySelectorAll<HTMLButtonElement>(
        `.${rowStyles.row} .${rowStyles.remove}`,
      );
      expect(found).toHaveLength(2);
      return found[1]!;
    });
    fireEvent.click(remove);

    await reads(container, "8 build caches · 1 bind");
  });
});
