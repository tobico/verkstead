//! A Repo's **Forge**, as the viewer reads it: the server reads it off the
//! Repo's remote host as the Repo is drawn (ADR-0030, *Forge*), and this is
//! the one place that turns it into words.

import type { AuthorEntry, RepoEntry } from "./api/types";
import { authorLabel } from "./authors";

/// The built-in GitHub plugin's uuid, which its tokens are kept under: what
/// **Create repo** makes a repository on, while GitHub is the one Forge that
/// can make one.
export const GITHUB = "b357e557-3716-4ea6-a3a8-d904c92c5eac";

/// Whether `author` holds a token for the Forge `forge` on the device it was
/// read from.
export function holdsToken(author: AuthorEntry, forge: string): boolean {
  return author.tokens.includes(forge);
}

/// The URL of the Repo's `origin` remote — what a press that clones it on
/// another machine clones from.
///
/// It sits beside the Forge because the two are the one reading of the remote:
/// `null` is a Repo whose remote was not read, or one with no origin, which is
/// a repository there is nothing to clone from either way.
export function originOf(repo: RepoEntry | null | undefined): string | null {
  return repo?.forge?.origin ?? null;
}

/// What is said where a pull request would have been drawn, for a Repo on no
/// Forge — *No forge for git.example.org* — and `null` for one on a Forge, or
/// one whose Forge was not read.
export function unforged(repo: RepoEntry): string | null {
  const on = repo.forge?.on ?? null;
  if (on === null || on.kind === "On") return null;
  return on.host === null ? "No forge" : `No forge for ${on.host}`;
}

/// Why a Share on a Repo on no Forge has no Publish — the server's own
/// sentence for the refusal, said again where the press would have been.
export function unpublishable(host: string | null): string {
  return host === null
    ? "No forge is enabled, so there's nowhere to publish the share."
    : `No forge for ${host}, so there's nowhere to publish the share.`;
}

/// Why a Share on this Repo has no Publish — no Forge, or one that declares
/// no `publish` — in the server's own sentence for the refusal, and `null`
/// where it has one.
export function unpublished(repo: RepoEntry): string | null {
  const on = repo.forge?.on ?? null;
  if (on === null) return null;
  if (on.kind === "Unmatched") return unpublishable(on.host);
  return on.capabilities.includes("publish") ? null : publishless(on.name);
}

/// Why a Share on a Repo whose Forge declares no `publish` has no Publish.
export function publishless(forge: string): string {
  return `${forge} can't publish shares, so there's nowhere to publish this one.`;
}

/// What a Repo's row says where its Author holds no token for the Forge it is
/// on — *No GitHub token for Work* — which leaves the Forge unasked: no
/// wrap-up there. `null` where it holds one, where the Repo is on no Forge,
/// where its Author is still to pick or gone (the row says that instead),
/// where there is no Author at all (the Git card says that), and while
/// `authors` is unread.
export function untokened(
  repo: RepoEntry,
  authors: AuthorEntry[] | undefined,
): string | null {
  const on = repo.forge?.on ?? null;
  if (on === null || on.kind !== "On" || authors === undefined) {
    return null;
  }

  const author =
    repo.author === null
      ? authors.length === 1
        ? authors[0]
        : undefined
      : authors.find(({ id }) => id === repo.author);

  if (author === undefined || holdsToken(author, on.uuid)) return null;

  return `No ${on.name} token for ${authorLabel(author)}`;
}

/// What the Repo's Forge calls a pull request — *pull request*, *merge
/// request* — lowercase, as it sits in a sentence. GitHub's word, and the
/// documents', where the Forge was not read or the Repo is on none.
export function nounOf(repo: RepoEntry | null | undefined): string {
  const on = repo?.forge?.on;
  return on && on.kind === "On" ? on.noun : "pull request";
}

/// The noun at the head of a sentence or a label: *Merge request*.
export function opening(noun: string): string {
  return noun.charAt(0).toUpperCase() + noun.slice(1);
}

/// The noun in a heading, which is Title Case: *Merge Request*.
export function headed(noun: string): string {
  return noun.replace(/\b\w/g, (first) => first.toUpperCase());
}

/// The noun with its indefinite article: *a pull request*, *an issue*.
export function aNoun(noun: string): string {
  return /^[aeiou]/i.test(noun) ? `an ${noun}` : `a ${noun}`;
}

/// And more than one: *merge requests*.
export function nouns(noun: string): string {
  return noun.endsWith("s") ? noun : `${noun}s`;
}

/// The name of the Repo's Forge — *GitHub* — and GitHub's where it was not
/// read or the Repo is on none.
export function forgeName(repo: RepoEntry | null | undefined): string {
  const on = repo?.forge?.on;
  return on && on.kind === "On" ? on.name : "GitHub";
}

/// `text`, written about a pull request, said in the word the Repo's Forge
/// calls one: *a pull request* and *pull requests* and *Pull request* each in
/// the noun's own article, plural and capital. GitHub's word comes back as it
/// was written.
export function spoken(text: string, noun: string): string {
  if (noun === "pull request") return text;

  return text
    .replace(/\b([Aa]) pull request\b/g, (_, a: string) => {
      const said = aNoun(noun);
      return a === "A" ? opening(said) : said;
    })
    .replace(/\bpull requests\b/g, nouns(noun))
    .replace(/\bPull requests\b/g, opening(nouns(noun)))
    .replace(/\bpull request\b/g, noun)
    .replace(/\bPull request\b/g, opening(noun));
}
