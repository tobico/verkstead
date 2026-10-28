//! The cluster's **one list** of Authors, as every place that names one reads
//! it: records every device holds, whichever made them, with no device mark
//! (ADR-0028). Two Authors with one name and email are two rows.
//!
//! One query per device rather than one per pane, so the Git switcher, the
//! Repos pane and the Brief's Author fact are one read of it — re-read on the
//! `authors` Nudge, which a save or a removal here and a record pulled off a
//! member both announce.

import { listAuthors } from "./api/client";
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
