//! What Verkstead has installed onto this machine beyond what the wizard checks
//! (ADR-0024, *The box on Windows*): the **Components** card, and the pane
//! behind it.
//!
//! **One row today, the Annex's** — *Linux development (WSL)* — and the card is
//! named for the place rather than the row: a harness added after setup is the
//! next thing that lives here.
//!
//! **Drawn only on a machine that can have the row**, which is a Windows Keeper.
//! The server says so — its reading is `annex: null` everywhere else — and both
//! halves then draw nothing, the way the Desktop section draws nothing outside
//! the app, so the middle pane's order is what it always was.
//!
//! **The row reads the machine and nothing else**: how the box stands, off the
//! backend's probe; the release it runs, as the Keeper read it off the box when
//! it last booted it, so a stopped box says it too; and what its disk image
//! takes. None of it boots the box. Where WSL is waiting on a restart or the
//! firmware has virtualization off, the row says so in the wizard row's own
//! words, there being nothing to press here until the human has done that.

import { Match, Show, Switch, type JSX } from "solid-js";

import { sized } from "../Attaching";
import { PaneSticky } from "../Panes";
import { loadComponents } from "../api/client";
import type { AnnexComponent, AnnexState } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { BARRED } from "../setup/Dependencies";
import { WSL_RESTART_OWED } from "../setup/instructions";
import { PaneHead } from "../workbench/PaneHead";
import styles from "./Components.module.css";
import { SettingsCard } from "./SettingsCard";

/// What the Annex's row is called, which is what the wizard's WSL row is
/// called: one thing named one way on both pages.
const ANNEX = "Linux development (WSL)";

/// What each state of the box reads as on its row.
///
/// The two that are WSL waiting on the human read *not installed*, which is
/// what they are: the note under the row says what is owed.
const STATE: Record<AnnexState, string> = {
  NotInstalled: "not installed",
  NeedsRestart: "not installed",
  VirtualizationOff: "not installed",
  Running: "running",
  Stopped: "stopped",
  Updating: "updating",
};

/// And what is owed before there can be one, in the wizard row's words — or
/// nothing for a state that owes the human nothing.
function owed(state: AnnexState): string | null {
  if (state === "NeedsRestart") return `${WSL_RESTART_OWED}.`;
  if (state === "VirtualizationOff") return BARRED.Virtualization;

  return null;
}

/// What this machine has installed, read for the card, the pane, and the page
/// deciding whether either is drawn.
///
/// A read of its own: none of it is a setting, and none of it is the cluster.
/// Re-read on the `devices` Nudge, which is what the Keeper's start says once
/// its Annex is linked — the moment the release the box runs is known.
export function useComponents() {
  return useReading(() => ({
    queryKey: ["components"],
    queryFn: loadComponents,
    freshness: { reconcile: "state" },
  }));
}

/// The card, where this machine can have a component — and nothing at all
/// where it cannot.
export function ComponentsCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const components = useComponents();

  return (
    <Show when={components.data?.annex} keyed>
      {(annex) => (
        <SettingsCard
          class={styles.componentsCard}
          title="Components"
          open={props.open}
          press={props.press}
          status={`WSL ${STATE[annex.state]}`}
          warn={annex.trouble !== null || owed(annex.state) !== null}
        />
      )}
    </Show>
  );
}

/// The pane: one row per component, which is the Annex's today.
export function ComponentsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const components = useComponents();

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title="Components" />
      </PaneSticky>

      <Switch>
        <Match when={components.isPending}>
          <Empty>Reading this machine…</Empty>
        </Match>
        <Match when={components.isError}>
          <ErrorLine>
            What this machine has installed couldn't be read.{" "}
            {components.error?.message}
          </ErrorLine>
        </Match>
        <Match when={components.data?.annex} keyed>
          {(annex) => (
            <ul class={styles.list}>
              <Annex of={annex} />
            </ul>
          )}
        </Match>
      </Switch>
    </>
  );
}

/// The Annex's row: its name and state, the release and the disk under them
/// where there is anything to say of either, and what is owed or went wrong.
function Annex(props: { of: AnnexComponent }): JSX.Element {
  /// The release and the disk, as one quiet line: no version before the
  /// Keeper's first boot has read one, rather than a guess.
  const facts = (): string[] =>
    [
      props.of.version === null ? null : `Verkstead ${props.of.version}`,
      props.of.disk === null ? null : sized(props.of.disk),
    ].filter((fact): fact is string => fact !== null);

  return (
    <li class={styles.component} data-component="annex">
      <p class={styles.name}>
        {ANNEX}
        <span class={styles.state} data-state={props.of.state}>
          {STATE[props.of.state]}
        </span>
      </p>

      <Show when={facts().length > 0}>
        <p class={styles.facts}>{facts().join(" · ")}</p>
      </Show>

      <Show when={owed(props.of.state)}>
        {(owing) => <Note>{owing()}</Note>}
      </Show>

      <Show when={props.of.trouble}>
        {(trouble) => <p class={styles.trouble}>{trouble()}</p>}
      </Show>
    </li>
  );
}
