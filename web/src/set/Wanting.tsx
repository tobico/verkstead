//! What a proposal says the work wants: which platforms, and why, drawn under
//! the three directions — as a line of prose, or, on a clustered Verkstead
//! while the Set waits, as the *May be transferred to* ticks.
//!
//! Its own module because the chooser and the record it becomes both draw it,
//! and a proposal wanting nothing draws nothing at all — which is most of them.

import type { JSX } from "solid-js";
import { For, Show, createUniqueId } from "solid-js";

import type {
  ClusterDevice,
  DeviceIdentity,
  WantedPlatform,
  WantView,
} from "../api/types";
import { ErrorLine } from "../notices";
import { rowPress } from "../rows";
import page from "./Sheet.module.css";

/// Each platform as it spells itself, rather than as the wire word an agent
/// writes after `on`.
export const PLATFORM: Record<WantedPlatform, string> = {
  linux: "Linux",
  macos: "macOS",
  windows: "Windows",
};

/// The joining word before the `index`th of `count` platforms: nothing before
/// the first, *and* before the last, and a comma between the rest.
function joining(index: number, count: number): string {
  if (index === 0) return "";
  return index === count - 1 ? ", and " : ", ";
}

/// *This work wants Windows for the ConPTY stage, and macOS for the bundle.*
///
/// Platform by platform in the order the agent wrote them, each with the reason
/// it gave, because the reason is what the human weighs the platform against.
export function Wanting(props: { wants: WantView[] }): JSX.Element {
  return (
    <Show when={props.wants.length > 0}>
      <p class={page.wants}>
        This work wants{" "}
        <For each={props.wants}>
          {(want, index) => (
            <>
              {joining(index(), props.wants.length)}
              <strong>{PLATFORM[want.platform]}</strong> for{" "}
              <span innerHTML={want.for_html} />
            </>
          )}
        </For>
        .
      </p>
    </Show>
  );
}

/// The devices the chooser draws a tick for: every one of the cluster the work
/// could be permitted to move to, where the proposal wants a platform and there
/// is a cluster at all. Nothing otherwise, and the chooser draws no ticks.
export function ticking(
  wants: WantView[],
  cluster: ClusterDevice[] | null,
): ClusterDevice[] {
  if (wants.length === 0 || cluster === null) return [];
  return cluster.filter((member) => member.standing === "Tickable");
}

/// The *May be transferred to* ticks under the three directions: the same
/// ticks the Attach menu's device rows make (ADR-0028), drawn in the open where
/// the human is already deciding.
///
/// Platform by platform first, each with the reason the agent wanted it and
/// the devices of it under that — a tick for one that could be permitted, and a
/// line for one that is already or is where the work is. A platform the cluster
/// has no device of says so, with nothing to tick. Then the cluster's other
/// devices the work could be permitted to move to, for the human who knows
/// better than the agent where it should go.
export function Placing(props: {
  wants: WantView[];
  cluster: ClusterDevice[];
  ticked: (device: string) => boolean;
  tick: (device: DeviceIdentity, ticked: boolean) => void;
  refused: () => string | null;
}): JSX.Element {
  const heading = createUniqueId();

  const of = (platform: WantedPlatform) =>
    props.cluster.filter((member) => member.platform === platform);
  const others = () =>
    ticking(props.wants, props.cluster).filter(
      (member) =>
        !props.wants.some((want) => want.platform === member.platform),
    );

  const row = (member: ClusterDevice) => {
    const name = member.device.name;

    if (member.standing === "WorkIsHere") {
      return <p class={page.placed}>The work is on {name}.</p>;
    }
    if (member.standing === "Permitted") {
      return (
        <p class={page.placed}>
          The work may already be transferred to {name}.
        </p>
      );
    }

    const ticked = () => props.ticked(member.device.device);
    return (
      <label
        class={page.tick}
        onClick={rowPress(() => props.tick(member.device, !ticked()))}
      >
        <input
          type="checkbox"
          name="transferable"
          value={member.device.device}
          checked={ticked()}
          onChange={(event) =>
            props.tick(member.device, event.currentTarget.checked)
          }
        />
        <span>{name}</span>
      </label>
    );
  };

  return (
    <div role="group" aria-labelledby={heading} class={page.placing}>
      <p id={heading} class={page.placingHead}>
        May be transferred to
      </p>
      <For each={props.wants}>
        {(want) => (
          <div class={page.wanted}>
            <p class={page.wants}>
              <strong>{PLATFORM[want.platform]}</strong> for{" "}
              <span innerHTML={want.for_html} />
            </p>
            <For
              each={of(want.platform)}
              fallback={
                <p class={page.placed}>
                  No {PLATFORM[want.platform]} device is linked to this one, so
                  the work will run where it is.
                </p>
              }
            >
              {row}
            </For>
          </div>
        )}
      </For>
      <Show when={others().length > 0}>
        <div class={page.wanted}>
          <For each={others()}>{row}</For>
        </div>
      </Show>
      <Show when={props.refused()}>
        {(said) => <ErrorLine class={page.placed}>{said()}</ErrorLine>}
      </Show>
    </div>
  );
}
