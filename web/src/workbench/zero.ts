//! The zero state: a Verkstead with nothing to list, read where the list is
//! read.
//!
//! It is a fact about the sidebar's list rather than about onboarding. It holds
//! whenever that list *as filtered* is empty — no unarchived Conversation, and
//! the archived ones either absent or hidden — and it stops holding the moment
//! there is something in it, whether that arrived by being created or by *Show
//! archived* being switched on. So it is decided here, off the very query the
//! sidebar draws itself from: a redirect written into the route table would be a
//! second opinion about a list only the sidebar reads.
//!
//! **The list as the sidebar draws it**, which is not quite the list the server
//! last sent: a press the server has not caught up with is already off the
//! sidebar (`eager.ts`), and a state read off the server's answer alone would
//! disagree with what is on the screen for exactly as long as a press is
//! outstanding. Archiving the last Conversation is that press, so the
//! disagreement was the one the human would have met: the compose page with the
//! sidebar still beside it saying there was nothing in it, and the frame
//! changing under them a round trip later. One list, read one way.
//!
//! Two pages ask. The workbench asks because the bare workbench has no sidebar
//! to be the left of and nothing beside it, so it sends the human to the compose
//! page; the compose page asks because it draws the sidebar or does without it,
//! and hangs the archived switch on itself where it does. One reading between
//! them: these are the sidebar's own two query keys, so a page that asks costs
//! nothing beyond what the sidebar was already fetching.
//!
//! **Whether anything is archived at all is the server's to say.** The list is
//! filtered in SQL by the switch's own setting, so an empty one cannot be told
//! apart from a list with archived rows hidden behind it — and that is exactly
//! what decides whether the pinned switch is worth drawing. The endpoint that
//! answers where the switch stands answers this beside it, so this reads one
//! fact rather than two.

import type { Accessor } from "solid-js";

import { listConversations, showingArchived } from "../api/client";
import { useReading } from "../freshness";
import { pressedRows } from "./eager";

/// What the two pages are asking about the list.
export type Zero = {
  /// Nothing has landed yet, so neither answer below is one: a page that drew
  /// the sidebar and then took it away would make a fresh install's first sight
  /// of Verkstead a flash of somebody else's empty list.
  pending: boolean;

  /// The zero state itself: the list, as filtered, has nothing in it.
  holds: boolean;

  /// And whether there is anything archived at all — what says whether the
  /// switch that could bring something back is worth drawing.
  archived: boolean;
};

/// The reading, as something a page can be built out of.
export function useZero(): Accessor<Zero> {
  // The sidebar's own two queries, under the sidebar's own keys: what comes
  // back here is the answer already in hand rather than a second fetch of it,
  // and a Nudge freshens both for everybody reading them at once.
  const conversations = useReading(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,
    freshness: { reconcile: "rank" } as const,
  }));

  const archived = useReading(() => ({
    queryKey: ["conversations", "archived"],
    queryFn: showingArchived,
    freshness: { reconcile: "id" } as const,
  }));

  return () => ({
    // Both, because both are asked at once and the second decides what the page
    // draws in the corner: a switch that arrived a moment after the page did
    // would move the one thing on it that is meant to stay put.
    pending: conversations.isPending || archived.isPending,

    // The list *as the sidebar draws it*, presses and all — see `eager.ts`.
    // Read off the server's answer alone, this would go on saying there was
    // something to list for as long as the archiving of the last Conversation
    // was still in the air: the human would land on the compose page with the
    // sidebar still beside it, reading *Nothing is being worked on yet*, and
    // watch the whole frame change under them when the read caught up. Two
    // readings of one list is what that was; there is one now.
    //
    // A list that could not be read is not an empty list. What the sidebar
    // draws then is the failure and the way to try again, which is a page with
    // something on it — and a redirect fired at a request that timed out would
    // take the human somewhere they could not read the failure from. Which is
    // why the answer's absence is asked about first rather than handed to
    // `pressedRows` as an empty list.
    holds:
      conversations.data !== undefined &&
      pressedRows(conversations.data, archived.data?.showing ?? false).length ===
        0,

    archived: archived.data?.any ?? false,
  });
}
