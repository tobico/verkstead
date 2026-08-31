//! Every details pane of the settings page can be reached at the path that
//! opens it.
//!
//! Two things have to agree for a card on that page to lead anywhere: `pathTo`
//! says where a pane stands, and the route table says whether that path reaches
//! the page at all. They were written apart, and the share viewer was added to
//! one and not the other — so pressing its card went to *No such page*, with
//! nothing about the section itself wrong. Every test the section had mounted
//! its card and its pane directly, which is how three of them passed over it.
//!
//! So this is the seam itself, and nothing else: the real route definitions
//! (`panes` in `src/settings/SettingsPage.tsx`) under the app's own catch-all,
//! walked over the real openings. What stands in the parent route is a stand-in
//! — what is being asked is which route matched, and the page's own drawing is
//! every other file in here.

import { Route, Router } from "@solidjs/router";
import { cleanup, render } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";

import { panes } from "../src/settings/SettingsPage";
import {
  SETTINGS,
  WORDS,
  opensProfile,
  opensRepo,
  pathTo,
  type Opening,
} from "../src/settings/openings";

/// What stands where the settings page would, and what stands where the app's
/// catch-all does — two sentences, so that which of them was reached is the
/// whole of what a test here reads.
const SETTLED = "the settings page";
const MISSED = "no such page";

afterEach(cleanup);

/// Render the app's settings routes at `path`, and answer with what was drawn.
function at(path: string): string {
  window.history.pushState({}, "", path);

  render(() => (
    <Router>
      <Route path={SETTINGS} component={() => <p>{SETTLED}</p>}>
        {panes()}
      </Route>
      <Route path="*" component={() => <p>{MISSED}</p>} />
    </Router>
  ));

  return document.body.textContent ?? "";
}

describe("the settings page's own paths", () => {
  /// The page itself, which is where the middle pane stands with nothing open.
  it("opens the page with no pane on it", () => {
    expect(at(SETTINGS)).toBe(SETTLED);
  });

  /// And one per section named by a word. Walked over `WORDS` rather than
  /// written out, so a section added to that list is a case here whether or not
  /// anybody remembered to add one: which is the whole of what went wrong.
  for (const word of WORDS) {
    it(`opens the ${word} pane at the path its card leads to`, () => {
      expect(at(pathTo(word))).toBe(SETTLED);
    });
  }

  /// And the two named by an id, both of which take the word `new` in the same
  /// segment.
  for (const opening of [
    opensProfile(4),
    opensProfile("new"),
    opensRepo(2),
    opensRepo("new"),
  ] satisfies Opening[]) {
    it(`opens ${opening} at the path its card leads to`, () => {
      expect(at(pathTo(opening))).toBe(SETTLED);
    });
  }

  /// And a path naming no pane at all is still no such page, which is what says
  /// the routes above are a list rather than a wildcard.
  it("refuses a path that names no pane", () => {
    expect(at(`${SETTINGS}/nonsense`)).toBe(MISSED);
  });
});
