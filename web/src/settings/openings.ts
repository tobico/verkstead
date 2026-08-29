//! What the settings' details pane has open, and the path it stands at.
//!
//! The same arithmetic the workbench's `openings.ts` does for a Conversation,
//! asked under `/settings` instead: the pane shows one thing, and which thing
//! it is, is in the URL. One account of what is open rather than two — a
//! selection held beside the URL is lost the moment the page is navigated away
//! from and back, and a link to a pane is a link to nothing.
//!
//! Nothing here is an id yet. What the settings open into is named by a word,
//! because there is one of each: the credentials are the one thing Verkstead
//! itself was told, and a word says so. The Profiles and the Repos arrive with
//! ids of their own, which is why this is a path rather than a flag.
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
export type Opening = "github";

/// Where the settings stand, which every one of their details panes is nested
/// under.
export const SETTINGS = "/settings";

/// And where one of those panes stands.
export function pathTo(opening: Opening): string {
  return `${SETTINGS}/${opening}`;
}

/// What a path says is open, or `null` where it names no details pane at all —
/// the settings' own path, or any other page.
export function openingAt(pathname: string): Opening | null {
  const segments = pathname.split("/").filter((segment) => segment !== "");
  if (segments[0] !== "settings") {
    return null;
  }

  // One segment under the settings, and nothing after it. Anything longer is no
  // path of ours, whatever it starts with.
  const [what, ...rest] = segments.slice(1);
  if (rest.length > 0) {
    return null;
  }

  return what === "github" ? "github" : null;
}
