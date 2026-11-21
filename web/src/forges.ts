//! A Repo's **Forge**, as the viewer reads it: the server reads it off the
//! Repo's remote host as the Repo is drawn (ADR-0030, *Forge*), and this is
//! the one place that turns it into words.

import type { RepoEntry } from "./api/types";

/// What is said where a pull request would have been drawn, for a Repo on no
/// Forge — *No forge for git.example.org* — and `null` for one on a Forge, or
/// one whose Forge was not read.
export function unforged(repo: RepoEntry): string | null {
  const forge = repo.forge;
  if (forge === null || forge.kind === "On") return null;
  return forge.host === null ? "No forge" : `No forge for ${forge.host}`;
}

/// Why a Share on a Repo on no Forge has no Publish — the server's own
/// sentence for the refusal, said again where the press would have been.
export function unpublishable(host: string | null): string {
  return host === null
    ? "No forge is enabled, so there's nowhere to publish the share."
    : `No forge for ${host}, so there's nowhere to publish the share.`;
}
