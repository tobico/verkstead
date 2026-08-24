//! Where a still-waiting Set stands: whether an agent is listening, and the
//! offer to close it if none ever will be again.
//!
//! The two sit together because one is why the other exists — the badge is the
//! title of a small menu, and archiving is the one thing in it. A menu rather
//! than a bare button so the offer is out of the way until it is asked for:
//! archiving is almost never the right thing to do to a Set, and it was a
//! thumb's width from the questions. It is confirmed besides: it is the only
//! thing on this page that cannot be taken back.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { Show, createMemo, createSignal, onCleanup } from "solid-js";

import { archiveSet } from "../api/client";
import type { Archived, Liveness } from "../api/types";
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
/// stop saying — that this cannot be taken back — can be held to. Archiving is
/// the only irreversible act in the whole UI.
export const ARCHIVE_WARNING =
  "It stops waiting on you for good and stands on its Conversation's timeline " +
  "with no Response. An agent still waiting on it is told the Set was " +
  "archived. This cannot be undone.";

/// The badge, and the one thing to do about a Set nobody is coming back for,
/// folded behind it as a menu. Drawn on the provenance line, which the
/// stylesheet puts it at the far end of.
export function Standing(props: {
  id: number;
  liveness: Liveness;
}): JSX.Element {
  // `true` while the menu hangs open under the badge.
  const [open, setOpen] = createSignal(false);

  // `true` while the human is being asked to confirm. Nothing is archived until
  // they answer it.
  const [confirming, setConfirming] = createSignal(false);

  const queries = useQueryClient();

  const archive = useMutation(() => ({
    mutationFn: () => archiveSet(props.id),
    onSuccess: (outcome: Archived) => {
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
    archive.mutate();
  };

  const failed = createMemo(() =>
    unarchived(archive.data, archive.error as Error | null),
  );

  // The way out that needs no aim: a menu drawn over the page has to be
  // dismissible from the keyboard. The other way — tapping the page — is the
  // backdrop's, so the tap taking the menu back cannot also press something on
  // the page underneath.
  const escape = (ev: KeyboardEvent) => {
    if (ev.key === "Escape") {
      setOpen(false);
    }
  };

  document.addEventListener("keydown", escape);
  onCleanup(() => document.removeEventListener("keydown", escape));

  return (
    <>
      <span class="standing">
        <button
          type="button"
          class="standing-trigger"
          aria-expanded={open() ? "true" : "false"}
          aria-controls="standing-actions"
          aria-haspopup="menu"
          disabled={archive.isPending}
          onClick={() => setOpen(!open())}
        >
          <span class={`liveness ${props.liveness}`}>
            {archive.isPending ? "Archiving…" : BADGE[props.liveness]}
          </span>
          {/* Which way the menu will go, and no part of what the badge says. */}
          <span class="standing-mark" aria-hidden="true">
            ▾
          </span>
        </button>
        <Show when={open()}>
          <div
            class="standing-backdrop"
            aria-hidden="true"
            onClick={() => setOpen(false)}
          />
          <div class="standing-actions" id="standing-actions" role="menu">
            <button
              type="button"
              role="menuitem"
              class="archive"
              onClick={() => {
                setOpen(false);
                setConfirming(true);
              }}
            >
              Archive unanswered
            </button>
          </div>
        </Show>
      </span>
      <Show when={failed()}>{(said) => <span class="error">{said()}</span>}</Show>
      {/* The one irreversible thing on the page, so it is asked about in as many
          words — including that it cannot be undone, which is what tells this
          dialog apart from the one before a submit. */}
      <Show when={confirming()}>
        <div class="confirm-backdrop">
          <div
            class="confirm"
            role="dialog"
            aria-modal="true"
            aria-labelledby="archive-title"
          >
            <p id="archive-title">Archive this Set unanswered?</p>
            <p class="note">{ARCHIVE_WARNING}</p>
            <div class="confirm-actions">
              <button
                type="button"
                class="secondary"
                onClick={() => setConfirming(false)}
              >
                Keep it pending
              </button>
              <button type="button" onClick={close}>
                Archive unanswered
              </button>
            </div>
          </div>
        </div>
      </Show>
    </>
  );
}

/// Why the Set was not archived, when it was not. A Set that was says nothing
/// here — the page is redrawing as the record of a Set nobody answered, and
/// that is the whole of what there is to say about it.
function unarchived(
  outcome: Archived | undefined,
  error: Error | null,
): string | null {
  if (error !== null) {
    return `The Set was not archived: ${error.message}`;
  }

  switch (outcome) {
    case undefined:
    case "Closed":
      return null;
    case "AlreadyAnswered":
      return "This Set was answered while this page was open, so it was not archived: it stands as the decision that was made.";
    case "AlreadyArchived":
      return "This Set has already been archived.";
    case "NoSuchSet":
      return "This Set is no longer here.";
  }
}
