//! The card that is pressed, wherever the app draws one: a Conversation in the
//! sidebar, and every Event on a Timeline that opens into the details pane.
//!
//! The two were the same card and neither of them knew it. The sidebar's row
//! carried its own paint and the record's cards carried theirs, in two sheets
//! that had drifted apart about what being the open one even looks like — one
//! said an accent border and the other a fill — so the app said the same thing
//! twice, differently, about cards a human reads side by side.
//!
//! So the card becomes a component and the two sheets keep what is actually
//! theirs. What a card *is* — its shape, that it can be pressed, what it does
//! under the pointer and what it does while its pane is open — is here. What
//! goes *in* one is the caller's, laid out through the class it hands down, the
//! bargain `QuietButton` and the notices already strike.
//!
//! It draws a `button`, because a card that is pressed is one. A caller whose
//! contents a button would not have asks for an `article` instead — rendered
//! markdown holds links, and the two lists and the pull request hold a heading
//! and rows, which a button would flatten to one run of text — and the card
//! takes the role, the tab stop and the two keys the button would have had for
//! nothing.
//!
//! Nothing else here is the card's own. There is no title, no mark and no
//! arrangement: a card is a surface, and every one of them has a different thing
//! standing on it.

import type { JSX } from "solid-js";
import { splitProps } from "solid-js";
import { Dynamic } from "solid-js/web";

import styles from "./CardButton.module.css";

/// What a card is told about itself, over and above the attributes it hands
/// straight on to the element.
type Card = {
  /// Whether this is the card whose pane is open. The one state a card draws
  /// about itself, and the whole of what makes it look unlike its neighbours.
  open: boolean;

  /// What pressing it does — or nothing, where the card is not a way into
  /// anything. A Brief still being written is a field rather than a target, and
  /// a card with nothing behind it is drawn without a pointer, without a tab
  /// stop and without a fill under the hand.
  press: (() => void) | null;

  /// Drawn as an `article` rather than as a `button`, for a card whose contents
  /// cannot live inside one.
  as?: "button" | "article";

  /// The caller's own class: how the card is laid out inside, and whatever the
  /// card itself has to do differently there. Styled by whoever passes it,
  /// never here.
  class?: string;

  /// Keys the caller answers itself, heard after the card has had the ones that
  /// are its own. The sidebar's arrows move a Conversation up and down the list.
  keys?: (event: KeyboardEvent) => void;

  /// Everything on the surface, which is the caller's entirely.
  children: JSX.Element;
};

/// Whatever else the element is given: the label a card's contents do not say
/// for themselves, the drag the sidebar starts on it, the menu a right-click
/// asks it for. Passed through rather than named one prop at a time, because
/// what a card is has nothing to do with any of them.
///
/// Not `classList`, which a caller would reasonably reach for and which cannot
/// work here: the card writes its own `class` after everything handed to it, and
/// what Solid does with the two of them together on one element is let the
/// second overwrite the first. A caller with a class of its own to turn on and
/// off spells it into `class`.
type Passed = Omit<
  JSX.HTMLAttributes<HTMLElement>,
  keyof Card | "onClick" | "onKeyDown" | "classList"
>;

/// One pressable card.
export function CardButton(props: Card & Passed): JSX.Element {
  const [own, rest] = splitProps(props, [
    "open",
    "press",
    "as",
    "class",
    "keys",
    "children",
  ]);

  const pressable = (): boolean => own.press !== null;

  /// An article doing a button's work, which is the one case that has to say in
  /// attributes what the element would have said by being itself.
  const standingIn = (): boolean => own.as === "article" && pressable();

  const press = (): void => own.press?.();

  return (
    <Dynamic
      component={own.as ?? "button"}
      // The caller's own first, so nothing it passes can quietly take the card
      // apart: what the card is is written after it.
      {...rest}
      type={own.as === "article" ? undefined : "button"}
      class={[
        styles.card,
        pressable() ? styles.pressable : undefined,
        own.open ? styles.open : undefined,
        own.class,
      ]
        .filter(Boolean)
        .join(" ")}
      role={standingIn() ? "button" : undefined}
      tabindex={standingIn() ? 0 : undefined}
      aria-pressed={pressable() ? own.open : undefined}
      onClick={press}
      onKeyDown={(event: KeyboardEvent) => {
        // What a button would have done for nothing: Enter and Space press it.
        if (standingIn() && (event.key === "Enter" || event.key === " ")) {
          event.preventDefault();
          press();
        }

        own.keys?.(event);
      }}
    >
      {own.children}
    </Dynamic>
  );
}
