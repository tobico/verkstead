//! What the page says about a session that has no Sandbox around it, said once.
//!
//! A Windows build has a pseudo-terminal now and no Sandbox yet, so its
//! sessions run as ordinary processes with the human's own account's reach.
//! The server works that out and says so on every Conversation it sends — one
//! value rather than a platform for the browser to reason about — and this is
//! the sentence it stands for.
//!
//! **One fact, read in three places**: above the press that starts the work,
//! beside the session's own terminal, and on a Conversation Terminal's pane.
//! The first two are what an agent is started and watched from; the third is a
//! shell in the same nothing, opened from the Timeline's header by somebody who
//! may have seen neither of the others.
//!
//! **A note rather than a refusal.** Nothing here is gated on it: the work
//! runs, and what differs is what it can reach. Which is why it is said where
//! somebody is about to press something, rather than left to a release note.

import { Show, type JSX } from "solid-js";

import type { ConversationView } from "../api/types";
import { Note } from "../notices";

/// What a session outside a Sandbox is, in the words the human reads.
///
/// Windows, and until the sandbox stage: what they are told is a platform that
/// has not got it yet rather than a product that will never have it, and what
/// it costs them today rather than a word they would have to look up.
export const UNSANDBOXED =
  "This session is not sandboxed: on Windows the agent runs with your own account's reach until the sandbox stage lands.";

/// And what a Conversation Terminal outside one is, which is the same fact
/// about a different thing.
///
/// Two sentences rather than one because the two panes are about two different
/// things in front of the human: the composer and the session pane are about an
/// agent they are about to set going or are watching, and the terminal pane is
/// about the shell they are about to type into themselves. A human on that pane
/// reading *the agent runs with your own account's reach* would have to work
/// out that it is true of their own keystrokes too.
export const UNSANDBOXED_SHELL =
  "This shell is not sandboxed: on Windows it runs with your own account's reach until the sandbox stage lands.";

/// The line itself, drawn wherever a session is started from or watched, or a
/// shell of the human's is opened — and nothing at all on a Verkstead with a
/// Sandbox, which is every other one.
///
/// `saying` is which of the two sentences this is, and [`UNSANDBOXED`] is what
/// it is when nobody said: the sessions are what there are most of, and a pane
/// about something else says so.
export function Unsandboxed(props: {
  conversation: ConversationView;
  class?: string;
  saying?: string;
}): JSX.Element {
  return (
    <Show when={props.conversation.unsandboxed}>
      <Note class={props.class}>{props.saying ?? UNSANDBOXED}</Note>
    </Show>
  );
}
