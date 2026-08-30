//! What just happened, said once and then gone.
//!
//! A press that reaches outside this machine has an outcome the human is owed —
//! where the share went, which pull request it could not be said on, what GitHub
//! refused. The control that made the press is the wrong place to put it: a menu
//! row is drawn from the conversation it is about and thrown away when the menu
//! closes, so an outcome held there is either lost or, worse, still standing
//! over the next conversation the human opens. That is a bug per feature rather
//! than a bug once, which is why this is part of the layout rather than part of
//! anything that uses it.
//!
//! So an outcome goes here instead: raised by whoever learned it, drawn over the
//! page wherever the human happens to be, and belonging to the moment rather
//! than to any control. One layer for the app, mounted in `App.tsx`'s shell.
//!
//! **A node rather than a sentence.** What a toast says often carries the way to
//! put it right — a link to the settings page, a link to what was just published
//! — and a message that could only be text would have the human find that for
//! themselves. Handed over as a function that makes one, because where a toast
//! is raised from is not a place a node can be built: see [`Raised`].
//!
//! **It waits while it is being read.** A toast that carries a link and goes
//! while the pointer is on the way to it is worse than no link at all, so the
//! clock stops under the pointer and while anything inside it has the focus, and
//! starts again when neither is true. And it can always be dismissed: an outcome
//! read is an outcome done with.

import { For, createSignal, onCleanup, type JSX } from "solid-js";

import styles from "./Toasts.module.css";

/// How long one stands, once nothing is reading it.
///
/// Long by the standards of these things, because what they carry here is the
/// outcome of a press that took seconds and may name a pull request or a
/// refusal: the human pressed, looked away while it worked, and is owed time to
/// look back.
const LINGER = 10_000;

/// One thing that has happened, and the number that tells it from the next.
///
/// Numbered rather than keyed by what it says: two presses that were refused the
/// same way are two outcomes, and a list keyed by their words would draw one.
///
/// **What it says is a function rather than a node.** A toast is raised from
/// wherever the outcome was learned, which is a callback rather than a component
/// — outside every owner and outside the router — and a node built there is
/// built with none of the context it needs: a link to the settings page would
/// find no router to resolve against. So the words are made where they are
/// drawn, which is inside this layer, which is inside the app.
type Raised = { id: number; said: () => JSX.Element };

const [raised, setRaised] = createSignal<Raised[]>([]);

let counted = 0;

/// Say something happened.
///
/// Called from wherever the outcome was learned — a mutation's `onSuccess`, its
/// `onError` — and never awaited: raising one is telling the human, and there is
/// nothing to hear back.
export function toast(said: () => JSX.Element): void {
  counted += 1;

  const id = counted;
  setRaised((standing) => [...standing, { id, said }]);
}

/// And take one down, which is what the clock and the dismiss both do.
function drop(id: number): void {
  setRaised((standing) => standing.filter((one) => one.id !== id));
}

/// The layer itself: everything raised and not yet gone, over whatever the human
/// is reading.
///
/// Mounted once, in the shell every page sits in. `aria-live` because that is
/// the whole of what a toast is to a screen reader — something said without
/// being asked for — and `polite` because none of it interrupts anything.
export function Toasts(): JSX.Element {
  // A layer that goes takes what it was holding with it: an outcome belongs to
  // the page it was raised on, and nothing should come back up under a page
  // that was mounted after it.
  onCleanup(() => setRaised([]));

  return (
    <div class={styles.toasts} aria-live="polite">
      <For each={raised()}>{(one) => <Toast raised={one} />}</For>
    </div>
  );
}

/// One of them, with its own clock.
function Toast(props: { raised: Raised }): JSX.Element {
  let going: ReturnType<typeof setTimeout> | undefined;

  /// Stop the clock, which is what being read does.
  const stays = (): void => {
    clearTimeout(going);
    going = undefined;
  };

  /// And start it again, from the beginning: a toast let go of has just been
  /// read, and the time it stood before the pointer arrived is not time the
  /// human spent reading it.
  const goes = (): void => {
    stays();
    going = setTimeout(() => drop(props.raised.id), LINGER);
  };

  goes();
  onCleanup(stays);

  return (
    <div
      class={styles.toast}
      role="status"
      onMouseEnter={stays}
      onMouseLeave={goes}
      onFocusIn={stays}
      onFocusOut={goes}
    >
      <div class={styles.said}>{props.raised.said()}</div>
      <button
        type="button"
        class={styles.dismiss}
        aria-label="Dismiss"
        onClick={() => drop(props.raised.id)}
      >
        ×
      </button>
    </div>
  );
}
