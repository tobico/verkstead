//! The one place a Conversation's status is said, at the head of its Timeline:
//! where the work stands, what is running in it, and everything there is to do
//! about it.
//!
//! What was here before was five pieces of chrome that had each been added
//! where there was room for it — a Done/Closed word, a *Blocked on you* badge,
//! a *Waiting on checks* label, a ⋯ hiding the actions, and a Resume block at
//! the foot of a scroll that may be long. Every one of them answered a piece of
//! the same question, and none of them was where the eye lands: the answer to
//! *what is this doing, and what can I do about it* was spread across the pane
//! and had to be assembled by whoever was reading it.
//!
//! So it is one button, in the sticky block under the pinned cards and at the
//! foot of it, drawn in every state. Pressing it opens the conversation actions
//! menu, and the chevron at its right edge is what says so.
//!
//! **Under the pinned cards rather than over them**, which is the other thing
//! it does: the sticky block ends where this button ends, and a line of chrome
//! along that edge is what says so. Over them it was the first thing in the
//! block and the boundary at the bottom was drawn by nothing.
//!
//! **One line**, which is where the *work* stands — a status word in bold, and
//! the lifecycle state beside it understated, because the state alone has never
//! been the answer to "is something happening?" and the status alone has never
//! said what the work is in the middle of.
//!
//! There was a second line under it, saying where the *machine* stood: the
//! backend, the model and the account of whatever was running. It has gone to
//! the pinned block, which now holds the running session's own card — the same
//! reading, on the card that is about that run, and the only moment the button
//! would have had anything to say is the moment that card is pinned. See
//! `Timeline.tsx`.
//!
//! Which leaves one thing the card cannot say: the out-of-window stop, where a
//! resume waits on the same press *and* an account. The word here stays
//! *Stopped*, and the whole sentence is on the resume row of the menu this
//! button drops.
//!
//! The accent is spent on the two statuses that need somebody: *Waiting on you*
//! and *Blocked*. Everything else is the ordinary text colour, including
//! *Stopped* — a stop the human pressed themselves is news to nobody.

import { Show, createMemo, type JSX } from "solid-js";

import { faChevronDown } from "@fortawesome/free-solid-svg-icons";

import { Icon } from "../Icon";
import type { ConversationView } from "../api/types";
import { Actions } from "./Actions";
import { WAITING_ON_CHECKS } from "./conditions";
import { pressed } from "./eager";
import { ENDED, STATE } from "./states";
import styles from "./StatusButton.module.css";

/// What the first line says.
export type Status = {
  /// The status word, or `null` where there is none to say and the state
  /// stands alone.
  word: string | null;
  /// The lifecycle state beside it, in the words `states.ts` spells it — and
  /// the whole of the line wherever there is no word.
  state: string;
  /// Whether the line is drawn in the accent, which is the page saying this one
  /// is waiting on the human.
  attention: boolean;
};

/// Where the work stands, in one word.
///
/// Every fact behind this is already on the Conversation: what is folded here
/// is which of them wins when more than one holds, because they overlap by
/// design — a run that stopped without the human is waiting *and* blocked *and*
/// resumable, and the button has one line to say it in. Highest precedence
/// first, which is the order they are written in below: the ones somebody has
/// to do something about, then the ones that say nothing is moving, then the
/// ones that say something is.
///
/// Draft, Done and Closed are answered before any of it. None of the statuses
/// is about a Conversation nothing is supposed to be driving, and the word for
/// where a finished one got to is the state itself — so the line is the state
/// on its own rather than the same word said twice with a colour between.
export function status(conversation: ConversationView): Status {
  const state = STATE[conversation.state];

  if (conversation.state === "Draft" || ENDED.has(conversation.state)) {
    return { word: null, state, attention: false };
  }

  // An ask left open, or driving that stopped without them — the fold the
  // sidebar's disc is drawn from, said here in words.
  if (conversation.waiting) {
    return { word: "Waiting on you", state, attention: true };
  }

  // A stop that is not the human's own. Rare above the line: nearly every one
  // of these is also something waiting on them, and is caught above.
  if (conversation.blocked_on !== null && !conversation.stopped_by_hand) {
    return { word: "Blocked", state, attention: true };
  }

  // A stop they made, or a Conversation something ought to be driving with no
  // stop on the record at all — a run that was never started, or a server that
  // came back up without it.
  if (
    conversation.stopped_by_hand ||
    (conversation.ready_to_resume && conversation.blocked_on === null)
  ) {
    return { word: "Stopped", state, attention: false };
  }

  // A wrap-up down to its checks, which is not stopped and not running: it is
  // waiting on GitHub, and there is nothing for anybody to do.
  if (conversation.waiting_on_checks) {
    return { word: WAITING_ON_CHECKS, state, attention: false };
  }

  // A session in the worktree. A session that has gone quiet says nothing
  // extra — no idle word and no ring: it is still what the run is doing, and a
  // second word for it would be the button reporting on the agent's typing.
  if (conversation.working) {
    return { word: "Running", state, attention: false };
  }

  // And nothing running, with something of Verkstead's own still holding it:
  // the moment between one step of a backlog and the next.
  if (conversation.driven) {
    return { word: "Driven", state, attention: false };
  }

  return { word: null, state, attention: false };
}

/// The button itself: the line, the chevron, and the menu behind them.
export function StatusButton(props: {
  conversation: ConversationView;
}): JSX.Element {
  // Off the Conversation as the page is drawing it rather than as the server
  // last described it: a close pressed a moment ago has this line reading
  // *Closed* at once, and the menu behind the button offering the rows a closed
  // Conversation has. See `eager.ts`, and `Actions.tsx`, where the rows do the
  // same to what they are handed.
  const said = createMemo(() => status(pressed(props.conversation)));

  return (
    <Actions
      conversation={props.conversation}
      class={styles.status!}
      trigger={
        <>
          <span class={styles.what}>
            <span
              class={
                said().attention
                  ? `${styles.standing} ${styles.attention}`
                  : styles.standing
              }
            >
              {/* The status in bold and the state understated beside it, which
                  is the pattern a card in the sidebar draws its name and its
                  repo in. Where there is no status word the state takes the
                  bold and stands alone: a line whose whole content was drawn as
                  a subtitle would read as a caption for nothing. */}
              <span class={styles.title}>{said().word ?? said().state}</span>
              <Show when={said().word !== null}>
                <span class={styles.state}>{said().state}</span>
              </Show>

              {/* And what a Cleanup has taken, where it has been through this
                  record: the word for the state it left the Conversation in,
                  in the same understated voice the lifecycle state is said in
                  and last on the line, because it is a fact about the record
                  rather than about the work. Drawn only once a trim has
                  happened — nothing here ever says one is coming. */}
              <Show when={props.conversation.trimmed}>
                <span class={styles.trimmed}>Trimmed</span>
              </Show>
            </span>
          </span>

          {/* Which way the menu will go, and no part of what the button says. */}
          <Icon of={faChevronDown} class={styles.mark!} />
        </>
      }
    />
  );
}
