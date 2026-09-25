//! The desktop app's own settings, at the top of the settings page: what
//! closing the window means, whether there is an icon in the tray, and the way
//! to this run's log file.
//!
//! **The one section on this page that is not the server's** (ADR-0020, Set 847
//! Q12). Everything else here is read from `/api/ui/settings` and written back
//! to it; this is read from the window the page is drawn in and written back to
//! a JSON file of Electron's own — see [`bridge`](./bridge.ts). Which is what it
//! has to be: whether this machine's Verkstead keeps running when its window is
//! closed is a fact about the desk in front of the human, and the same account
//! read from a laptop and from a desktop wants two different answers.
//!
//! **So it is drawn only inside the app.** With no bridge on the window both
//! halves draw nothing — no card, no pane, and nothing about the order of the
//! middle pane changed — which is how a phone on the tailnet never sees the
//! section at all, and how a browser on this very machine does not either.
//!
//! **The close policy is one radio of three positions**, keep running in the
//! tray by default, rather than the two mutually exclusive checkboxes the brief
//! asked for: three answers to one question are a radio. And **with the tray off
//! the keep-running position is greyed**, with a note saying why — an app with no
//! icon that has hidden its only window is a Verkstead nobody can reach, so the
//! choice falls to Quit while there is no icon to come back from. Greyed rather
//! than taken away, because a position that vanished would say the setting had,
//! and it has not. The browser's own `disabled` is what greys it and takes it out
//! of the tab order, which is the page's pattern for configuration that only
//! means something while something else is on — see [`Nested`](../Check.tsx).
//!
//! **View Logs is drawn whatever the tray setting says, and on every platform**
//! (ADR-0020). A switch somebody can turn off cannot be the only way to a log
//! file, and the desktop most likely to have the tray off is the one the tray
//! misbehaved on — which is the machine whose log is worth reading. It is the
//! tray item's own act reached over the bridge, so the two are one action reached
//! two ways rather than two that agree.
//!
//! **A Mac is the platform's own.** The radio is not drawn there at all, because
//! closing a window on a Mac leaves the application running in the Dock and Cmd+Q
//! is what quits — see `closing` in `desktop/src/closing.ts`, which is what
//! enacts the same reading. **Show tray icon** reads as the menu bar there, that
//! being where the icon goes.
//!
//! **The page's own conventions hold.** The settings page is a form, so the tray
//! is the page's checkbox rather than the sliding switch that sits on a pane
//! head; and a control shows where things stand rather than where somebody
//! pressed, so a press is handed up as the state it would become and what moves
//! the control is the answer coming back. A set the app refuses answers with the
//! settings unchanged, and the radio goes back where it was.
//!
//! **Launch on Startup is the registration and nothing else** (Set 846 Q9a). It
//! is read from the platform through the bridge rather than out of the app's
//! settings file, so a human who turns it off with their desktop's own settings
//! has unchecked this box; and where there is nowhere to keep a registration —
//! an unpackaged run, or a machine that names no configuration directory — the
//! box is greyed with the reason under it, which is the same shape the greyed
//! keep-running position has. It is drawn on every platform: what differs
//! between them is the registration behind it rather than anything here.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch as Choose, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { Check } from "../Check";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import {
  MAC,
  POSITIONS,
  bridge,
  type Bridge,
  type DesktopSettings,
  type Registration,
  type WhenClosed,
} from "./bridge";
import styles from "./Desktop.module.css";

/// What the settings the bridge answers with are held under.
///
/// A key of the query client's like every other section's, because the card and
/// the pane are mounted apart and both draw the one reading — and because a set
/// answers with the settings in force afterwards, which is written straight over
/// it.
const KEY = "desktop";

/// And what the startup registration is held under, which is a reading of its
/// own: it is the platform's rather than the settings file's, and only the pane
/// draws it.
const STARTUP = [KEY, "startup"];

/// Each position of the radio, in its own words.
const WORDS_FOR: Record<WhenClosed, string> = {
  tray: "Keep running in the tray",
  ask: "Ask before quitting",
  quit: "Quit Verkstead",
};

/// Why the keep-running position will not take a press while the tray is off.
const FALLEN =
  "With no icon in the tray there is no way back to a hidden window, so " +
  "closing the window quits while the icon is off.";

/// The settings of the machine the app is running on, out of the bridge.
///
/// Static, and it is the one reading on this page that really is: a Nudge is
/// something the server sent, and nothing the server has to say could change
/// what is in a JSON file beside this window's remembered bounds. What moves it
/// is a set answering, which writes over this by hand.
function useDesktop(reach: Bridge) {
  return useReading(() => ({
    queryKey: [KEY],
    queryFn: () => reach.settings(),
    freshness: "static",
  }));
}

/// And how **Launch on Startup** stands, out of the platform's own registration.
///
/// Static for [`useDesktop`]'s reason and one of its own: nothing the server has
/// to say could change a registration with the desktop session, and what moves
/// this is a tick answering. A desktop's own settings can change it under the
/// page — which is what a pane opened afresh reads, the registration being asked
/// of the platform every time rather than remembered anywhere.
function useStartup(reach: Bridge) {
  return useReading(() => ({
    queryKey: STARTUP,
    queryFn: () => reach.startup(),
    freshness: "static",
  }));
}

/// How the window and the tray stand, as the card that opens the section — or
/// nothing at all, where this page is not inside the app.
export function DesktopCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const reach = bridge();

  return reach === null ? null : (
    <Card reach={reach} open={props.open} press={props.press} />
  );
}

/// And the controls that change it, which is the details pane the card opens —
/// nothing, again, without a bridge to read and write over.
export function DesktopPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const reach = bridge();

  return reach === null ? null : <Pane reach={reach} back={props.back} />;
}

/// The card, given the bridge there turned out to be.
function Card(props: {
  reach: Bridge;
  open: boolean;
  press: () => void;
}): JSX.Element {
  const desktop = useDesktop(props.reach);
  const mac = () => props.reach.platform === MAC;

  /// What a close comes to, which is not always what the radio says: with the
  /// tray off it is a quit whatever was chosen, and on a Mac it is the Dock's
  /// business and no line of this card's. The same reading `closing` makes in
  /// `desktop/src/closing.ts`, said in words.
  const closes = (stands: DesktopSettings): string => {
    if (!stands.trayIcon) {
      return "Closing the window quits Verkstead.";
    }

    switch (stands.whenClosed) {
      case "tray":
        return "Closing the window keeps Verkstead running in the tray.";
      case "ask":
        return "Closing the window asks before quitting.";
      case "quit":
        return "Closing the window quits Verkstead.";
    }
  };

  /// And where the icon stands, which is the menu bar on a Mac and the tray
  /// everywhere else.
  const icon = (stands: DesktopSettings): string => {
    const where = mac() ? "the menu bar" : "the tray";

    return stands.trayIcon
      ? `There is an icon in ${where}.`
      : `There is no icon in ${where}.`;
  };

  return (
    <Choose>
      <Match when={desktop.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={desktop.isError}>
        <ErrorLine>
          Could not read this machine's desktop settings:{" "}
          {desktop.error?.message}
        </ErrorLine>
      </Match>
      <Match when={desktop.data}>
        {(stands) => (
          <CardButton
            as="article"
            class={styles.desktopCard}
            open={props.open}
            press={props.press}
          >
            <h2>Desktop</h2>

            {/* Not on a Mac: there is no close policy there to report, the
                platform having its own. */}
            <Show when={!mac()}>
              <p class={styles.standing}>{closes(stands())}</p>
            </Show>
            <p class={styles.standing}>{icon(stands())}</p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the pane, likewise.
///
/// There is no Save over the whole of it and no Cancel: each control is its own
/// press, as the checkboxes on this page are, and a details pane is left by
/// opening something else or by the way back a narrow window draws.
function Pane(props: { reach: Bridge; back: () => void }): JSX.Element {
  const queries = useQueryClient();
  const desktop = useDesktop(props.reach);
  const starts = useStartup(props.reach);
  const mac = () => props.reach.platform === MAC;

  /// A control moved, which saves itself.
  ///
  /// What comes back is the settings in force afterwards — a refused set answers
  /// with them unchanged — so it is written straight over the reading both
  /// halves are drawn from: the app enacted the set in this run, and a second
  /// read would learn nothing it did not already say.
  const save = useMutation(() => ({
    mutationFn: (changed: Partial<DesktopSettings>) => props.reach.set(changed),
    onSuccess: (stands: DesktopSettings) => {
      queries.setQueryData([KEY], stands);
    },
  }));

  /// And the box that is the registration itself, which saves itself the same
  /// way — the answer being how the registration stands once the platform has
  /// been asked, so a registration it refused is a box that goes back where it
  /// was with the reason under it.
  const registering = useMutation(() => ({
    mutationFn: (on: boolean) => props.reach.register(on),
    onSuccess: (stands: Registration) => {
      queries.setQueryData(STARTUP, stands);
    },
  }));

  /// And **View Logs**, which changes nothing: the app opens the file, or puts
  /// its own dialog up to say there is none. A mutation for the one thing it
  /// buys — a line here where the bridge itself could not be reached.
  const viewing = useMutation(() => ({
    mutationFn: () => props.reach.logs(),
  }));

  /// Every position's box, so a press can put the group back where the reading
  /// has it.
  ///
  /// The rule the page's checkbox holds — see [`Check`](../Check.tsx) — and the
  /// radio needs the same help for the same reason: the browser moves the group
  /// on the press, and a set the app refuses answers with the position
  /// unchanged, so nothing would move it back.
  const boxes = new Map<WhenClosed, HTMLInputElement>();

  /// The group as the reading has it, whatever the browser has just done to it.
  const showing = (stands: DesktopSettings): void => {
    for (const [position, box] of boxes) {
      box.checked = position === stands.whenClosed;
    }
  };

  /// Whether a position is one the tray being off has taken away — which is the
  /// one that hides the window, there being nothing to bring it back.
  const fallen = (stands: DesktopSettings, position: WhenClosed): boolean =>
    position === "tray" && !stands.trayIcon;

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title="Desktop" />
      </PaneSticky>

      <Choose>
        <Match when={desktop.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={desktop.isError}>
          <ErrorLine>
            Could not read this machine's desktop settings:{" "}
            {desktop.error?.message}
          </ErrorLine>
        </Match>
        <Match when={desktop.data}>
          {(stands) => (
            <div class={styles.desktop}>
              {/* Not drawn on a Mac at all: closing a window there leaves the
                  application in the Dock, which is the platform's answer rather
                  than a position anybody picked. */}
              <Show when={!mac()}>
                <fieldset class={styles.whenClosed}>
                  <legend>When the window is closed</legend>

                  <For each={POSITIONS}>
                    {(position) => (
                      <div class={styles.position}>
                        <label>
                          <input
                            type="radio"
                            name="desktop-when-closed"
                            value={position}
                            ref={(box) => boxes.set(position, box)}
                            // Solid sets this as a property rather than an
                            // attribute, which is what a choice somebody keeps
                            // changing wants: the attribute says only what the
                            // group started as.
                            checked={stands().whenClosed === position}
                            disabled={
                              save.isPending || fallen(stands(), position)
                            }
                            // The group the browser has just moved, and then
                            // straight back where the reading has it — what
                            // moves it is the answer arriving.
                            onChange={() => {
                              showing(stands());
                              save.mutate({ whenClosed: position });
                            }}
                          />
                          <span>{WORDS_FOR[position]}</span>
                        </label>
                      </div>
                    )}
                  </For>

                  {/* Why the position above is greyed. Drawn under the group
                      rather than beside the box, because what it is about is
                      the choice rather than the one position: the tray is off,
                      so this is what closing the window does. */}
                  <Show when={!stands().trayIcon}>
                    <Note class={styles.fallen}>{FALLEN}</Note>
                  </Show>
                </fieldset>
              </Show>

              <Check
                label={mac() ? "Show menu bar icon" : "Show tray icon"}
                on={stands().trayIcon}
                disabled={save.isPending}
                flip={(on) => save.mutate({ trayIcon: on })}
              />

              {/* The registration itself, drawn once the platform has been asked
                  about it — and on every platform, what differs between them
                  being the registration behind the box rather than the box. */}
              <Show when={starts.data}>
                {(how) => (
                  <div class={styles.startup}>
                    <Check
                      label="Launch on Startup"
                      on={how().on}
                      disabled={registering.isPending || !how().possible}
                      title={how().why}
                      flip={(on) => registering.mutate(on)}
                    />

                    {/* Why it will not take a tick, under the box as the close
                        policy's note is under its group. */}
                    <Show when={how().why}>
                      {(why) => <Note class={styles.why}>{why()}</Note>}
                    </Show>

                    {/* And what the platform said about a registration it would
                        not make, which the box springing back is otherwise the
                        whole of. */}
                    <Show when={how().refused}>
                      {(refused) => (
                        <ErrorLine class={styles.failure}>{refused()}</ErrorLine>
                      )}
                    </Show>
                  </div>
                )}
              </Show>

              <Show when={starts.isError}>
                <ErrorLine class={styles.failure}>
                  Could not read whether Verkstead starts with this machine:{" "}
                  {starts.error?.message}
                </ErrorLine>
              </Show>

              {/* Drawn whatever the switch above says and on every platform: a
                  log file reached only through a control somebody can turn off
                  is a log file nobody sends. */}
              <div class={styles.logs}>
                <QuietButton onClick={() => viewing.mutate()}>
                  View Logs
                </QuietButton>
                <Note>
                  This run's log file, which is what the tray's own View Logs
                  opens.
                </Note>
              </div>

              <Show when={registering.isError}>
                <ErrorLine class={styles.failure}>
                  Launch on Startup could not be set:{" "}
                  {registering.error?.message}
                </ErrorLine>
              </Show>
              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The setting could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
              <Show when={viewing.isError}>
                <ErrorLine class={styles.failure}>
                  The log file could not be opened: {viewing.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </>
  );
}
