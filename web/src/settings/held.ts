//! The paths a save carries when the form in front of the human is not about
//! them.
//!
//! One request writes the whole of `config.yaml`, so every section's save sends
//! every value in it — the author, the build cache, the share viewer and now the
//! two lists of paths. A section that left a list out would be a section that
//! emptied it: what is sent is what the file holds afterwards.
//!
//! Only the settings' own go back. The installation's entries come back on every
//! read labelled as the unit's word, they were never in this file, and sending
//! one would be asking the server to write down a flag — so they are filtered
//! out here, once, rather than in each of the sections that has to ride them
//! along.
//!
//! A bind goes back in the grammar it was written in: `/abs/path` for one every
//! sandbox gets, and `name=/abs/path` for one Repo's own. The view takes that
//! apart so a page can draw the two halves; this puts it back together.

import type { SettingsView } from "../api/types";

/// The two lists as they stand, ready to be spread into a save.
///
/// Empty lists where the read has not landed, which is the same thing the server
/// would write for a Verkstead nobody has told anything.
export function heldPaths(told: SettingsView | undefined): {
  watched_paths: string[];
  sandbox_binds: string[];
} {
  const paths = told?.paths;

  return {
    watched_paths: (paths?.watched ?? [])
      .filter((entry) => entry.source === "Settings")
      .map((entry) => entry.path),
    sandbox_binds: (paths?.binds ?? [])
      .filter((entry) => entry.source === "Settings")
      .map((entry) => (entry.repo ? `${entry.repo}=${entry.path}` : entry.path)),
  };
}
