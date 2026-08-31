//! Where a share reads its Conversation from: the slot in its own document.
//!
//! A share is one file with everything in it, so there is nothing to fetch and
//! nowhere to fetch it from — the record is a `<script type="application/json">`
//! the server wrote on the way out, and this is the whole of reading it. See
//! `crates/server/src/sharing.rs`, which is the other end of the same seam.
//!
//! Anything that is not a record reads as nothing at all: the untouched
//! template, which holds `null`, a slot somebody edited, a document with no slot
//! in it. The page then says it is holding no Conversation, which is the honest
//! answer and the only one it could act on.

import type { SharedConversation } from "../api/types";

/// The id the server writes the slot under, and the one this looks for. Said
/// twice across the wire and once on each side, as everything about that seam
/// is.
const SLOT = "share";

/// The Conversation this file carries, or `null` where it carries none.
export function boarded(): SharedConversation | null {
  const slot = document.getElementById(SLOT);
  if (slot === null) {
    return null;
  }

  let read: unknown;
  try {
    read = JSON.parse(slot.textContent ?? "null");
  } catch {
    return null;
  }

  // The one field that says this is a record rather than something else that
  // parsed. There is no validating the whole of it here — the shape is the
  // server's, generated from the Rust both sides share — and a share whose
  // bundle is half a Conversation is a build that disagrees with itself rather
  // than a case to draw.
  return typeof read === "object" &&
    read !== null &&
    "conversation" in read &&
    read.conversation !== null
    ? (read as SharedConversation)
    : null;
}
