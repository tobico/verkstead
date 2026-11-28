//! What a session runs inside, beyond its own worktree: the build caches it is
//! handed and the paths bound into its sandbox. One section of the settings
//! page, `/settings/sandbox`, with the two as parts of one pane.
//!
//! **One section rather than two**, because they are one question — what the
//! sandbox a session starts in holds — and the page reads better as a short list
//! of places than as one entry per kind of directory. The parts keep everything
//! they had, each saving itself as it did: see [`BuildCaches`] in
//! `Languages.tsx` and [`SandboxBinds`] in `SandboxBinds.tsx`.
//!
//! The card's line says what is waiting on the human in either part first — an
//! unreadable entry, no sccache, a bind the server cannot see — and otherwise
//! how many caches are on and how many binds there are.
//!
//! **And one line over both parts that is neither**: whether Smart App Control
//! is on, which is the machine rather than a setting and is read here long
//! after the wizard said it, in the wizard's own words — see
//! [`APP_CONTROL`](../appControl.ts). It leads the card's line as well, where
//! it is on — see [`ON_THE_CARD`] — because this is the surface that is read
//! long after onboarding, and a warning the open pane alone shows is one
//! nobody opens the pane to find.

import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
import type { AppControl } from "../api/types";
import { APP_CONTROL } from "../appControl";
import { Empty, ErrorLine } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { BuildCaches, cacheWarnings, cachesOn } from "./Languages";
import { useSettings } from "./PathEditor";
import { SandboxBinds, unseen } from "./SandboxBinds";
import { SettingsCard } from "./SettingsCard";
import styles from "./Sandbox.module.css";

/// What the section is called, in the one place the card and the pane read it.
const TITLE = "Sandbox";

/// A count with the word it counts, so the line reads `1 bind` rather than
/// `1 binds`.
function counted(many: number, one: string): string {
  return `${many} ${one}${many === 1 ? "" : "s"}`;
}

/// What the card says where Smart App Control is on: the state in two words,
/// on the line that names what is waiting on the human.
///
/// **On the card and not only in the pane**, the way an unreadable
/// `config.yaml` entry is and for the same reason — whoever needs to read it is
/// precisely whoever is not in here editing build caches and binds. This is the
/// surface meant to be read long after onboarding, and a condition nobody opens
/// the pane to find would leave the wizard's one sighting of it as the only one
/// there ever was.
///
/// **Evaluation counts as well as enforcing.** It blocks nothing today, and
/// Windows decides for itself whether to harden, so a machine in it is one that
/// can cross into the broken state with nobody told — and a card that said
/// nothing is how nobody is told. The pane is where the two states are worded
/// apart; this is two words naming which.
const ON_THE_CARD: Record<AppControl, string> = {
  Enforcing: "Smart App Control enforcing",
  Evaluation: "Smart App Control evaluating",
};

/// The section as the page lists it.
export function SandboxCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <Choose>
      <Match when={settings.isPending}>
        <Empty>Reading the settings…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          The settings couldn't be read. {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data}>
        {(told) => {
          const waiting = () => {
            // The machine's own condition first, it being about neither part:
            // what a session can do with what it builds, over what either part
            // hands it.
            const guard = told().app_control;

            return [
              ...(guard ? [ON_THE_CARD[guard]] : []),
              ...cacheWarnings(told()),
              ...(unseen(told().paths) > 0
                ? [`${counted(unseen(told().paths), "bind")} unseen`]
                : []),
            ];
          };

          return (
            <SettingsCard
              class={styles.sandboxCard}
              title={TITLE}
              open={props.open}
              press={props.press}
              warn={waiting().length > 0}
              status={
                waiting().length > 0
                  ? waiting().join(" · ")
                  : `${counted(cachesOn(told()), "build cache")} · ${counted(
                      told().paths.binds.length,
                      "bind",
                    )}`
              }
            />
          );
        }}
      </Match>
    </Choose>
  );
}

/// And the pane it opens: the standing line where there is one, then the build
/// caches and the binds, each a part under its own heading.
export function SandboxPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title={TITLE} />
      </PaneSticky>

      <div class={styles.parts}>
        {/* Whether Smart App Control is on, over both parts.

            **Here because the wizard is a first run rather than somewhere to go
            back to.** The machine step says this while somebody is standing at
            the box; this is where the same condition is read months later, and
            where a machine that has started enforcing since onboarding says so.
            In the Sandbox section because what it is about is what a session
            can do: the build caches and the binds say what a session is given,
            and this says the one thing it cannot do with what it builds.

            **Drawn as the warning it is**, which is the shape the
            unreadable-entry line in the part below already wears: a condition
            this section reports looks like the other one it can report, rather
            than like a note under a control.

            Read-only, and nothing beside it to press. Turning the setting off
            is the human's one-way decision in Windows Security, and Verkstead
            never writes it. The words are the wizard's too — see
            `APP_CONTROL`. */}
        <Show when={settings.data?.app_control}>
          {(said) => (
            <p class={styles.appControl}>
              {APP_CONTROL[said()].said} {APP_CONTROL[said()].means}
            </p>
          )}
        </Show>

        <BuildCaches />
        <SandboxBinds />
      </div>
    </>
  );
}
