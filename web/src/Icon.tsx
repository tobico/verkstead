//! One Font Awesome icon, drawn as the SVG it is.
//!
//! The marks in the viewer were characters and CSS. A `▾` typed into three
//! different files was three fonts' idea of a caret rather than one shape, and
//! the three marks a pull request's checks are said in were a disc with a tick
//! built out of two borders and a rotation. Both work, and neither is a drawing
//! anybody can change: the caret is whatever the reader's font has, and the tick
//! is arithmetic.
//!
//! So the shapes come from Font Awesome and this is the whole of what draws one.
//! An icon there is data — a viewBox and a path — so what is needed here is an
//! `svg` around it and nothing else. No runtime of Font Awesome's, no icon
//! library registered at startup, no class names swapped for shapes after the
//! page has been drawn: the caller imports the one icon it wants and the bundler
//! carries that one.
//!
//! Sized in `em` and painted in `currentColor`, so an icon inherits the line it
//! is on exactly as the character it replaced did — a caller that wants it
//! bigger or in another colour says so about the text around it, or styles the
//! class it hands down.
//!
//! Of the ones the app draws, one is the regular cut and the rest are solid:
//! Font Awesome Free has a regular `circle` and no regular `check`, `xmark` or
//! `chevron-down`, those three being Pro's. At a mark's size the difference is
//! a hairline, and the alternative was three shapes from somewhere else.

import type { JSX } from "solid-js";
import { For } from "solid-js";

import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";

import styles from "./Icon.module.css";

export function Icon(props: {
  /// The icon itself, imported by whoever wants it drawn — `faCheck` and the
  /// rest. The definition rather than a name, so an icon nobody imports is an
  /// icon the bundle does not carry.
  of: IconDefinition;

  /// What it says when it is read aloud, for an icon that is saying something
  /// the words around it do not. Most say nothing — a caret beside *New
  /// conversation* is which way the menu will go and not part of what the
  /// button is called — and an icon with no label is hidden from a screen
  /// reader rather than read out as a shape.
  label?: string;

  /// Where it stands in the line it was drawn into, and how big. Styled by
  /// whoever passes it, never here.
  class?: string;
}): JSX.Element {
  const box = (): string => `0 0 ${props.of.icon[0]} ${props.of.icon[1]}`;

  /// The path, or paths: a duotone icon is drawn in two, and Font Awesome hands
  /// either over in the same slot.
  const paths = (): string[] =>
    [props.of.icon[4]].flat().filter((path) => typeof path === "string");

  return (
    <svg
      class={[styles.icon, props.class].filter(Boolean).join(" ")}
      viewBox={box()}
      role={props.label === undefined ? undefined : "img"}
      aria-label={props.label}
      aria-hidden={props.label === undefined ? "true" : undefined}
    >
      <For each={paths()}>{(path) => <path d={path} />}</For>
    </svg>
  );
}
