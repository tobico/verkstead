//! What a device is drawn as, wherever it is drawn.
//!
//! One thing so far, and it is here rather than on either of the two pages
//! because both draw it: the Devices section of the Remote access pane puts a
//! mark beside every row, and the modal that asks whether to let a device in
//! puts the same mark beside the name it is asking about. A second copy of the
//! table would be a Windows drawn two ways.

import {
  faApple,
  faLinux,
  faWindows,
} from "@fortawesome/free-brands-svg-icons";
import { faDesktop } from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";

/// Which mark stands beside a device's name, off the word for its operating
/// system.
///
/// Font Awesome's brand set, drawn through `Icon` like every other icon in the
/// app — so the bundle carries these three and nothing else of it.
///
/// Matched on what the word *starts* with, because one of them is not a bare
/// platform name: a WSL reads *Linux (WSL)* and wears the Linux mark, which is
/// the whole point of the word — a Windows machine and the WSL on it share a
/// hostname, and this is what tells the two rows apart.
///
/// A word this build has no mark for still draws a row, which is what the
/// desktop is for: a device is worth naming whether or not its OS is one of the
/// three anybody here has heard of.
export function osIcon(os: string): IconDefinition {
  if (os.startsWith("macOS")) return faApple;
  if (os.startsWith("Windows")) return faWindows;
  if (os.startsWith("Linux")) return faLinux;

  return faDesktop;
}
