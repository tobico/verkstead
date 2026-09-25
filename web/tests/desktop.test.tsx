//! The desktop app's own settings, on the settings page: what the card says of
//! the window and the tray, what the controls in its pane ask of the app, and
//! the section not being there at all in a browser.
//!
//! **The one section on this page with no server behind it.** Everything else
//! here reads `/api/ui/settings`; this reads `window.verkstead`, which the
//! Electron app's preload puts there and nothing else does. So what stands in
//! for the server in these tests is a stub bridge beside the stubbed fetch —
//! the settings it holds, the platform it claims, and a record of what was asked
//! of it — and the case with no bridge at all is a case like any other, because
//! that is what every browser and every phone is.
//!
//! Four things the stub buys that a real bridge could not be asked for here.
//! **A platform**, so the Mac arm — where the close radio is not drawn at all
//! and the icon is the menu bar's — is an ordinary test on Linux, as
//! `desktop/tests/closing.test.ts` makes the same arm one on the other side.
//! **A refusal**, which is a set answering with the settings unchanged: the
//! rule the page's controls hold is that what moves one is the answer coming
//! back rather than the press, and a refusal is the only thing that tells the
//! two apart. **The log file**, which the app opens and no page could check was
//! opened. And **the tray turned off**, which is what greys the keep-running
//! position and makes the choice fall to Quit.
//!
//! The shape the stub is built to is `src/settings/bridge.ts`'s, which is
//! `desktop/src/bridge.ts` said again on this side — the two packages are built
//! apart, so what keeps them in step is this suite and
//! `desktop/tests/bridge.test.ts` pinning the same literals.
//!
//! The page mounted whole is here for the two things only the page can answer:
//! that the card stands above the git card and opens at the path the word list
//! gives it, and that a browser's settings page is exactly what it was. Which
//! path a word reaches at all is `settings-routes.test.tsx`'s, and the new word
//! is a case there without a line of it being edited.

import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ConversationEntry,
  ProfileEntry,
  RepoEntry,
  SettingsView,
  ShowingArchived,
} from "../src/api/types";
import check from "../src/Check.module.css";
import { DesktopCard, DesktopPane } from "../src/settings/Desktop";
import styles from "../src/settings/Desktop.module.css";
import git from "../src/settings/Git.module.css";
import {
  NAME,
  POSITIONS,
  type Bridge,
  type DesktopSettings,
} from "../src/settings/bridge";
import {
  SettingsPage,
  panes as settingsPanes,
} from "../src/settings/SettingsPage";
import { pathTo } from "../src/settings/openings";
import { json, serving, whenever } from "./serving";
import conversations from "./fixtures/conversations.json" with { type: "json" };
import profiles from "./fixtures/profiles.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };

const TOLD = told as SettingsView;
const PROFILES = profiles as ProfileEntry[];
const REPOS = repos as RepoEntry[];
const SIDEBAR = conversations as ConversationEntry[];
const HIDING_ARCHIVED: ShowingArchived = { showing: false, any: false };

/// The app every first run gets, which is what `DEFAULTS` in
/// `desktop/src/settings.ts` holds: it keeps running in the tray, and there is a
/// tray to keep running in.
const DEFAULTS: DesktopSettings = { whenClosed: "tray", trayIcon: true };

afterEach(() => {
  vi.unstubAllGlobals();
});

/// What was asked of the app, which is what a test about a control reads.
type Asked = {
  settings: ReturnType<typeof vi.fn>;
  set: ReturnType<typeof vi.fn>;
  logs: ReturnType<typeof vi.fn>;
};

/// The app's own window, stood in for.
///
/// Put on the global the way the preload puts it on the window, because that is
/// where the page looks — see `bridge` in `src/settings/bridge.ts`. Every test
/// that wants no bridge simply does not call this, and `unstubAllGlobals` takes
/// it away after each one.
///
/// A set answers with what `enact` on the other side answers with: the settings
/// in force afterwards. `refuse` is the app saying no — the settings unchanged,
/// which is what a set of the wrong shape comes back as.
function standingIn(
  held: DesktopSettings = DEFAULTS,
  platform = "linux",
  refuse = false,
): Asked {
  let stands = held;

  const asked: Asked = {
    settings: vi.fn(() => Promise.resolve(stands)),
    set: vi.fn((changed: Partial<DesktopSettings>) => {
      if (!refuse) {
        stands = { ...stands, ...changed };
      }
      return Promise.resolve(stands);
    }),
    logs: vi.fn(() => Promise.resolve()),
  };

  const bridge: Bridge = {
    platform,
    settings: () => asked.settings() as Promise<DesktopSettings>,
    set: (changed) => asked.set(changed) as Promise<DesktopSettings>,
    logs: () => asked.logs() as Promise<void>,
  };

  vi.stubGlobal(NAME, bridge);

  return asked;
}

/// Whichever half of the section a test is about, over one query client: both
/// halves draw the one reading, exactly as the page has them.
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
    ...mounting(() => <DesktopCard open={open} press={press} />),
    press,
  };
}

/// The controls in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <DesktopPane back={back} />), back };
}

/// The three positions of the radio, in the order they are drawn.
function positions(container: ParentNode): HTMLInputElement[] {
  return [
    ...container.querySelectorAll<HTMLInputElement>(
      'input[name="desktop-when-closed"]',
    ),
  ];
}

/// And which of them the group is showing as chosen.
function chosen(container: ParentNode): string | undefined {
  return positions(container).find((box) => box.checked)?.value;
}

describe("the card", () => {
  it("says what closing the window does, and where the icon is", async () => {
    standingIn();
    mountCard();

    await screen.findByText(
      "Closing the window keeps Verkstead running in the tray.",
    );
    screen.getByText("There is an icon in the tray.");
  });

  /// Each position in its own words, which is what the card answers: the human
  /// came to it to find out what closing the window does.
  it("says of each position what it comes to", async () => {
    for (const [whenClosed, said] of [
      ["ask", "Closing the window asks before quitting."],
      ["quit", "Closing the window quits Verkstead."],
    ] as const) {
      standingIn({ whenClosed, trayIcon: true });
      const { unmount } = mountCard();

      await screen.findByText(said);
      unmount();
      vi.unstubAllGlobals();
    }
  });

  /// With no icon in the tray the close is a quit whatever the radio says — the
  /// reading `closing` makes in `desktop/src/closing.ts`. The card says what
  /// closing the window *does*, so it says that rather than the position.
  it("says the close quits where there is no icon to come back from", async () => {
    standingIn({ whenClosed: "tray", trayIcon: false });
    mountCard();

    await screen.findByText("Closing the window quits Verkstead.");
    screen.getByText("There is no icon in the tray.");
  });

  /// A Mac has no close policy of this page's to report: closing a window there
  /// leaves the application in the Dock, and the icon is the menu bar's.
  it("says nothing of closing on a Mac, and names the menu bar", async () => {
    standingIn(DEFAULTS, "darwin");
    mountCard();

    await screen.findByText("There is an icon in the menu bar.");
    expect(screen.queryByText(/Closing the window/)).toBeNull();
  });

  it("opens the pane when it is pressed", async () => {
    standingIn();
    const { press } = mountCard();

    fireEvent.click(await screen.findByText("Desktop"));
    expect(press).toHaveBeenCalled();
  });

  /// And the whole of what a browser sees of it. Nothing is drawn, nothing is
  /// asked of anything, and no line says a bridge was missing: a page with no
  /// bridge is not the app, which is a fact about where it is being read rather
  /// than something that failed.
  it("draws nothing at all without a bridge", () => {
    const { container } = mountCard();

    expect(container.textContent).toBe("");
  });
});

describe("the close policy", () => {
  it("draws the three positions, with the chosen one showing", async () => {
    standingIn({ whenClosed: "ask", trayIcon: true });
    const { container } = mountPane();

    await waitFor(() => expect(positions(container)).toHaveLength(3));
    expect(positions(container).map((box) => box.value)).toEqual([...POSITIONS]);
    expect(chosen(container)).toBe("ask");
  });

  /// Its own press, as every control on this page is: there is no Save over the
  /// pane and nothing to press twice.
  it("sends the position that was picked, and moves to what came back", async () => {
    const asked = standingIn();
    const { container } = mountPane();

    fireEvent.click(await screen.findByLabelText("Quit Verkstead"));

    await waitFor(() => expect(asked.set).toHaveBeenCalledWith({ whenClosed: "quit" }));
    await waitFor(() => expect(chosen(container)).toBe("quit"));
  });

  /// The rule the page's checkbox holds, and the reason a press is handed up as
  /// the state it would become: a set the app refuses answers with the settings
  /// unchanged, and the group goes back where the reading has it rather than
  /// staying where the browser put it.
  it("leaves the group where the reading has it when the set is refused", async () => {
    standingIn(DEFAULTS, "linux", true);
    const { container } = mountPane();

    fireEvent.click(await screen.findByLabelText("Quit Verkstead"));

    await waitFor(() => expect(chosen(container)).toBe("tray"));
  });

  /// With the tray off there is no way back to a hidden window, so the position
  /// that hides it is greyed and a note says why. Greyed by the browser's own
  /// disabling — which takes it out of the tab order too — rather than taken
  /// away: a position that vanished would say the setting had.
  it("greys the keep-running position with the tray off, and says why", async () => {
    standingIn({ whenClosed: "quit", trayIcon: false });
    const { container } = mountPane();

    await waitFor(() => expect(positions(container)).toHaveLength(3));

    const [tray, ask, quit] = positions(container);
    expect(tray!.disabled).toBe(true);
    expect(ask!.disabled).toBe(false);
    expect(quit!.disabled).toBe(false);

    screen.getByText(/no way back to a hidden window/);
  });

  /// And the note is gone with the icon back, which is what says it is about the
  /// tray rather than about the position.
  it("says nothing of the sort with the tray on", async () => {
    standingIn();
    const { container } = mountPane();

    await waitFor(() => expect(positions(container)).toHaveLength(3));

    expect(positions(container)[0]!.disabled).toBe(false);
    expect(container.querySelector(`.${styles.fallen}`)).toBeNull();
  });

  /// Not drawn on a Mac at all. The radio is the one control here that is any
  /// platform's business: closing a window on a Mac leaves the application in
  /// the Dock, which is the platform's answer rather than a position anybody
  /// picked.
  it("is not drawn on a Mac", async () => {
    standingIn(DEFAULTS, "darwin");
    const { container } = mountPane();

    await screen.findByLabelText("Show menu bar icon");
    expect(positions(container)).toHaveLength(0);
    expect(screen.queryByText("When the window is closed")).toBeNull();
  });

  /// And drawn on Windows, which is the other platform the app is packaged for:
  /// the arm is a Mac's rather than a Linux-only one.
  it("is drawn on Windows", async () => {
    standingIn(DEFAULTS, "win32");
    const { container } = mountPane();

    await waitFor(() => expect(positions(container)).toHaveLength(3));
    screen.getByLabelText("Show tray icon");
  });
});

describe("the tray", () => {
  it("says where the switch stands, and sends a tick", async () => {
    const asked = standingIn();
    mountPane();

    const box = await screen.findByLabelText<HTMLInputElement>("Show tray icon");
    expect(box.checked).toBe(true);

    fireEvent.click(box);

    await waitFor(() => expect(asked.set).toHaveBeenCalledWith({ trayIcon: false }));
    await waitFor(() => expect(box.checked).toBe(false));
  });

  /// The page's checkbox rather than the sliding switch a pane head carries: the
  /// settings page is a form, and a form's answer to *is this on* is the box the
  /// browser draws for that question.
  it("is the page's own checkbox", async () => {
    standingIn();
    const { container } = mountPane();

    await screen.findByLabelText("Show tray icon");
    expect(container.querySelector(`.${check.check}`)).not.toBeNull();
  });

  /// Reads as the menu bar on a Mac, that being where the icon goes there.
  it("reads as the menu bar icon on a Mac", async () => {
    standingIn(DEFAULTS, "darwin");
    mountPane();

    await screen.findByLabelText("Show menu bar icon");
    expect(screen.queryByLabelText("Show tray icon")).toBeNull();
  });
});

describe("View Logs", () => {
  /// The tray item's own act, reached over the bridge: one action reached two
  /// ways rather than two that agree.
  it("asks the app to open this run's log file", async () => {
    const asked = standingIn();
    mountPane();

    fireEvent.click(await screen.findByText("View Logs"));

    await waitFor(() => expect(asked.logs).toHaveBeenCalled());
  });

  /// And with the tray off as readily as with it on (ADR-0020). A switch
  /// somebody can turn off cannot be the only way to a log file, and the desktop
  /// most likely to have the tray off is the one the tray misbehaved on — which
  /// is the machine whose log is worth reading.
  it("is drawn and pressed with the tray off", async () => {
    const asked = standingIn({ whenClosed: "quit", trayIcon: false });
    mountPane();

    fireEvent.click(await screen.findByText("View Logs"));

    await waitFor(() => expect(asked.logs).toHaveBeenCalled());
  });

  /// And on a Mac, where the pane draws no radio at all: the button is drawn
  /// whatever the tray setting says and on every platform.
  it("is drawn on a Mac", async () => {
    const asked = standingIn(DEFAULTS, "darwin");
    mountPane();

    fireEvent.click(await screen.findByText("View Logs"));

    await waitFor(() => expect(asked.logs).toHaveBeenCalled());
  });

  it("draws nothing without a bridge, the pane included", () => {
    const { container } = mountPane();

    expect(container.textContent).toBe("");
  });
});

/// The page whole, which is what says where the section stands and what a
/// browser's settings are without it.
function thePage(at = "/settings") {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  serving(
    whenever("/api/ui/settings", json(TOLD)),
    whenever("/api/ui/profiles", json(PROFILES)),
    whenever("/api/ui/repos", json(REPOS)),
    whenever("/api/ui/update", json("Current")),
    whenever("/api/ui/conversations", json(SIDEBAR)),
    whenever("/api/ui/conversations/archived", json(HIDING_ARCHIVED)),
    whenever("/api/ui/abandoned-roadmaps", json([])),
  );

  const history = createMemoryHistory();
  history.set({ value: at });

  return {
    ...render(() => (
      <QueryClientProvider client={queries}>
        <MemoryRouter history={history}>
          {/* Nested out of the page's own `panes()`, exactly as `App.tsx` nests
              them: a list spelled out here would be a second opinion about
              where a card leads. */}
          <Route path="/settings" component={SettingsPage}>
            {settingsPanes()}
          </Route>
          <Route path="*" component={() => <p>somewhere else</p>} />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
  };
}

describe("where the section stands", () => {
  /// At the top of the settings: it is about the window in front of the human
  /// rather than about anything Verkstead was told, which is the first thing
  /// somebody who has just installed the app is looking for.
  it("draws the card above the git card", async () => {
    standingIn();
    const { container } = thePage();

    const desktop = await waitFor(() => {
      const found = container.querySelector(`.${styles.desktopCard}`);
      if (!found) {
        throw new Error("the Desktop card has not been drawn");
      }
      return found;
    });
    const credentials = await waitFor(() => {
      const found = container.querySelector(`.${git.gitCard}`);
      if (!found) {
        throw new Error("the git card has not been drawn");
      }
      return found;
    });

    expect(
      desktop.compareDocumentPosition(credentials) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// And the pane opens at the path the word list gives it, replacing the way
  /// every card on this page does.
  it("opens the pane at /settings/desktop, replacing", async () => {
    standingIn();
    const { container, history } = thePage();

    const desktop = await waitFor(() => {
      const found = container.querySelector<HTMLElement>(
        `.${styles.desktopCard}`,
      );
      if (!found) {
        throw new Error("the Desktop card has not been drawn");
      }
      return found;
    });

    fireEvent.click(desktop);

    await waitFor(() => expect(history.get()).toBe(pathTo("desktop")));
    await screen.findByLabelText("Show tray icon");
  });

  /// And a browser's settings page is what it was: no card, nothing added to the
  /// order of the middle pane, and the git card still the first thing under the
  /// head. Which is also every phone on the tailnet — there is nothing about
  /// this drawn either way, so there is no width at which it appears.
  it("draws no card at all in a browser", async () => {
    const { container } = thePage();

    await waitFor(() => {
      if (!container.querySelector(`.${git.gitCard}`)) {
        throw new Error("the git card has not been drawn");
      }
    });

    expect(container.querySelector(`.${styles.desktopCard}`)).toBeNull();
    expect(screen.queryByText("Desktop")).toBeNull();
  });

  /// And a link somebody kept to the pane leaves the details bare rather than
  /// walking a narrow window into a pane with no way back out of it. The URL
  /// stands: it is a record of what was picked rather than a promise that it is
  /// still there.
  it("leaves the details bare at /settings/desktop in a browser", async () => {
    const { container } = thePage(pathTo("desktop"));

    await waitFor(() => {
      if (!container.querySelector(`.${git.gitCard}`)) {
        throw new Error("the git card has not been drawn");
      }
    });

    const frame = container.querySelector(`[data-pane]`);
    expect(frame!.getAttribute("data-pane")).toBe("middle");
    expect(screen.queryByLabelText("Show tray icon")).toBeNull();
  });

  /// And with the bridge it opens on the details, which is what a cold load of
  /// a details pane does on this page — the narrow window's walk included.
  it("opens on the details at /settings/desktop inside the app", async () => {
    standingIn();
    const { container } = thePage(pathTo("desktop"));

    await screen.findByLabelText("Show tray icon");
    expect(
      container.querySelector(`[data-pane]`)!.getAttribute("data-pane"),
    ).toBe("details");
  });
});

describe("the name on the window", () => {
  /// The whole section hangs off one string, and the two packages are built
  /// apart: this is its other half — `desktop/tests/bridge.test.ts` pins the
  /// same literal on the side that puts it there.
  it("is the product's own", () => {
    expect(NAME).toBe("verkstead");
  });
});
