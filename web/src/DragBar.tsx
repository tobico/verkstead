//! The bar a page with no pane head is dragged by.
//!
//! The app's window has no title bar (ADR-0020), so what moves it is the bar
//! across the top of whatever is open — and every pane's head is that bar, which
//! is how the workbench, the settings, the composer and the Set sheet all move
//! the window without one of them having been told about it. Three pages have no
//! head to do it with: the wizard, which is the only page there is while
//! onboarding mode is on; the no-such-page, which is one line of notice and
//! nothing else; and the moment before the verdict about this machine lands,
//! where the gate draws nothing at all. A window that cannot be moved for the
//! length of any of them is the same hole three times over — which is why it is
//! one component in three places rather than three bars.
//!
//! **The head's band, measured where the head's is** — [`band`](./head.ts),
//! which is the same sum the app is told the overlay's height in. The bar stands
//! where a head would stand and the platform draws its controls as tall as a
//! head, so a bar of any other height would leave them hanging over its edge or
//! under it.
//!
//! **Fixed across the window rather than stuck to the top of the page**, which
//! is the one thing here that is not what a head does. A head spans its pane and
//! a pane spans the window; these three pages are drawn in the column
//! `styles/base.css` measures, and the window's controls are at its own top-right
//! corner — outside that column in any window wider than it. So what is drawn is
//! a strip across the whole window, and what stands in the page is the room it
//! takes. The strip is the paper the page is on, the way `.paneChrome` is, so
//! what scrolls up passes under it rather than over it.
//!
//! **And only where the bridge is.** A drag region is inert outside the app, so a
//! bar drawn in a browser would move nothing — but it would still be a bar: a
//! band of the head's height across the top of a page a phone has no use for. So
//! this is drawn on the bridge being there, the way the Desktop section of the
//! settings is, rather than drawn always and relied upon to be harmless.

import type { JSX } from "solid-js";

import styles from "./DragBar.module.css";
import { band } from "./head";
import { bridge } from "./settings/bridge";

/// The bar, where this page is drawn inside the app — and nothing at all where
/// it is not.
export function DragBar(): JSX.Element {
  // Read once rather than followed, the way the Desktop section reads it: a
  // preload is there before the document is, so nothing can arrive or leave
  // while a page is open.
  if (bridge() === null) {
    return null;
  }

  // How tall the band stands, carried as a variable the way the frame carries
  // what its own controls took: the room in the page and the strip over the
  // window are the same height, and there is one place it is said.
  return (
    <div class={styles.bar} style={{ "--band": `${band()}px` }}>
      <div class={styles.drag} />
    </div>
  );
}
