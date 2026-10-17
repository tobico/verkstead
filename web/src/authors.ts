//! The cluster's **one list** of Authors, as every place that names one reads
//! it: a device's own Authors and its mirrors of every member's, each mirror
//! marked with the device it is at home on (ADR-0025).
//!
//! One query per device rather than one per pane, so the Git switcher, the
//! Repos pane and the Brief's Author fact are one read of it — re-read on the
//! `authors` Nudge, which a save or a removal here and a mirror rewritten off a
//! member both announce.

import { listAuthors } from "./api/client";
import type { AuthorEntry } from "./api/types";
import { useReading } from "./freshness";
import { keyOf, type Device } from "./reaching";

/// The one list of `device`'s, this device's own where none is named.
export function useAuthors(device: () => Device = () => null) {
  return useReading(() => ({
    queryKey: keyOf(device(), "authors"),
    queryFn: () => listAuthors(device()),
    freshness: { reconcile: "id" },
  }));
}

/// What an Author is called in a row: the display name somebody gave it, or
/// its git name where nobody did — and something for one with neither yet,
/// which is the one **Add author** has just opened, a row reading as nothing
/// being a row nobody could find again.
export function authorLabel(author: {
  display_name: string;
  name: string;
}): string {
  return author.display_name || author.name || "New author";
}

/// And the same with the machine a mirror is at home on after it, the way a
/// mirrored Profile's row names its machine: "Work · the-workstation".
export function authorRow(
  author: Pick<AuthorEntry, "display_name" | "name" | "device">,
): string {
  return author.device === null
    ? authorLabel(author)
    : `${authorLabel(author)} · ${author.device.name}`;
}
