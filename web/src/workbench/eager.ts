//! What a press has already said about a Conversation, before the server has
//! said it.
//!
//! Closing one takes seconds. The handler behind it stops the running session
//! and waits for the task it was on, gives back a worktree per repository,
//! deletes the handoff directory, writes the record, locks whatever the
//! Conversation was still asking, and then sweeps the whole worktrees directory
//! — all inside the POST. Held to that round trip, the row disabled itself,
//! relabelled to *Closing…*, and nothing else on the page moved until every bit
//! of it was over: a wait of seconds for a press whose outcome was never in
//! doubt.
//!
//! So the outcome is drawn at the press and the request runs behind it. What is
//! held here is one entry per Conversation saying what the presses have decided
//! about it — that it is closed, and whether it is on the list — laid over the
//! server's own answer wherever that Conversation is drawn: the sidebar's row,
//! the status line, and the menu's own rows, which is what stands Archive where
//! Close was the instant Close is pressed.
//!
//! **Held rather than written into the cache.** A payload written over the
//! query would be stomped by the next read to land, and this page is re-read
//! constantly — a Nudge, a reconnect, the window coming back to the front. So
//! this is an overlay the reads pass under, exactly as the sidebar's drag order
//! is (see `Conversations.tsx`), and it is let go of only once the re-read
//! behind the press has landed: the press comes back, the queries it left stale
//! are read again, and *then* the overlay goes — so what replaces it is the
//! server saying the same thing.
//!
//! **A failure is the rollback.** A refusal, or a request that fell over on the
//! way out, takes the entry away: the Conversation quietly stands as it did,
//! and a toast says why. The reason alone — the row coming back is what says
//! which Conversation it is about — and a toast rather than the card over the
//! page the other rows open, because there is nothing here to decide. See
//! `Actions.tsx`, where that card is and stays, for the presses whose outcome
//! the page cannot draw ahead of.
//!
//! **A second press waits for the first.** Archive is what the menu offers the
//! moment Close is pressed, and an Archive posted while the close is still
//! running would be refused `NotClosed` — the record says the Conversation is
//! open until the close commits. So presses on one Conversation go out in
//! order, each waiting for the one before it to land. One queued behind a press
//! that failed is dropped rather than sent: the state it assumed is gone, and
//! the human has already been told.

import { createSignal } from "solid-js";

import { toast } from "../Toasts";
import type { ConversationEntry, ConversationView } from "../api/types";

/// What the presses have said about one Conversation, ahead of the server.
///
/// Merged into rather than replaced, because the two facts are decided by
/// different rows: Close says the first, Archive and Unarchive say the second,
/// and Close and archive says both in one press.
export type Said = {
  /// It is closed. Absent where no press here has said so, which is the only
  /// two values there are: nothing closes a Conversation back open.
  closed?: true;

  /// And whether it is on the conversations list, where a press has decided.
  archived?: boolean;

  /// The row to put back on that list, for the one press that puts a
  /// Conversation somewhere the list has no row for it: unarchiving one while
  /// the archived ones are hidden is asking for a row the server's own list
  /// does not carry. See [`rowFor`].
  row?: ConversationEntry;
};

const [said, setSaid] = createSignal<Record<number, Said>>({});

/// The press in flight on each Conversation, as something for the next one to
/// queue behind. It answers whether it landed, so a press waiting on it knows
/// whether the world it assumed is there.
///
/// Not a signal: nothing is drawn from it. What is drawn is [`said`], which the
/// press writes the moment it is made.
const running = new Map<number, Promise<boolean>>();

/// What is true of a Conversation the moment it is closed, in the fields the
/// menu and the status line read.
///
/// Written out rather than left as a changed state word. Every one of these is
/// a fact the server computes and something on the page keys off, and a
/// half-closed Conversation is one whose menu offers rows nothing could answer:
/// a stop for a session that has ended, a resume for work that is over.
const CLOSED = {
  state: "Closed",
  working: false,
  driven: false,
  ready_to_resume: false,
  ready_to_stop: false,
  stop_asked: false,
  ready_to_grill: false,
  ready_to_continue: false,
  waiting: false,
  waiting_on_checks: false,
} as const satisfies Partial<ConversationView>;

/// And the same on a sidebar row, which carries the marks rather than the
/// readiness: a closed Conversation has no session running, nothing waiting on
/// anybody and no checks to wait for.
const CLOSED_ROW = {
  state: "Closed",
  working: false,
  idle: false,
  waiting: false,
  waiting_on_checks: false,
} as const satisfies Partial<ConversationEntry>;

/// The Conversation as the page is drawing it: the server's answer, with
/// whatever a press has already said about it laid over.
///
/// The very object it was handed wherever nothing has been pressed, which is
/// nearly every moment: the reading is a store the whole pane is merged into
/// (see `freshness.ts`), and handing back a copy of it would rebuild rows that
/// have not moved.
export function pressed(view: ConversationView): ConversationView {
  const over = said()[view.id];
  if (over === undefined) return view;

  return {
    ...view,
    ...(over.closed ? CLOSED : {}),
    ...(over.archived === undefined ? {} : { archived: over.archived }),
  };
}

/// And the sidebar's list the same way: each row as its presses left it, the
/// ones put away taken out of it, and the one put back on it added.
///
/// `archived` is the sidebar's own setting — whether the ones put away are
/// drawn among them — because that is what decides whether an archiving takes a
/// row off the list at all. It is the server's rule for the same list, read the
/// same way (see the store's `conversations`), so a press and the read behind
/// it say the same thing about the same row.
export function pressedRows(
  rows: ConversationEntry[],
  archived: boolean,
): ConversationEntry[] {
  const over = said();
  const ids = Object.keys(over);
  if (ids.length === 0) return rows;

  const drawn = rows.flatMap((row) => {
    const on = over[row.id];
    if (on === undefined) return [row];
    if (on.archived === true && !archived) return [];

    return [on.closed ? { ...row, ...CLOSED_ROW } : row];
  });

  // And the one that has to be put back rather than left alone. At the top,
  // which is where a Conversation the order says nothing about goes here and on
  // the server both.
  const back = ids
    .map((id) => over[Number(id)]!.row)
    .filter(
      (row): row is ConversationEntry =>
        row !== undefined &&
        over[row.id]!.archived === false &&
        !drawn.some((one) => one.id === row.id),
    );

  return back.length === 0 ? drawn : [...back, ...drawn];
}

/// The sidebar row a Conversation put back on the list stands as until the
/// server's own list arrives with one.
///
/// Off the reading the press was made against, which carries every field a row
/// does but the marks — and the marks are all off: unarchiving is offered on a
/// closed Conversation, and nothing on one of those is running, waiting, or
/// news the human has not read.
export function rowFor(view: ConversationView): ConversationEntry {
  return {
    id: view.id,
    branch: view.branch,
    branch_named: view.branch_named,
    naming: view.naming,
    repo: view.repo.name,
    state: view.state,
    working: false,
    idle: false,
    waiting: false,
    waiting_on_checks: false,
    unseen: false,
  };
}

/// One press whose outcome the page is already drawing.
export type Press<Outcome> = {
  /// Which Conversation it is about.
  conversation: number;

  /// What it says is true of that Conversation, from this moment.
  says: Said;

  /// The request itself, which runs behind the page.
  post: () => Promise<Outcome>;

  /// What the answer means: the refusal's own sentence, or the empty string
  /// where the press landed. The maps in `Actions.tsx` are written exactly this
  /// way — the outcomes that are not refusals map to nothing.
  refusal: (outcome: Outcome) => string;

  /// And what a request that never landed is said in, which is the page's own
  /// sentence rather than the server's.
  fell: (error: Error) => string;

  /// Reading back whatever the press left stale, which is the caller's: this
  /// waits for it, and lets the overlay go once it has landed.
  reread: () => Promise<unknown>;
};

/// Make one: what it says goes on the page now, and the request goes out behind
/// whatever is already in flight on the same Conversation.
export function eagerly<Outcome>(press: Press<Outcome>): void {
  const id = press.conversation;

  setSaid((standing) => ({
    ...standing,
    [id]: { ...standing[id], ...press.says },
  }));

  const queued = running.get(id) ?? Promise.resolve(true);
  const mine = queued.then((landed) => (landed ? made(press) : false));
  running.set(id, mine);

  void mine.then(() => {
    // The last press on this Conversation, and the re-read behind it has
    // landed: there is nothing left for the overlay to be ahead of. A press
    // made since is holding the entry instead, and this one is not its to drop.
    if (running.get(id) !== mine) return;

    running.delete(id);
    forgetOne(id);
  });
}

/// One press, once whatever was in front of it has landed. Answers whether it
/// landed itself, which is what the press behind it waits on.
async function made<Outcome>(press: Press<Outcome>): Promise<boolean> {
  try {
    const refused = press.refusal(await press.post());
    if (refused) {
      undo(press.conversation, refused);
      return false;
    }
  } catch (error) {
    undo(press.conversation, press.fell(error as Error));
    return false;
  }

  // Before the overlay is let go of rather than after: between the press coming
  // back and the read landing is exactly where a page swapped for the server's
  // old answer would flick back to it.
  await press.reread();
  return true;
}

/// A press that did nothing: what it said comes off the page, and the reason is
/// said where the human is standing.
///
/// The whole entry rather than this press's half of it. Presses on one
/// Conversation go out in order and each waits for the read behind the one
/// before it, so anything an earlier press said is already in the reading this
/// leaves behind — and anything a later one said assumed the state this press
/// has just failed to reach.
function undo(id: number, why: string): void {
  forgetOne(id);
  toast(() => why);
}

function forgetOne(id: number): void {
  setSaid((standing) => {
    const rest = { ...standing };
    delete rest[id];
    return rest;
  });
}

/// Let go of everything said, which is what the app going away does.
///
/// Nothing here outlives its own request in an app that stays up — an entry is
/// made by a press and dropped when that press settles — so this is about the
/// page itself ending: what a press said belongs to the app it was said in, and
/// nothing should come back up under one mounted after it. The toast layer is
/// cleared for the same reason and in the same place, `Shell` in `App.tsx`.
export function forget(): void {
  setSaid({});
  running.clear();
}
