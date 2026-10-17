//! The cluster's one list of Authors, as a test answers `/api/ui/authors`: the
//! Authors a settings read holds, with no device mark, and whichever mirrors of
//! a member's the test hands in after them.

import type { AuthorEntry, RowDevice, SettingsView } from "../src/api/types";

/// The machine a mirror in these tests is at home on.
export const THE_WORKSTATION: RowDevice = {
  id: "aa00bb11cc22dd33ee44ff5566778899",
  name: "the-workstation",
  os: "macOS 15.1",
  reachable: true,
};

/// `settings`' own Authors as the one list draws them, then `mirrors`.
export function oneList(
  settings: SettingsView,
  ...mirrors: AuthorEntry[]
): AuthorEntry[] {
  return [
    ...settings.authors.map((author) => ({
      id: author.id,
      display_name: author.display_name,
      name: author.name,
      email: author.email,
      holds_token: author.github_token !== null,
      device: null,
    })),
    ...mirrors,
  ];
}

/// A member's Author, at home on [`THE_WORKSTATION`].
export const MIRRORED: AuthorEntry = {
  id: "3d5f7a9b-1c2e-4f60-8a7b-9c0d1e2f3a4b",
  display_name: "Client",
  name: "Ada Lovelace",
  email: "ada@client.example",
  holds_token: true,
  device: THE_WORKSTATION,
};
