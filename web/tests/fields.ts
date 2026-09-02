//! Driving the path field's browse dropdown — see `src/PathField.tsx`.
//!
//! Written here rather than at each of the places a path is written, for the
//! reason the listbox's own driving is — see `pickers.ts`, whose shape this
//! follows. A field's rows are not a value to read off an element: they are the
//! server's answer about a directory, drawn as rows a finger can hit, and a
//! test that reached past them would be asserting against its own shortcut.
//!
//! Everything is found the way the field's own human finds it: the box by the
//! label that names it, the press beside it by what a screen reader calls it,
//! the rows through the `aria-controls` that same reader follows, and each row
//! by the words it reads as.
//!
//! Every one of these takes the label, because a page holds more than one of
//! these fields: the settings' Paths section draws two, and each is a browse of
//! its own.

import { fireEvent, screen } from "@solidjs/testing-library";

import type { BrowseScope } from "../src/api/types";
import styles from "../src/picking.module.css";

/// The box itself, by the label that names it.
export function pathField(label: string): HTMLInputElement {
  return screen.getByLabelText(label) as HTMLInputElement;
}

/// What it holds — typed into or tapped together, which nothing here can tell
/// apart and nothing about it should.
export function held(label: string): string {
  return pathField(label).value;
}

/// Whether the rows are down.
export function browsing(label: string): boolean {
  return pathField(label).getAttribute("aria-expanded") === "true";
}

/// Drop them, unless they are down already.
export function browse(label: string): void {
  if (browsing(label)) return;

  const press = pathField(label).parentElement?.querySelector<HTMLElement>(
    '[aria-label="Browse"]',
  );

  if (!press) {
    throw new Error(`the "${label}" field has nothing to browse with`);
  }

  fireEvent.click(press);
}

/// The list under an open field: the element its `aria-controls` names, which is
/// the way a screen reader reaches it as well.
export function listed(label: string): HTMLElement {
  const named = pathField(label).getAttribute("aria-controls");
  const list = named === null ? null : document.getElementById(named);

  if (!list) {
    throw new Error(`the "${label}" field has no rows down`);
  }

  return list;
}

/// The rows themselves, in the order they come down — none at all while the
/// level is still being read, which is a state to wait through rather than an
/// error.
export function offered(label: string): HTMLElement[] {
  return [...listed(label).querySelectorAll<HTMLElement>('[role="option"]')];
}

/// And what each of them reads as.
export function rows(label: string): string[] {
  return offered(label).map(words);
}

/// The row the keyboard is on, by the `aria-activedescendant` that says so.
export function walked(label: string): string {
  const on = pathField(label).getAttribute("aria-activedescendant");
  const row = on === null ? null : document.getElementById(on);

  if (!row) {
    throw new Error(`the "${label}" field says the keyboard is on no row`);
  }

  return words(row);
}

/// Tap the row that reads as `reading`.
export function tap(label: string, reading: string): void {
  const row = offered(label).find((option) => words(option) === reading);

  if (!row) {
    throw new Error(
      `no row of the "${label}" field reads as "${reading}" — it offers ${JSON.stringify(rows(label))}`,
    );
  }

  fireEvent.click(row);
}

/// And the words of one row, without the arrow that says which way it goes.
function words(row: Element): string {
  return row.querySelector(`.${styles.words}`)?.textContent ?? "";
}

/// And where one level of a browse is asked for, spelled as the client spells
/// it: the scope in the query beside the path, and no path at all for the field
/// standing empty.
///
/// Here rather than in each test, because it is the one thing about a browse
/// that is neither the dropdown nor the server: a test serving a level has to
/// name the request the field is about to make.
export function listingAt(
  path: string | null,
  scope: BrowseScope = "anywhere",
): string {
  const asking = new URLSearchParams({ scope });

  if (path !== null) {
    asking.set("path", path);
  }

  return `/api/ui/directories?${asking}`;
}
