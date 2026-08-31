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
//! So it is one button, in the sticky block under the title and above the
//! pinned cards, drawn in every state. Two lines, as a card in the sidebar is:
//! the status word over what is running. Pressing it opens the conversation
//! actions menu, and the chevron at its right edge is what says so.
//!
//! The two lines say different kinds of thing on purpose. The first is where
//! the *work* stands — a status word in bold, and the lifecycle state beside it
//! understated, because the state alone has never been the answer to "is
//! something happening?" and the status alone has never said what the work is
//! in the middle of. The second is where the *machine* stands, which is a
//! different question and a quieter one: what is running, said the way every
//! other place that says who runs a session says it — "Claude Code Fable 5 —
//! Work" rather than a profile id and `claude-fable-5`.
//!
//! That reading is not this pane's own and is not composed here: it is the
//! shared one in [`../agents`], read off what the session recorded as it
//! started. This line said the Profile's name first for as long as it was the
//! only place saying any of this; a page that says one thing three ways is a
//! page whose reader has to learn three, so the convention went with the
//! second and third sites arriving.
//!
//! The accent is spent on the two statuses that need somebody: *Waiting on you*
//! and *Blocked*. Everything else is the ordinary text colour, including
//! *Stopped* — a stop the human pressed themselves is news to nobody.

import { Show, createMemo, type JSX } from "solid-js";

import { faChevronDown } from "@fortawesome/free-solid-svg-icons";

import { Icon } from "../Icon";
import { ran, reading } from "../agents";
import { listProfiles } from "../api/client";
import type {
  AgentOutputEvent,
  ConversationView,
  ProfileEntry,
} from "../api/types";
import { useReading } from "../freshness";
import { Actions } from "./Actions";
import { WAITING_ON_CHECKS } from "./conditions";
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

/// The session running now, where there is one: the last output on the record
/// that is still being written to.
///
/// The last, because a Conversation runs one session at a time — a record is a
/// column of finished sessions with at most one live one at the end of it.
export function running(
  conversation: ConversationView,
): AgentOutputEvent | undefined {
  return conversation.timeline
    .flatMap((event) =>
      "AgentOutput" in event && event.AgentOutput.running
        ? [event.AgentOutput]
        : [],
    )
    .at(-1);
}

/// What the second line says: the agent running, or what there is instead of
/// one.
///
/// The shared reading of the three facts the session wrote down as it started —
/// the backend, the model and the Profile's name where that is what tells two
/// runs apart. All three come off the record rather than off the Pairing the
/// Conversation is configured with: what is running is what was launched, and a
/// Pairing repicked since does not change it.
///
/// `saved` is only for the last of the three — whether the Profile's name is
/// worth saying — and is `undefined` until the list has been read.
///
/// The out-of-window line is the one stop that says something a resume cannot:
/// every other stop waits for the same press, and this one waits for the same
/// press *and* an account. The short form of it, the sentence itself being the
/// resume row's.
export function agent(
  conversation: ConversationView,
  saved: ProfileEntry[] | undefined,
): string {
  const session = running(conversation);

  if (session) {
    // A session from before Verkstead wrote any of the three down reads as
    // nothing at all. There is one running and nothing true to say about it,
    // which is exactly what this says instead.
    return reading(ran(session), saved) || "Agent running";
  }

  if (conversation.resets !== null) {
    return `Out of window until ${conversation.resets}`;
  }

  return "No agent running";
}

/// The button itself: the two lines, the chevron, and the menu behind them.
export function StatusButton(props: {
  conversation: ConversationView;
}): JSX.Element {
  const said = createMemo(() => status(props.conversation));

  // The saved Profiles, for the one thing the reading needs them for: whether
  // the account behind the running session is the only one on its backend, and
  // so whether its name is said after the model. The same query the pickers
  // make, so the cache is what a second caller pays — and the reading says the
  // name while it is still in flight, saying it being the answer that can never
  // misattribute a run.
  //
  // Read here rather than passed in, so the button is whole wherever it is
  // drawn — and never in a share, which draws no status button at all and so
  // makes no request for this.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,
    freshness: { reconcile: "id" },
  }));

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
            </span>

            <span class={styles.agent}>
              {agent(props.conversation, profiles.data)}
            </span>
          </span>

          {/* Which way the menu will go, and no part of what the button says. */}
          <Icon of={faChevronDown} class={styles.mark!} />
        </>
      }
    />
  );
}
