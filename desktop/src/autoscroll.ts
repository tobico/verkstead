//! **Middle-Click Autoscroll**: hold the middle button and drag, and the pane
//! under the pointer scrolls with the pointer.
//!
//! It is one of the two things a middle button does on a desktop, and the one
//! Chromium does not do on Linux. Chromium has the whole feature already —
//! `blink::AutoscrollController`, the drift-with-the-pointer behaviour, the
//! release that ends it — behind the Blink runtime feature
//! `MiddleClickAutoscroll`, and that feature is enabled on Windows and on no
//! other platform. So a Windows Verkstead has autoscroll and a Linux one does
//! not, over the same document, for no reason either of them is aware of.
//!
//! **`--enable-blink-features` is the whole of asking for it**, and it is asked
//! for at the command line rather than per window: a Blink runtime feature is
//! read by the renderer as it starts, so there is nothing on `webPreferences`
//! and nothing the page can be told. Measured on the pinned Electron under a
//! nested X session: with the switch, a middle press and a 200px drag scrolls
//! the page some six thousand pixels; without it, `scrollY` never leaves nought.
//!
//! **And it takes nothing away from the other thing a middle button does.** On
//! Linux a middle click in a text field pastes the X primary selection, and that
//! is Blink's own arm for an editable node rather than something autoscroll
//! stands in front of: with the switch on, a middle click in an `<input>` still
//! pastes what `xclip -selection primary` put there, and a middle press on a
//! link still scrolls nothing. Both measured the same way, in both arms.
//!
//! **A function of the platform and nothing else**, for the reason
//! [`platform.ts`](./platform.js) is one: the value goes in and a switch comes
//! back, so the arm this machine will never run is still an arm vitest calls.
//! Appending it is `main.ts`'s, that being the one file holding the running
//! `app`.

/// The Chromium switch that turns Blink runtime features on by name. One
/// `appendSwitch` owns its whole value, so this is the app's only reach for it —
/// a second feature asked for somewhere else would silently replace this one.
export const FEATURES = "enable-blink-features";

/// The feature itself, spelled as Blink spells it. Not a name to tidy: it is
/// matched against `runtime_enabled_features.json5`'s own string, and an
/// unrecognised one is ignored in silence rather than refused.
export const AUTOSCROLL = "MiddleClickAutoscroll";

/// A switch for the command line: what to append, and what to append it as.
export interface Switch {
  /// The switch's name, without its dashes — what `appendSwitch` takes.
  readonly name: string;
  /// And its value.
  readonly value: string;
}

/// What this platform has to be told to autoscroll, and `undefined` where it
/// has to be told nothing.
///
/// **Linux is what is told**, along with every other Unix that is not a Mac, and
/// the two that are not are not told for opposite reasons. **Windows** has the
/// feature on already: Chromium ships it enabled there, the switch would change
/// nothing, and asking anyway would put a line in the log about something that
/// was already true. **A Mac** is not a platform that autoscrolls — nothing else
/// on it does, a trackpad has no middle button to hold, and middle-click
/// autoscroll where the desktop has no such gesture is a pane that scrolls when
/// somebody meant to paste. Neither is the brief either, which was Linux.
export function autoscrolling(platform: NodeJS.Platform): Switch | undefined {
  switch (platform) {
    case "darwin":
    case "win32":
      return undefined;

    default:
      return { name: FEATURES, value: AUTOSCROLL };
  }
}
