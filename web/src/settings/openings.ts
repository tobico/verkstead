//! What the settings' details pane has open, and the path it stands at.
//!
//! The same arithmetic the workbench's `openings.ts` does for a Conversation,
//! asked under `/settings` instead: the pane shows one thing, and which thing
//! it is, is in the URL. One account of what is open rather than two — a
//! selection held beside the URL is lost the moment the page is navigated away
//! from and back, and a link to a pane is a link to nothing.
//!
//! Three shapes under the settings, because there are three kinds of thing the
//! pane draws:
//!
//! - `github`, `build-cache` and `share-viewer` — the credentials, the shared
//!   Rust build cache and where the share viewer is hosted, each named by a
//!   word. There is one of each of them, and a word says so.
//! - `profiles/:id` — an Agent Profile, which arrives with an id of its own,
//!   and `profiles/new` for the blank form that adds one.
//! - `repos/:id` — a registered Repo, opened; and `repos/new` for the path
//!   another is registered by.
//!
//! The `profiles/` and `repos/` segments are what keep the ids and the
//! word-named panes apart, as the workbench's `events/` does: a bare id segment
//! would have read the same as `github` the moment anything was named by a
//! word, so the ids go behind a segment of their own and can never collide with
//! one.
//!
//! `new` stands where an id stands rather than beside it, because the blank
//! form and the filled one are one pane asked about a Profile that does not
//! exist yet — and no id the server issues is the word `new`, so the two cannot
//! be confused for each other. The Repos' form stands in the same place for the
//! same reason, beside the ids of the ones that are registered.
//!
//! A path naming a pane this build does not have leaves the details bare, which
//! is what they are when nothing is open at all: the URL is a record of what was
//! picked rather than a promise that it is still there.

/// The openings named by a word rather than by an id: the credentials, the
/// shared Rust build cache, and where the share viewer is hosted — the things
/// there is exactly one of on this page.
///
/// A list rather than a word written wherever one is needed, because three
/// separate things read it and all three have to agree: the [`Opening`] below is
/// made of it, [`openingAt`] turns a path into one, and the routes the app
/// declares under `/settings` decide whether that path reaches this page at all.
/// Written apart, a fourth section can be added to the page, given a card and a
/// pane and a path, and answer that path with *No such page* — a nested route
/// with no matching child falls to the catch-all, and nothing about the section
/// itself is wrong. Which is what happened to the share viewer.
///
/// So the app writes those routes from this — see `panes` in `SettingsPage.tsx`
/// — and a word added here arrives with the route that reaches it.
export const WORDS = ["github", "build-cache", "share-viewer"] as const;

/// What the details pane on the settings page is showing.
///
/// One channel, so that opening any closes the rest — a details pane shows one
/// thing. A string for the reason the workbench's is: what is open is compared
/// against what a card would open, and two of the same selection have to be the
/// same value.
export type Opening =
  | (typeof WORDS)[number]
  | "repo:new"
  | `repo:${number}`
  | "profile:new"
  | `profile:${number}`;

/// Whether a segment is one of the words above.
function worded(what: string | undefined): what is (typeof WORDS)[number] {
  return WORDS.some((word) => word === what);
}

/// What opens a Profile's form: its id, or `"new"` for the blank one.
export function opensProfile(which: number | "new"): Opening {
  return `profile:${which}`;
}

/// And which Profile an opening names — its id, `"new"` for the blank form, or
/// `null` where it names no Profile at all.
export function profileOpened(opening: Opening | null): number | "new" | null {
  return named("profile", opening);
}

/// What opens a Repo: its id, or `"new"` for the form another is registered by.
export function opensRepo(which: number | "new"): Opening {
  return `repo:${which}`;
}

/// And which Repo an opening names, read the way a Profile's is.
export function repoOpened(opening: Opening | null): number | "new" | null {
  return named("repo", opening);
}

/// Which of one kind of thing an opening names: its id, `"new"`, or `null`
/// where the opening is about something else entirely.
///
/// The Profiles and the Repos are the same shape — a segment, then an id or the
/// word — so they are read by the one function. Two copies of this would be two
/// places for the two to drift apart.
function named(kind: string, opening: Opening | null): number | "new" | null {
  const prefix = `${kind}:`;
  if (opening === null || !opening.startsWith(prefix)) {
    return null;
  }

  const which = opening.slice(prefix.length);
  return which === "new" ? "new" : Number(which);
}

/// Where the settings stand, which every one of their details panes is nested
/// under.
export const SETTINGS = "/settings";

/// And where one of those panes stands.
export function pathTo(opening: Opening): string {
  const profile = profileOpened(opening);
  if (profile !== null) {
    return `${SETTINGS}/profiles/${profile}`;
  }

  const repo = repoOpened(opening);
  if (repo !== null) {
    return `${SETTINGS}/repos/${repo}`;
  }

  return `${SETTINGS}/${opening}`;
}

/// What a path says is open, or `null` where it names no details pane at all —
/// the settings' own path, or any other page.
export function openingAt(pathname: string): Opening | null {
  const segments = pathname.split("/").filter((segment) => segment !== "");
  if (segments[0] !== "settings") {
    return null;
  }

  // What stands under the settings, which is a segment and at most one thing
  // named by it. Anything longer is no path of ours, whatever it starts with.
  const [what, which, ...rest] = segments.slice(1);
  if (rest.length > 0) {
    return null;
  }

  if (worded(what) && which === undefined) {
    return what;
  }

  if (what === "profiles" && which !== undefined) {
    const profile = which === "new" ? "new" : id(which);
    return profile === null ? null : opensProfile(profile);
  }

  if (what === "repos" && which !== undefined) {
    const repo = which === "new" ? "new" : id(which);
    return repo === null ? null : opensRepo(repo);
  }

  return null;
}

/// The id a segment names, or `null` where it names none.
///
/// Digits and nothing else, because an id is what the server issued. A segment
/// that is anything else names no Profile and no Repo — and neither does an id
/// nothing is saved under, which the panes answer the same way.
function id(segment: string): number | null {
  return /^\d+$/.test(segment) ? Number(segment) : null;
}
