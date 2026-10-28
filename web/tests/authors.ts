//! The cluster's one list of Authors, as a test answers `/api/ui/authors`: the
//! Authors a settings read holds, and whichever others the test hands in after
//! them — records made on another device, drawn no differently (ADR-0028).

import type { AuthorEntry, SettingsView } from "../src/api/types";

/// `settings`' own Authors as the one list draws them, then `others`.
export function oneList(
  settings: SettingsView,
  ...others: AuthorEntry[]
): AuthorEntry[] {
  return [
    ...settings.authors.map((author) => ({
      id: author.id,
      display_name: author.display_name,
      name: author.name,
      email: author.email,
      holds_token: author.github_token !== null,
    })),
    ...others,
  ];
}

/// An Author made on another device of the cluster, pulled here as a record.
export const CLIENT: AuthorEntry = {
  id: "3d5f7a9b-1c2e-4f60-8a7b-9c0d1e2f3a4b",
  display_name: "Client",
  name: "Ada Lovelace",
  email: "ada@client.example",
  holds_token: true,
};
