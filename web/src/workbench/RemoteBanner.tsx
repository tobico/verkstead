//! The banner that points at Remote access: the one line above the Timeline
//! that says the human need not stay at this desk.
//!
//! **It is drawn in one window and nowhere else**: a Conversation that is
//! Grilling, with a session running, and with no Question Set on its Timeline
//! yet. That is the stretch while the first Set is being *prepared* — the first
//! moment there is nothing at the desk to do — and it closes the moment a Set
//! lands, because a Set waiting is something to do rather than a moment to be
//! told about answering from a phone.
//!
//! So it comes back at the next grilling start, and the one after, until
//! somebody says they have read it. Which is what makes the dismissal worth
//! keeping: a line that reappeared for ever would be one nobody reads.
//!
//! **And the dismissal is the server's**, read back on every load — see
//! `remoteBannerDismissed` in `../api/client`. The whole of what this banner is
//! for is a human standing at a desk who is about to pick up a phone, and a
//! dismissal kept in this browser would meet them again on the very device it
//! had just pointed them at.
//!
//! Nothing is asked for outside the window. The read is gated on the same
//! condition the banner is drawn on, so a Conversation that is not being
//! grilled costs no request at all — and a Conversation that is spends one, on
//! a key every page of the workbench shares.

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, type JSX } from "solid-js";

import { dismissRemoteBanner, remoteBannerDismissed } from "../api/client";
import type { ConversationView, RemoteBanner as Dismissal } from "../api/types";
import { useReading } from "../freshness";
import styles from "./RemoteBanner.module.css";

/// The window the banner stands in: grilling, with a session running, and
/// nothing asked yet.
///
/// Both kinds of Set count. An `UnreadableSet` is a Set this build cannot draw
/// rather than a Set that was never asked, and a banner that went on saying
/// *nothing to do here* over one would be saying something untrue about the
/// record beneath it.
export function preparingTheFirstSet(conversation: ConversationView): boolean {
  return (
    conversation.state === "Grilling" &&
    conversation.working &&
    !conversation.timeline.some(
      (event) => "QuestionSet" in event || "UnreadableSet" in event,
    )
  );
}

/// The banner, where there is one to draw.
export function RemoteBanner(props: {
  conversation: ConversationView;
}): JSX.Element {
  /// Whether the human is done with it, off the server rather than off this
  /// device. Read only inside the window: a Conversation the banner could not
  /// be drawn on has no question to ask.
  const dismissed = useReading(() => ({
    queryKey: ["remote", "banner"],
    queryFn: remoteBannerDismissed,
    enabled: preparingTheFirstSet(props.conversation),

    // One boolean, so there is nothing in it to hold on to and nothing to match
    // up: what a re-read lands on is the whole payload either way.
    freshness: { reconcile: "id" } as const,
  }));

  const queries = useQueryClient();

  const press = useMutation(() => ({
    mutationFn: dismissRemoteBanner,
    // The answer is the flag as it now stands, so it is written straight into
    // the read the banner is drawn from: the line goes at once rather than
    // after a round trip that asks the same question again. Every other device
    // reads it on its next load, which is the whole point of keeping it here.
    onSuccess: (said: Dismissal) =>
      queries.setQueryData(["remote", "banner"], said.dismissed),
  }));

  /// Drawn while the window holds, the server has said it was not dismissed,
  /// and nobody has just pressed it away.
  ///
  /// Pending counts as gone rather than as still there: a press that left the
  /// line standing for the length of a round trip would read as a press that
  /// did nothing.
  const showing = (): boolean =>
    preparingTheFirstSet(props.conversation) &&
    dismissed.data === false &&
    !press.isPending;

  return (
    <Show when={showing()}>
      <aside class={styles.banner}>
        <span>
          Nothing to do here while the first questions are written. Answer them
          from your phone: <A href="/settings/remote">Remote Access</A> puts
          this workbench on your tailnet.
        </span>

        {/* Quiet, at the end of the line: being done with it is not what the
            banner is about, and a control competing with the link would be
            offering the way out ahead of the way in. */}
        <button
          type="button"
          class={styles.done}
          onClick={() => press.mutate()}
        >
          Dismiss
        </button>
      </aside>
    </Show>
  );
}
