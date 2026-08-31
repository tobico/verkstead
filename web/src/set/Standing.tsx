//! Where a still-waiting Set stands: whether an agent is listening, and the
//! offer to close it if none ever will be again.
//!
//! The two sit together because one is why the other exists — the badge is the
//! title of a small menu, and locking is the one thing in it. A menu rather
//! than a bare button so the offer is out of the way until it is asked for:
//! locking is almost never the right thing to do to a Set, and it was a
//! thumb's width from the questions. It is confirmed besides: it is the only
//! thing on this page that cannot be taken back.

import { faChevronDown } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { Show, createMemo, createSignal } from "solid-js";

import { Icon } from "../Icon";
import { Menu } from "../Menu";
import { Modal } from "../Modal";
import { lockSet } from "../api/client";
import type { Locked, Liveness } from "../api/types";
import { ErrorLine, Note } from "../notices";
import page from "./Sheet.module.css";
import styles from "./Standing.module.css";
import { clearDraft } from "./sheet";

/// What the badge says. The word that colours it is the Liveness itself, which
/// is the class on the span.
///
/// Worded about the agent rather than about the connection: what the human
/// wants to know before answering is whether anyone is still on the other end.
/// A Deferred Ask answers that outright — nobody, by design — where
/// "disconnected" would say that somebody had gone.
export const BADGE: Record<Liveness, string> = {
  waiting: "agent waiting",
  disconnected: "agent disconnected",
  deferred: "no agent waiting",
};

/// What the human is asked before a Set is closed unanswered.
///
/// Named rather than written into the dialog so that the one thing it must not
/// stop saying — that this cannot be taken back — can be held to. Locking is
/// the only irreversible act in the whole UI.
export const LOCK_WARNING =
  "It stops waiting on you for good and stands on its Conversation's timeline " +
  "with no Response. An agent still waiting on it is told the Set was " +
  "locked. This cannot be undone.";

/// The badge, and the one thing to do about a Set nobody is coming back for,
/// folded behind it as a menu. Drawn on the provenance line, which the
/// stylesheet puts it at the far end of.
///
/// The menu itself is the [`Menu`](../Menu.tsx) every dropdown here is: the
/// badge is what it drops from, and locking is the whole of what it drops. The
/// confirmation is the [`Modal`](../Modal.tsx) every sheet drawn over the page
/// is, which is what gives it Escape and a press away from it as ways of saying
/// no.
export function Standing(props: {
  id: number;
  liveness: Liveness;
}): JSX.Element {
  // `true` while the human is being asked to confirm. Nothing is locked until
  // they answer it.
  const [confirming, setConfirming] = createSignal(false);

  // The menu's own way to shut, held here so the row can take the menu back on
  // its way to the confirmation.
  let shut = (): void => {};

  const queries = useQueryClient();

  const lock = useMutation(() => ({
    mutationFn: () => lockSet(props.id),
    onSuccess: (outcome: Locked) => {
      if (outcome !== "Closed") {
        return;
      }

      // A Set that can never take a Response has no use for a half-filled
      // sheet.
      clearDraft(props.id);

      // And the page stays where it is, read back as the Set nobody ever
      // answered: it was not discarded, it was closed, and seeing it closed on
      // its own Timeline is the confirmation that nothing was lost. Everything
      // is invalidated because the row this Set is on has changed as well as
      // the Set itself.
      void queries.invalidateQueries();
    },
  }));

  const close = () => {
    setConfirming(false);
    lock.mutate();
  };

  const failed = createMemo(() =>
    unlocked(lock.data, lock.error as Error | null),
  );

  return (
    <>
      <Menu
        class={styles.standing!}
        name="How this Set stands"
        disabled={lock.isPending}
        closer={(close) => (shut = close)}
        trigger={
          <>
            <span class={`${styles.liveness} ${styles[props.liveness]}`}>
              {lock.isPending ? "Locking…" : BADGE[props.liveness]}
            </span>
            {/* Which way the menu will go, and no part of what the badge
                says. */}
            <Icon of={faChevronDown} class={styles.standingMark} />
          </>
        }
      >
        {() => (
          <button
            type="button"
            role="menuitem"
            class={styles.lock}
            onClick={() => {
              shut();
              setConfirming(true);
            }}
          >
            Lock unanswered
          </button>
        )}
      </Menu>
      <Show when={failed()}>
        {(said) => (
          <ErrorLine inline class={styles.failure}>
            {said()}
          </ErrorLine>
        )}
      </Show>
      {/* The one irreversible thing on the page, so it is asked about in as many
          words — including that it cannot be undone, which is what tells this
          dialog apart from the one before a submit. Every way out of it that is
          not the one button leaves the Set pending, which is the safe way round
          for the only act here that cannot be taken back. */}
      <Modal
        class={page.confirm!}
        open={confirming()}
        close={() => setConfirming(false)}
        labelledBy="lock-title"
      >
        <p id="lock-title" class={page.confirmTitle}>
          Lock this Set unanswered?
        </p>
        <Note class={page.caveat}>{LOCK_WARNING}</Note>
        <div class={page.confirmActions}>
          <button
            type="button"
            class={page.secondary}
            onClick={() => setConfirming(false)}
          >
            Keep it pending
          </button>
          <button type="button" onClick={close}>
            Lock unanswered
          </button>
        </div>
      </Modal>
    </>
  );
}

/// Why the Set was not locked, when it was not. A Set that was says nothing
/// here — the page is redrawing as the record of a Set nobody answered, and
/// that is the whole of what there is to say about it.
function unlocked(
  outcome: Locked | undefined,
  error: Error | null,
): string | null {
  if (error !== null) {
    return `The Set was not locked: ${error.message}`;
  }

  switch (outcome) {
    case undefined:
    case "Closed":
      return null;
    case "AlreadyAnswered":
      return "This Set was answered while this page was open, so it was not locked: it stands as the decision that was made.";
    case "AlreadyLocked":
      return "This Set has already been locked.";
    case "NoSuchSet":
      return "This Set is no longer here.";
  }
}

/// The badge on its own, with nothing behind it: how a Set stood, said to
/// somebody who cannot do anything about it.
///
/// What a read-only sheet draws in place of [`Standing`] — the share, where a
/// colleague is reading a Conversation out of a file. The words and the colour
/// are the same, because it is the same fact: this Set was waiting on somebody
/// when the record was made. What is gone is the menu, which offers the one act
/// on this page that cannot be taken back and has no server here to take it.
export function Badge(props: { liveness: Liveness }): JSX.Element {
  return (
    <span class={`${styles.standing} ${styles.liveness} ${styles[props.liveness]}`}>
      {BADGE[props.liveness]}
    </span>
  );
}
