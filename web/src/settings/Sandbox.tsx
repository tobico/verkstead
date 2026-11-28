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
//! after the wizard said it. See [`APP_CONTROL`].

import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
import type { AppControl } from "../api/types";
import { Empty, ErrorLine, Note } from "../notices";
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
          const waiting = () => [
            ...cacheWarnings(told()),
            ...(unseen(told().paths) > 0
              ? [`${counted(unseen(told().paths), "bind")} unseen`]
              : []),
          ];

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

/// What the standing line says where Smart App Control is on, the two states
/// worded apart.
///
/// **Here because the wizard is a first run rather than somewhere to go back
/// to.** The machine step says this while somebody is standing at the box; this
/// is where the same condition is read months later, and where a machine that
/// has started enforcing since onboarding says so. In the Sandbox section
/// because what it is about is what a session can do: the build caches and the
/// binds say what a session is given, and this says the one thing it cannot do
/// with what it builds.
///
/// Read-only, and nothing beside it to press. Turning the setting off is the
/// human's one-way decision in Windows Security, and Verkstead never writes it.
const APP_CONTROL: Record<AppControl, string> = {
  Enforcing:
    "Smart App Control is enforcing on this machine, so a session can't run " +
    "an executable it builds here. Verkstead never changes this setting: " +
    "turning it off is in Windows Security, and Windows can't turn it back " +
    "on without a reinstall.",
  Evaluation:
    "Smart App Control is evaluating on this machine. It blocks nothing " +
    "today, and Windows decides on its own whether to start enforcing it, " +
    "after which a session can't run what it builds here.",
};

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
        <Show when={settings.data?.app_control}>
          {(said) => (
            <Note class={styles.appControl}>{APP_CONTROL[said()]}</Note>
          )}
        </Show>

        <BuildCaches />
        <SandboxBinds />
      </div>
    </>
  );
}
