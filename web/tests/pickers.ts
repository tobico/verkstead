//! Driving the listbox the app draws for itself — see `src/picking.tsx`.
//!
//! A native `<select>` is driven in one line: `fireEvent.change` with the value
//! a row sends, and `.value` back out. The listbox has neither, which is the
//! point of it — what it holds is a button and, while the rows are down, a row
//! per option — so the few lines a test needs are written here rather than at
//! each of the places a picker is asked about.
//!
//! Everything is found the way the app's own users find it: the control by the
//! label that names it, the rows through the `aria-controls` a screen reader
//! follows, and each row by the words it reads as. Nothing reaches for what a
//! row would send, because that is exactly the thing the module guarantees is
//! what is shown — a test that read the value off an attribute would be
//! asserting against its own shortcut.

import { fireEvent, screen } from "@solidjs/testing-library";

import styles from "../src/picking.module.css";

/// The control, by the label that names it — which is how a native one was
/// found too, the listbox's own being a `button` for exactly that reason.
export function picker(label: string): HTMLButtonElement {
  return screen.getByLabelText(label) as HTMLButtonElement;
}

/// What it is showing: the words of the row it is on, or its placeholder.
export function showing(label: string): string {
  return words(picker(label));
}

/// Whether the rows are down.
export function expanded(label: string): boolean {
  return picker(label).getAttribute("aria-expanded") === "true";
}

/// Drop the rows, unless they are down already, and hand back the list.
export function opened(label: string): HTMLElement {
  if (!expanded(label)) {
    fireEvent.click(picker(label));
  }

  return listbox(label);
}

/// The rows themselves, in the order they come down — the ones that can be
/// picked, which is what a picker *offers*.
///
/// The rows under the rule at the foot are not among them: they press rather
/// than pick, and a test asking what a control offers is not asking about those.
/// They are [`actions`], one function along — and the Repo dropdown's
/// repositories to clone are not among them either: see [`clonables`].
export function offered(label: string): HTMLElement[] {
  return [
    ...opened(label).querySelectorAll<HTMLElement>('[role="option"]'),
  ].filter(
    (row) =>
      !row.classList.contains(styles.action!) &&
      !row.classList.contains(styles.clonable!),
  );
}

/// And what each of them reads as.
export function rows(label: string): string[] {
  return offered(label).map(words);
}

/// The rows at the foot that press rather than pick — the Repo dropdown's
/// **Create repo** and **Open repo**, and no other control's.
export function actions(label: string): HTMLElement[] {
  return [...opened(label).querySelectorAll<HTMLElement>(`.${styles.action}`)];
}

/// And what each of those reads as.
export function actionRows(label: string): string[] {
  return actions(label).map(words);
}

/// The rows of the Repo dropdown's second group — the repositories the rest of
/// the cluster holds that the device lacks — which press rather than pick too.
export function clonables(label: string): HTMLElement[] {
  return [
    ...opened(label).querySelectorAll<HTMLElement>(`.${styles.clonable}`),
  ];
}

/// And what each of those reads as: the name, and where its origin is.
export function clonableRows(label: string): Array<[string, string]> {
  return clonables(label).map((row) => [
    words(row),
    row.querySelector(`.${styles.says}`)?.textContent ?? "",
  ]);
}

/// Pick the row that reads as `reading`.
export function pick(label: string, reading: string): void {
  const row = offered(label).find((option) => words(option) === reading);

  if (!row) {
    throw new Error(
      `nothing on the "${label}" picker reads as "${reading}" — it offers ${JSON.stringify(rows(label))}`,
    );
  }

  fireEvent.click(row);
}

/// And press the row at the foot that reads as `reading`.
export function press(label: string, reading: string): void {
  const row = actions(label).find((action) => words(action) === reading);

  if (!row) {
    throw new Error(
      `nothing at the foot of the "${label}" picker reads as "${reading}" — it holds ${JSON.stringify(actionRows(label))}`,
    );
  }

  fireEvent.click(row);
}

/// The list under an open control: the element its `aria-controls` names, which
/// is the way a screen reader reaches it as well.
export function listbox(label: string): HTMLElement {
  const named = picker(label).getAttribute("aria-controls");
  const list = named === null ? null : document.getElementById(named);

  if (!list) {
    throw new Error(`the "${label}" picker has no rows down`);
  }

  return list;
}

/// The words of one row, or of the closed control — with the arrow that says
/// which way the rows come down, and is no part of what the control says, left
/// out of it.
export function words(scope: Element): string {
  return scope.querySelector(`.${styles.words}`)?.textContent ?? "";
}
