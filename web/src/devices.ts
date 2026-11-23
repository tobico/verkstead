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
//! the pane one of them opens has to look one up — and leave the mark off
//! where it is this device's own (see [`sameMark`]).

import {
  faApple,
  faLinux,
  faWindows,
} from "@fortawesome/free-brands-svg-icons";
import { faDesktop } from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";
import { useQueryClient } from "@tanstack/solid-query";

import { loadDevices, retrying } from "./api/client";
import type { DeviceIdentity, DevicesView } from "./api/types";
import { useReading } from "./freshness";
import type { Device } from "./reaching";
import { repoParent } from "./remembered";

/// How often the reading is asked again while an Update is going — this
/// device's, or a member's pressed here: often enough that *restarting…* gives
/// way to the new version within a second or so of the new server answering.
const UPDATING = 1_000;

/// The one OS word that is not a bare platform name — see the server's
/// `platform::os_word`, which is where it is written.
const WSL = "Linux (WSL)";

/// The Windows four-pane glyph as a badge: Font Awesome's own `faWindows` path
/// at half size, moved into the lower-right of the box [`faWsl`] draws in.
///
/// The glyph is straight lines and nothing else, so halving it and moving it is
/// arithmetic on its numbers rather than a drawing of our own — which is what
/// keeps this Microsoft's shape rather than an impression of it.
const WSL_BADGE =
  "M394 512.9l91.8-12.6 0 88.7-91.8 0 0-76zM394 675.2l91.8 12.7 0-87.6-91.8" +
  " 0 0 75zm101.9 14l122.1 16.9 0-105.8-122.1 0 0 89zm0-190.3l0 90.1 122.1 0" +
  " 0-106.9-122.1 16.9z";

/// The mark a **Linux (WSL)** device wears: Tux with that badge in the corner.
///
/// The one mark here the app composes rather than imports, and the reason it
/// has to: an **Annex** carries its Keeper's hostname, so on the device select
/// the Keeper's row and its Annex's row read as one machine twice and the mark
/// is the whole of what tells them apart. *Mainly the penguin*, which is what
/// says the box is a Linux, with the Windows glyph small beside its foot saying
/// which Linux.
///
/// **Drawn in one colour like every other mark**, so the badge has to stand
/// clear of the penguin rather than sit on it: two paths of one `currentColor`
/// that touched would be one shape. Which is what the box is for — Tux is
/// `faLinux`'s path untouched and the box is grown around it, leaving the
/// corner the badge stands in. The box keeps the Linux mark's proportions
/// (624/712 against 448/512), so the row's mark takes up what it always did.
const faWsl: IconDefinition = {
  // Tux's own, because that is what this is: the penguin, said twice over.
  ...faLinux,
  icon: [
    624,
    712,
    faLinux.icon[2],
    faLinux.icon[3],
    [faLinux.icon[4], WSL_BADGE].flat(),
  ],
};

/// Which mark stands beside a device's name, off the word for its operating
/// system.
///
/// Font Awesome's brand set, drawn through `Icon` like every other icon in the
/// app — so the bundle carries these three and nothing else of it, and the
/// fourth is the two of them composed (see [`faWsl`]).
///
/// Matched on what the word *starts* with, because one of them is not a bare
/// platform name: a WSL reads *Linux (WSL)*, and it is asked before the plain
/// Linux arm because every WSL would answer that one too.
///
/// A word this build has no mark for still draws a row, which is what the
/// desktop is for: a device is worth naming whether or not its OS is one of the
/// three anybody here has heard of.
export function osIcon(os: string): IconDefinition {
  if (os.startsWith("macOS")) return faApple;
  if (os.startsWith("Windows")) return faWindows;
  if (os.startsWith(WSL)) return faWsl;
  if (os.startsWith("Linux")) return faLinux;

  return faDesktop;
}

/// The three platforms, spelled as the people whose machine it is spell them —
/// the server's own `Platform::word` stated a second time, as [`sameMark`] below
/// states `platform::same_mark`. In any order: no one of them is a prefix of
/// another, so which is held against a word first decides nothing.
const PLATFORMS = ["macOS", "Windows", "Linux"];

/// What the platform a device is on is called where a person reads it, out of
/// that device's OS word.
///
/// The platform's own name, which is the word itself everywhere but on a WSL:
/// *Linux (WSL)* is a Linux, and the parenthesis is there to tell two rows of
/// one hostname apart on the Devices list rather than to be read as a platform
/// (see [`osIcon`], which is the mark drawn off that same difference). A word
/// this build has never heard of is its own answer, for the reason a device
/// with no mark still draws a row.
///
/// **What the two options on an Annex's Open repo card are labelled by**
/// (ADR-0024): a repository is on Windows or on Linux, and which machine of the
/// pair that is is the member row's own business — so a Docker box beside a Mac
/// names itself here with no new copy.
export function platformWord(os: string): string {
  return PLATFORMS.find((platform) => os.startsWith(platform)) ?? os;
}

/// Whether two OS words are drawn with the same mark, which is what *the same
/// OS* means beside a device's name: a mark that is the mark of the machine the
/// browser opened says nothing, so it is left off. *Linux (WSL)* seen from
/// *Linux* is **not** the same — the WSL has a mark of its own, and that is the
/// mark worth drawing. Two words this build has no mark for are the same.
///
/// **The server's `platform::same_mark` stated a second time**, and the two
/// must agree: the hub decides off that one whether a sidebar row carries its
/// device's OS at all (see `RowDevice`), and the pane header decides off this
/// one, so a row and the header it opens say the one device the one way.
export function sameMark(one: string, other: string): boolean {
  return osIcon(one) === osIcon(other);
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
  const queries = useQueryClient();

  /// Whether the reading last read says this device's own Update is going.
  const updating = (): boolean =>
    queries.getQueryData<DevicesView>(["devices"])?.this_updating != null;

  /// And whether it says a member's is, pressed here: its progress reaches
  /// this server with no Nudge either.
  const memberUpdating = (): boolean =>
    queries
      .getQueryData<DevicesView>(["devices"])
      ?.members.some((member) => member.updating != null) ?? false;

  return useReading(() => ({
    queryKey: ["devices"],
    queryFn: loadDevices,
    enabled: on === undefined || on(),
    // And polled while this device's own Update is going, the way the Annex's
    // row is while it reads *updating*: the app's progress reports carry no
    // Nudge, and the server that would send one is about to go. Kept polling
    // through the failed reads of the restart, so the page finds the new
    // server the moment it answers.
    refetchInterval: () => (updating() || memberUpdating() ? UPDATING : false),
    // And a failed read is not retried meanwhile: a server that has stopped
    // answering is the restart, and the row says so at the first failure
    // rather than after the ordinary backoff.
    retry: (attempts: number, error: Error) =>
      !updating() && retrying(attempts, error),
    freshness: { reconcile: "device" },
  }));
}

/// Where a new repository on a device starts its browse — see `repoParent` in
/// `remembered.ts` — with whether that device is an **Annex** read off the
/// cluster, an Annex's repositories having a place of their own to go.
export function useRepoParent(): (device: Device) => string {
  const devices = useDevices();

  return (device) =>
    repoParent(device, deviceShown(devices.data, device)?.kept_by != null);
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

/// The sessions running on a device, as an Update's confirm card names them
/// for being stopped and resumed (ADR-0027, *A confirm card names the sessions
/// that will be stopped*): *the session on rate-limiting*, *the sessions on
/// rate-limiting and 2 drafts* — each branch by name and the drafts counted.
/// `null` with none running.
///
/// Here rather than on the Remote access pane because the Desktop page's
/// Install says the same of this device's: installing quits the app, and the
/// server with it.
export function sessionsOn(branches: string[], drafts: number): string | null {
  const count = branches.length + drafts;
  if (count === 0) return null;

  const named = [
    ...branches,
    ...(drafts === 0 ? [] : [drafts === 1 ? "a draft" : `${drafts} drafts`]),
  ];
  const sessions = count === 1 ? "the session" : "the sessions";

  return `${sessions} on ${listed(named)}`;
}

/// `a`, `a and b`, `a, b and c`.
export function listed(names: string[]): string {
  if (names.length < 2) return names.join("");

  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}
