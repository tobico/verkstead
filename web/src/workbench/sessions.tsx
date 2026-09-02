//! What the page says on a Verkstead that runs no sessions, said once.
//!
//! A Windows build has no terminal for an agent to work in — the server's
//! `terminal` module is where that ends — so nothing it is asked to start can
//! start. The server says so on every Conversation it sends, and every way into
//! a session refuses by name.
//!
//! **One sentence, in five places.** Starting the work, adopting a stage,
//! resuming, steering and sending a conflict back to a wrap-up are five presses
//! and one answer, so the words are here rather than written out again in each
//! of their refusal maps. What differs between them is nothing the human cares
//! about: there is no session to start, whichever press asked for one.
//!
//! **And the press is not offered where the page can tell.** Where a session
//! would be started from, this stands instead of the button: a button that
//! could only be refused is one the human should not have to press to find out.
//! The refusal maps are still filled in, because a page is only as fresh as its
//! last read — and because the server is what decides, not this.

import type { JSX } from "solid-js";

import type { ConversationView } from "../api/types";
import { Empty } from "../notices";

/// Whether this Verkstead has any session to start.
///
/// A fact about the build the server is running, said on every Conversation it
/// sends — so it is read off the conversation rather than worked out here.
///
/// Asked as *anything but running*, so that a platform named later is read as
/// one without sessions rather than as one with them: what this guards is a
/// press, and the wrong way round is a press that spawns nothing.
export function noSessions(conversation: ConversationView): boolean {
  return conversation.sessions !== "Run";
}

/// The words, which are the same words wherever they are said.
///
/// Windows, and not yet: a Mac runs sessions and a later stage brings them to
/// Windows too, so what the human reads is a platform that has not got them
/// rather than a product that will never have them. And what they can do about
/// it today, which is the other half of an honest refusal.
export const NO_SESSIONS =
  "Verkstead does not run sessions on Windows yet: an agent works in a terminal, and Windows has none to give it. Everything else here works — start the work on a Linux machine or a Mac in the meantime.";

/// And the line itself, drawn where the press would have been.
export function NoSessions(props: { class?: string }): JSX.Element {
  return <Empty class={props.class}>{NO_SESSIONS}</Empty>;
}
