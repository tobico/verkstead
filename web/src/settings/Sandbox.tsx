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

import { Match, Switch as Choose, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
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
          The settings couldn't be read: {settings.error?.message}
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

/// And the pane it opens: the build caches, then the binds, each a part under
/// its own heading.
export function SandboxPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title={TITLE} />
      </PaneSticky>

      <div class={styles.parts}>
        <BuildCaches />
        <SandboxBinds />
      </div>
    </>
  );
}
