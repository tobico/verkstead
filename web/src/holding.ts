//! Files a page is holding until there is something to attach them to.
//!
//! The composer of a draft attaches as it goes: there is a Conversation under
//! it, so a file chosen there is a request of its own and the record is back
//! before the pill stops being dimmed. The compose page has no Conversation at
//! all — nothing exists on the server until Start or Save as draft is pressed —
//! so the files chosen there are held here, the `File` objects themselves,
//! beside what the device is holding of the composition.
//!
//! **Held in the page rather than on the device.** What is composed survives a
//! reload because it is text and a reload can put text back; a `File` the human
//! picked is a handle the browser gave this page and nothing that outlives it.
//! So a reload keeps the brief and loses the files, and the row of pills simply
//! is not there any more — see `workbench/composing.ts` for what *is* kept, and
//! `device.ts` for where it is kept.
//!
//! A piece of its own rather than a fold in the compose page, because that page
//! is not the only thing that will hold files until there is somewhere to send
//! them: an Answer sheet filled in over an hour and submitted at the end of it
//! is the same shape, and the same three things are asked of it — put one in,
//! take one out, and send the lot once something exists to send them to.

import { createSignal } from "solid-js";

import { attachFile } from "./api/client";
import type { Attached } from "./api/types";

/// One file being held: the file itself, and the key its pill is drawn under.
///
/// A key of its own rather than the name, because two files chosen together may
/// share one — and what the key is for is taking the right one out of the row.
export type Held = { key: number; file: File };

/// One file the flush could not put on the Conversation: the name it went up
/// under, and which refusal came back.
///
/// A refused upload is one more refusal for the page that pressed to carry,
/// worded where the rest of a replay's refusals are worded — see `composing.ts`.
/// A request that never landed at all is not one of these: it throws, the way
/// every other step of a replay throws, and the press says so.
export type Rejected = {
  name: string;
  refused: Exclude<Attached, { Attached: unknown }>;
};

/// The files one page is holding, and the three things there are to do with
/// them.
export type Holding = {
  /// What is held, in the order it was chosen.
  held: () => Array<Held>;

  /// Hold some more — a choice is several files at once, through the picker and
  /// through a drop alike.
  add: (files: Array<File>) => void;

  /// And take one out again, by the key its pill was drawn under.
  drop: (key: number) => void;

  /// Send the lot to a Conversation that exists now, one request per file and
  /// in the order they were chosen — holding nothing afterwards, whatever became
  /// of them. What came back refused is returned for the page to say; what never
  /// landed throws.
  flush: (conversation: number) => Promise<Array<Rejected>>;
};

/// A page's holding, made where the page is.
export function holding(): Holding {
  const [held, setHeld] = createSignal<Array<Held>>([]);

  let keys = 0;

  const add = (files: Array<File>) =>
    setHeld((was) => [
      ...was,
      ...files.map((file) => ({ key: (keys += 1), file })),
    ]);

  const drop = (key: number) =>
    setHeld((was) => was.filter((one) => one.key !== key));

  const flush = async (conversation: number): Promise<Array<Rejected>> => {
    const rejected: Array<Rejected> = [];

    // One at a time rather than all at once, and in the order the row had them:
    // a name already taken counts up on its way in, so which of two `notes.md`s
    // becomes `notes-2.md` is decided by which of them was chosen first.
    for (const one of held()) {
      const outcome = await attachFile(conversation, one.file);
      if (typeof outcome === "string") {
        rejected.push({ name: one.file.name, refused: outcome });
      }

      // Sent is sent, refused or not: the page that pressed is on its way into
      // the Conversation it made, and a file left here would be one this device
      // offered to attach to whatever it composed next.
      drop(one.key);
    }

    return rejected;
  };

  return { held, add, drop, flush };
}
