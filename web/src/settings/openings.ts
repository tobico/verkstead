//! What the settings' details pane has open, and the path it stands at.
//!
//! The same arithmetic the workbench's `openings.ts` does for a Conversation,
//! asked under `/settings` instead: the pane shows one thing, and which thing
//! it is, is in the URL. One account of what is open rather than two — a
//! selection held beside the URL is lost the moment the page is navigated away
//! from and back, and a link to a pane is a link to nothing.
//!
//! Two shapes under the settings, because there are two kinds of thing the pane
//! draws:
//!
//! - `github` — the credentials, named by a word. There is one of what
//!   Verkstead itself was told, and a word says so.
//! - `profiles/:id` — an Agent Profile, which arrives with an id of its own,
//!   and `profiles/new` for the blank form that adds one.
//!
//! The `profiles/` segment is what keeps the ids and the word-named panes
//! apart, as the workbench's `events/` does: a bare id segment would have read
//! the same as `github` the moment anything was named by a word, so the ids go
//! behind a segment of their own and can never collide with one.
//!
//! `new` stands where an id stands rather than beside it, because the blank
//! form and the filled one are one pane asked about a Profile that does not
//! exist yet — and no id the server issues is the word `new`, so the two cannot
//! be confused for each other.
//!
//! A path naming a pane this build does not have leaves the details bare, which
//! is what they are when nothing is open at all: the URL is a record of what was
//! picked rather than a promise that it is still there.

/// What the details pane on the settings page is showing.
///
/// One channel, so that opening any closes the rest — a details pane shows one
/// thing. A string for the reason the workbench's is: what is open is compared
/// against what a card would open, and two of the same selection have to be the
/// same value.
export type Opening = "github" | "profile:new" | `profile:${number}`;

/// What opens a Profile's form: its id, or `"new"` for the blank one.
export function opensProfile(which: number | "new"): Opening {
  return `profile:${which}`;
}

/// And which Profile an opening names — its id, `"new"` for the blank form, or
/// `null` where it names no Profile at all.
export function profileOpened(
  opening: Opening | null,
): number | "new" | null {
  if (opening === null || !opening.startsWith("profile:")) {
    return null;
  }

  const which = opening.slice("profile:".length);
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

  if (what === "github" && which === undefined) {
    return "github";
  }

  if (what === "profiles" && which !== undefined) {
    // The blank form, or digits and nothing else, because an id is what the
    // server issued. A segment that is neither names no Profile — and neither
    // does an id nothing is saved under, which the pane answers the same way.
    if (which === "new") {
      return opensProfile("new");
    }

    if (/^\d+$/.test(which)) {
      return opensProfile(Number(which));
    }
  }

  return null;
}
