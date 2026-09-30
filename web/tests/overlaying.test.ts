//! What the presses have said about a Conversation, once more than one device
//! is in play.
//!
//! The overlay is keyed by the device and the id together, because ids are each
//! device's own and collide by construction (see `src/reaching.ts`). What is
//! asked here is the half of that which is not a lookup: the sidebar's rows are
//! walked out of the *whole* table, so a press made on a member's Conversation
//! has to be told from one made on this device's Conversation of the same
//! number — and told apart by what the entry says rather than by an id taken
//! back off the row it carries.
//!
//! A file of its own because the table is the module's: there is one of it for
//! the app, nothing empties it, and a suite that mounted pages beside these
//! would be reading presses those pages made.

import { describe, expect, it } from "vitest";

import type { ConversationEntry, ConversationView } from "../src/api/types";
import type { Device } from "../src/reaching";
import { eagerly, pressed, pressedRows, rowFor } from "../src/workbench/eager";
import grilling from "./fixtures/conversation-grilling.json" with {
  type: "json",
};

/// The member, by the Device Id a URL carries.
const MEMBER = "8f2a1c0b4d6e7f902b13c4d5e6f70819";

/// The Conversation both devices have, which is the point of the exercise:
/// every Verkstead issues a 3, so nothing but the device tells the two apart.
const BOTH = grilling as ConversationView;

/// This device's row for it, as the sidebar's list carries one.
const HERE: ConversationEntry = rowFor(BOTH);

/// Unarchive it on `device`, which is the one press that asks for a row the
/// server's own list does not carry — and so the one the put-back is about.
///
/// Landed rather than in flight: what holds the entry is the read count, and
/// nothing here reads anything.
function unarchived(device: Device, view: ConversationView): void {
  eagerly({
    device,
    conversation: view.id,
    says: { archived: false, row: rowFor(view) },
    post: () => Promise.resolve("Unarchived"),
    refusal: () => "",
    fell: () => "it fell over",
    reread: () => Promise.resolve(),
  });
}

describe("a press on a member's conversation", () => {
  /// The load-bearing one. Both presses are on the one table under the one
  /// number, and the sidebar is this device's list: the member's row is on no
  /// account of it, and an entry found by the id off that row would be this
  /// device's press of the same number.
  it("puts no row on this device's sidebar", () => {
    unarchived(MEMBER, { ...BOTH, branch: "over-on-the-member" });
    unarchived(null, BOTH);

    const drawn = pressedRows([], false);

    expect(drawn.map((row) => row.branch)).toEqual([HERE.branch]);
    expect(drawn).toHaveLength(1);
  });

  /// And it is still drawn where it belongs, which is the Conversation being
  /// read: the overlay is keyed rather than filtered, so a press on a member's
  /// is on the member's and on nothing else.
  it("is laid over that member's own conversation and no other", () => {
    eagerly({
      device: MEMBER,
      conversation: BOTH.id,
      says: { closed: true },
      post: () => Promise.resolve("Closed"),
      refusal: () => "",
      fell: () => "it fell over",
      reread: () => Promise.resolve(),
    });

    expect(pressed(MEMBER, BOTH).state).toBe("Closed");
    expect(pressed(null, BOTH).state).toBe(BOTH.state);
  });
});
