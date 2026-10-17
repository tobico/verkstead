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
//!
//! **Install is the wizard's run** (ADR-0024: *Install from the pane is the
//! wizard's run*), for a machine that skipped the row at setup: the same plan
//! and the same units, and the same two screens drawn in this pane — the
//! install screen while it goes, and the hint screen with the row's own
//! instruction where a restart is owed or something failed. They are the
//! wizard's components over the wizard's reading rather than copies of them, so
//! there is one of each. A run already going, from this pane or another
//! device's, is drawn rather than started again.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  on,
  type JSX,
} from "solid-js";

import { sized } from "../Attaching";
import { PaneSticky } from "../Panes";
import {
  cancelInstall,
  installAnnex,
  loadComponents,
  loadOnboarding,
} from "../api/client";
import type {
  AnnexComponent,
  AnnexState,
  DependencyView,
  OnboardingView,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import {
  BARRED,
  Hints,
  Installing,
  absent,
  running,
} from "../setup/Dependencies";
import { PROBE, RUNNING } from "../setup/steps";
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

/// The pane: one row per component, which is the Annex's today — or, while
/// its Install is going or has something left to say, the wizard's screen for
/// that.
export function ComponentsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const components = useComponents();
  const queries = useQueryClient();

  // Whether the hint screen is up: set by a run this pane watched end with the
  // WSL row still missing, and put down by its Back or its Next.
  const [hinting, setHinting] = createSignal(false);

  // And what the server said about a press it would not take.
  const [refused, setRefused] = createSignal<string | null>(null);

  // The wizard's own reading, which is what its two screens are drawn from:
  // the run, and the WSL row as the machine and the run have it. Read with the
  // wizard's cadence while there is something to watch — a run going, or a
  // hint screen waiting on the row — and once on opening otherwise, which is
  // how a run already going is found and drawn rather than started again.
  const onboarding = useReading(() => {
    const waiting = hinting();

    return {
      queryKey: ["onboarding"],
      queryFn: loadOnboarding,
      refetchInterval: (query) => {
        const reading = query.state.data as OnboardingView | undefined;

        if (running(reading?.run)) {
          return RUNNING;
        }

        return waiting ? PROBE : false;
      },
      freshness: { reconcile: "dependency" },
    };
  });

  /// The WSL row of the wizard's reading.
  const wsl = (): DependencyView | undefined =>
    onboarding.data?.dependencies.find((row) => row.dependency === "Wsl");

  /// A run is over: the row reads what the probe now says, and the hint screen
  /// comes up where the row is still missing — a restart owed, or a step that
  /// failed — with the row's own instruction on it.
  const landed = (reading: OnboardingView): void => {
    void queries.invalidateQueries({ queryKey: ["components"] });

    const row = reading.dependencies.find((row) => row.dependency === "Wsl");
    setHinting(row !== undefined && absent(row.state));
  };

  // Watched from the reading rather than from the press, so that a run this
  // pane found going is landed the same way as one it started.
  createEffect(
    on(
      () => running(onboarding.data?.run),
      (now, before) => {
        if (before === true && !now && onboarding.data !== undefined) {
          landed(onboarding.data);
        }
      },
    ),
  );

  const start = useMutation(() => ({
    mutationFn: installAnnex,
    onSuccess: (reading: OnboardingView) => {
      setRefused(null);
      queries.setQueryData(["onboarding"], reading);

      // A run with nothing in it to do — a server with no way to raise a
      // dialog — is over before the answer arrives, so nothing will be seen
      // to end.
      if (!running(reading.run)) {
        landed(reading);
      }
    },
    onError: (error: Error) => {
      setRefused(error.message);

      // Refused as already going: the run there is, is what to draw.
      void queries.invalidateQueries({ queryKey: ["onboarding"] });
    },
  }));

  const stop = useMutation(() => ({
    mutationFn: cancelInstall,
    onSuccess: (reading: OnboardingView) =>
      queries.setQueryData(["onboarding"], reading),
  }));

  /// Leave the hint screen for the row, read again.
  const unhint = (): void => {
    setHinting(false);
    void queries.invalidateQueries({ queryKey: ["components"] });
  };

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
        <Match
          when={
            running(onboarding.data?.run) ? onboarding.data?.run : null
          }
        >
          {(run) => (
            <div class={styles.screen}>
              <Installing
                run={run()}
                stopping={stop.isPending}
                cancel={() => stop.mutate()}
              />
            </div>
          )}
        </Match>
        <Match when={hinting() ? onboarding.data : null}>
          {(reading) => (
            <div class={styles.screen}>
              <Hints
                reading={reading()}
                rows={wsl() === undefined ? [] : [wsl()!]}
                detected={
                  wsl() === undefined || absent(wsl()!.state) ? 0 : 1
                }
                back={unhint}
                onwards={unhint}
              />
            </div>
          )}
        </Match>
        <Match when={components.data?.annex} keyed>
          {(annex) => (
            <ul class={styles.list}>
              <Annex
                of={annex}
                install={() => start.mutate()}
                pressing={start.isPending}
                refused={refused()}
              />
            </ul>
          )}
        </Match>
      </Switch>
    </>
  );
}

/// The Annex's row: its name and state, the release and the disk under them
/// where there is anything to say of either, what is owed or went wrong, and
/// Install where there is no box.
function Annex(props: {
  of: AnnexComponent;
  install: () => void;
  pressing: boolean;
  refused: string | null;
}): JSX.Element {
  /// The release and the disk, as one quiet line: no version before the
  /// Keeper's first boot has read one, rather than a guess.
  const facts = (): string[] =>
    [
      props.of.version === null ? null : `Verkstead ${props.of.version}`,
      props.of.disk === null ? null : sized(props.of.disk),
    ].filter((fact): fact is string => fact !== null);

  return (
    <li class={styles.component} data-component="annex">
      <div class={styles.head}>
        <p class={styles.name}>
          {ANNEX}
          <span class={styles.state} data-state={props.of.state}>
            {STATE[props.of.state]}
          </span>
        </p>

        {/* Only where there is no box and nothing owed first: a restart or
            the firmware is the human's, and a press would have nothing to do
            until it is done. */}
        <Show when={props.of.state === "NotInstalled"}>
          <button
            type="button"
            class={styles.install}
            disabled={props.pressing}
            onClick={() => props.install()}
          >
            Install
          </button>
        </Show>
      </div>

      <Show when={facts().length > 0}>
        <p class={styles.facts}>{facts().join(" · ")}</p>
      </Show>

      <Show when={owed(props.of.state)}>
        {(owing) => <Note>{owing()}</Note>}
      </Show>

      <Show when={props.of.trouble}>
        {(trouble) => <p class={styles.trouble}>{trouble()}</p>}
      </Show>

      <Show when={props.refused}>
        {(why) => <ErrorLine>{why()}</ErrorLine>}
      </Show>
    </li>
  );
}
