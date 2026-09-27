//! What a device is drawn as, wherever it is drawn.
//!
//! The mark first, and it is here rather than on either of the two pages
//! because both draw it: the Devices section of the Remote access pane puts a
//! mark beside every row, and the modal that asks whether to let a device in
//! puts the same mark beside the name it is asking about. A second copy of the
//! table would be a Windows drawn two ways.
//!
//! And under it the reading that mark is drawn off, for the same reason one
//! page along: the sidebar's rows carry their own device, and the header of
//! the pane one of them opens has to look one up.

import {
  faApple,
  faLinux,
  faWindows,
} from "@fortawesome/free-brands-svg-icons";
import { faDesktop } from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";

import { loadDevices } from "./api/client";
import type { DeviceIdentity, DevicesView } from "./api/types";
import { useReading } from "./freshness";

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

/// The **Devices** reading: what this Verkstead *is* — the device this
/// machine runs, and every other device in its cluster.
///
/// Nothing here is configured, which is why it is a read of its own rather
/// than a field of the settings query the rest of the page shares: what
/// device this is changes when a link is made or an address moves, and a
/// setting changes when somebody saves one.
///
/// Here rather than on the Remote access pane because two pages read it now.
/// That pane draws the rows of it; the header of an open Conversation reads
/// it for one device — the machine that record's work is being done on — to
/// put the name and the mark beside the branch. One reading between them is
/// one cache entry, re-read on the `devices` Nudge for both.
///
/// Merged by the device id: what says one row from another is the id, so a
/// re-read that found another device leaves the rows it already drew alone.
///
/// **Always this device's own**, whichever machine the page around it is
/// about: the membership is the hub's own finding about its cluster, and a
/// member asked for one would answer the cluster as it sees it — so there is
/// no `keyOf` around the key and no device on the call (see `reaching.ts`).
///
/// `on` is for the one page that must not ask at all: a share fetches
/// nothing, and the header it draws is a record's rather than a machine's.
export function useDevices(on?: () => boolean) {
  return useReading(() => ({
    queryKey: ["devices"],
    queryFn: loadDevices,
    enabled: on === undefined || on(),
    freshness: { reconcile: "device" },
  }));
}

/// Which device a page is about, as something drawing it needs one: the name
/// and the OS word, or `null` where there is nothing to say.
///
/// **`null` is the answer wherever there is no cluster**, which is the rule the
/// sidebar's rows are drawn under and the reason they carry no device of their
/// own there (see `RowDevice`): a Verkstead linked to nothing draws the page it
/// has always drawn, and a machine's own name on every header would be a
/// column of one answer repeated.
///
/// And `null` again while the reading has not arrived, or for an id the
/// membership does not hold — a member unlinked in another tab, say. Both are
/// the same thing to a caller: there is no device to name yet.
export function deviceShown(
  devices: DevicesView | undefined,
  device: string | null,
): DeviceIdentity | null {
  if (!devices || devices.members.length === 0) return null;
  if (device === null) return devices.this;

  return (
    devices.members.find((member) => member.identity.device === device)
      ?.identity ?? null
  );
}
