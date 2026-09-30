//! **Middle-Click Autoscroll**: hold the middle button and drag, and the pane
//! under the pointer scrolls with the pointer.
//!
//! It is one of the two things a middle button does on a desktop, and the one
//! Chromium does not do on Linux. Chromium has the whole feature already —
//! `blink::AutoscrollController`, the drift-with-the-pointer behaviour, the
//! release that ends it — behind the Blink runtime feature
//! `MiddleClickAutoscroll`, and that feature is enabled on Windows and on no
//! other platform. So a Windows Verkstead had autoscroll and a Linux one did
//! not, over the same document, for no reason either of them was aware of.
//!
//! **`--enable-blink-features` is the whole of asking for it**, and it is asked
//! for at the command line rather than per window: a Blink runtime feature is
//! read by the renderer as it starts, so there is nothing on `webPreferences`
//! and nothing the page can be told. Measured on the pinned Electron under a
//! nested X session: with the switch, a middle press and a 180px drag carries
//! the settings page's middle pane to the bottom of itself; without it,
//! `scrollTop` never leaves nought.
//!
//! **And it is asked for on every platform, rather than on the one that needed
//! it.** A gesture is the same gesture wherever there is a middle button to hold
//! down, so this is one switch and no platform branch — which is a switch that
//! is either right or wrong rather than three arms to keep true. **Windows** is
//! told what is already the case there, and a switch saying so changes nothing.
//! **A Mac** is told as well: a three-button mouse plugged into one has the
//! button, and a trackpad has nothing to hold, so the feature is reached by
//! whoever has the hardware for it and by nobody else. Neither platform gets an
//! arm of its own to go stale.
//!
//! **And it takes nothing away from the other thing a middle button does.** On
//! Linux a middle click in a text field pastes the X primary selection, and that
//! is Blink's own arm for an editable node rather than something autoscroll
//! stands in front of: with the switch on, a middle click in the composer's
//! Brief still pastes what `xclip -selection primary` put there, and a middle
//! click on a link that leads off the workbench still goes to the browser. Both
//! measured the same way, in both arms.
//!
//! A value rather than a call, there being nothing left to decide. Appending it
//! is `main.ts`'s, that being the one file holding the running `app`.

/// The Chromium switch that turns Blink runtime features on by name. One
/// `appendSwitch` owns its whole value, so this is the app's only reach for it —
/// a second feature asked for somewhere else would silently replace this one.
export const FEATURES = "enable-blink-features";

/// The feature itself, spelled as Blink spells it. Not a name to tidy: it is
/// matched against `runtime_enabled_features.json5`'s own string, and an
/// unrecognised one is ignored in silence rather than refused — so a tidied one
/// is an app that starts, says autoscroll is on, and does not autoscroll.
export const AUTOSCROLL = "MiddleClickAutoscroll";

/// A switch for the command line: what to append, and what to append it as.
export interface Switch {
  /// The switch's name, without its dashes — what `appendSwitch` takes.
  readonly name: string;
  /// And its value.
  readonly value: string;
}

/// What the app tells Chromium so that a held middle button scrolls, on every
/// platform it runs on.
export const AUTOSCROLLING: Switch = { name: FEATURES, value: AUTOSCROLL };
