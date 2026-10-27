//! The foot of the wizard's paper, and how a step puts its press there.
//!
//! **The frame owns the foot and the step owns what is pressed in it.** Back is
//! the frame's — it opens the step before, which is the frame's arithmetic — and
//! the press on the right is the step's, because what it does is: start a run,
//! save what is ticked, finish the wizard. So the frame draws the foot pinned to
//! the bottom of the paper and hands its right-hand end down, and a step wraps
//! its press in [`Foot`] rather than drawing it under its own rows.
//!
//! **And a step drawn with no frame around it draws its press where it is.**
//! The step suites mount a step alone, and a press that went looking for a foot
//! nobody drew would be no press at all.

import { Portal } from "solid-js/web";
import {
  Show,
  createContext,
  useContext,
  type JSX,
  type ParentProps,
} from "solid-js";

import styles from "./SetupPage.module.css";

/// Where the open step's press goes: the right-hand end of the frame's foot.
/// Nothing where there is no frame.
export const FootContext = createContext<HTMLElement>();

/// A step's press, drawn in the frame's foot.
export function Foot(props: ParentProps): JSX.Element {
  const foot = useContext(FootContext);

  return (
    <Show
      when={foot}
      fallback={<div class={styles.unframed}>{props.children}</div>}
    >
      {(foot) => (
        <Portal
          mount={foot()}
          ref={(wrapper) => (wrapper.className = styles.pressing!)}
        >
          {props.children}
        </Portal>
      )}
    </Show>
  );
}
