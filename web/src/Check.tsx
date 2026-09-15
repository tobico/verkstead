//! The on/off control the settings page is written in, and the group of
//! configuration that hangs off one.
//!
//! A checkbox with nothing painted over it. The switch beside this — see
//! `Switch.tsx` — is a device setting offered inside a page about something
//! else, where a track that slides says *this is a thing you flip*; the
//! settings page is a form, and a form's answer to *is this on* is the box the
//! browser has drawn for that question since there were browsers. So this
//! keeps the browser's own box, its focus ring, its tick and the size the
//! platform draws a tick at, and adds the words beside it.
//!
//! **It keeps the switch's one rule**: it shows where things stand rather than
//! where somebody pressed. A tick is a request handed up as the state it would
//! become, the box goes straight back, and only the caller's answer arriving
//! moves it — so a save still in flight, or one that came back refused, leaves
//! the box saying what is true rather than what was asked for.
//!
//! [`Nested`] is the other half of the pattern. Configuration that only means
//! something while a checkbox is on sits inside one, under the box it belongs
//! to: indented so that the eye reads it as hanging off that line, and handed
//! to the browser as a disabled `fieldset` while the checkbox is off. Native
//! disabling rather than `readonly` — the browser greys every control inside,
//! takes them out of the tab order and refuses input, for nothing.

import type { JSX } from "solid-js";

import styles from "./Check.module.css";

/// A labelled checkbox.
///
/// `on` and `disabled` are read from the props, so the caller holds the state
/// and this draws it; `flip` is handed what the box would become, which is the
/// one thing the caller cannot work out for itself while a change is in flight.
export function Check(props: {
  /// The words beside it. They are the control's name, so they are also what a
  /// screen reader reads — markup rather than a string only, for the label that
  /// has to say more than it shows.
  label: JSX.Element;
  /// Whether it reads as ticked.
  on: boolean;
  /// Whether it will take a tick. A disabled box still says where it stands.
  disabled?: boolean;
  /// Why it will not take one, where the box is disabled for a reason of its
  /// own rather than for a save in flight. Sits on the whole row as the
  /// browser's own tooltip, so the words beside the box carry it as well.
  title?: string;
  /// What to do about a tick, given the state being asked for.
  flip: (on: boolean) => void;
}): JSX.Element {
  return (
    <label class={styles.check} title={props.title}>
      <input
        type="checkbox"
        // Solid sets `checked` as a property rather than an attribute, which is
        // what a tick the human keeps changing wants: the attribute says only
        // what the box started as.
        checked={props.on}
        disabled={props.disabled}
        // The box the browser has just ticked, and then straight back where
        // `on` says it stands — what moves it is the caller's answer arriving.
        onChange={(ev) => {
          const asked = ev.currentTarget.checked;
          ev.currentTarget.checked = props.on;

          props.flip(asked);
        }}
      />
      <span>{props.label}</span>
    </label>
  );
}

/// The configuration that hangs off a checkbox, indented under it and disabled
/// while it is off.
///
/// A `fieldset` rather than a `div` with a `disabled` written onto each control
/// inside it: one attribute in one place disables every field and every press
/// underneath, whatever the group comes to hold, and nothing here has to know
/// what those are.
export function Nested(props: {
  /// Whether what is inside means anything yet — which is the checkbox above it
  /// being on, and whatever else the section knows about its own fields.
  on: boolean;
  children: JSX.Element;
}): JSX.Element {
  return (
    <fieldset class={styles.nested} disabled={!props.on}>
      {props.children}
    </fieldset>
  );
}
