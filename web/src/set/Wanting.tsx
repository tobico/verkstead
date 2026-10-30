//! What a proposal says the work wants: which platforms, and why, drawn under
//! the three directions in a line of prose.
//!
//! Its own module because the chooser and the record it becomes both draw it,
//! and a proposal wanting nothing draws nothing at all — which is most of them.

import type { JSX } from "solid-js";
import { For, Show } from "solid-js";

import type { WantedPlatform, WantView } from "../api/types";
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
