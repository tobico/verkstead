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
//! is (see `Conversations.tsx`), and it is let go of only once **a read has
//! landed** since the press did — rather than when the request behind the press
//! came back, which happens whether anything was read or not. See [`caughtUp`],
//! which is where that decision is written down: what replaces what a press drew
//! is always an answer of the server's own.
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

  /// How many times the list had answered when the press behind this landed —
  /// see [`answered`]. Absent while the press is still out.
  ///
  /// Which is what a read has to be later than to be a read of the world the
  /// press made, and so the whole of what lets the entry go — see [`caughtUp`].
  /// Until the press lands nothing releases it whatever any read says: an entry
  /// ahead of the server is *supposed* to disagree with what the server last
  /// said, and that is what holds a close on the page through a Nudge landing
  /// mid-flight.
  landed?: number;
};

/// How many times the list the sidebar draws has answered.
///
/// Counted rather than timed. What a press needs to know is whether a read has
/// landed *since* it did, and a clock cannot say so about two reads a
/// millisecond apart — while a count that only ever moves forward can, and can
/// never move for a read that had already happened. Which is the direction that
/// matters: an entry let go of a read too late is a page still telling the
/// truth, and one let go of a read too early is the bug this is here about.
///
/// Moved by [`caughtUp`], which the sidebar calls whenever its read of the list
/// answers again.
let answered = 0;

/// The value `dataUpdatedAt` last had, for telling an answer from a redraw.
let answeredAt = 0;

const [said, setSaid] = createSignal<Record<number, Said>>({});

/// The press in flight on each Conversation, as something for the next one to
/// queue behind. It answers when it landed, or `null` where it did nothing — so
/// a press waiting on it knows whether the world it assumed is there.
///
/// Not a signal: nothing is drawn from it. What is drawn is [`said`], which the
/// press writes the moment it is made.
const running = new Map<number, Promise<number | null>>();

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

/// Let go of every press a read has since caught up with, which is the one thing
/// that ever releases one.
///
/// `readAt` is when the list the sidebar draws last **answered** — tanstack's
/// `dataUpdatedAt`, which moves on a read that landed and stays put on one that
/// did not. That is the whole of the test, and the reason it is that rather than
/// the request behind the press coming back: the re-read a press ends with comes
/// back whatever became of it. `invalidateQueries` swallows a refetch that
/// failed, and refetches nothing at all for a query nothing is reading. Released
/// on the request alone, a close the server really made would quietly reappear
/// as open the first time either of those happened — the page taking back
/// something true because the read of it fell over.
///
/// What a press is held against is [`answered`] rather than that moment itself,
/// so that reads are counted rather than clocked — see there.
///
/// **A read that landed and says something else still wins.** It is the server's
/// own word about where the Conversation stands now, later than the press and
/// as entitled to have moved — somebody steering it back into the work from
/// another device is exactly that. What is held is not an opinion about the
/// answer, only the refusal to let go without one.
///
/// **Off the list rather than off the pane's own reading**, because the list is
/// what is on the screen throughout: the sidebar is drawn beside every
/// Conversation and a press can be made from a card in it about one no pane is
/// showing. What the pane reads is invalidated in the same breath and lands in
/// the same round, and the failure this is about — nothing landing at all — is
/// one they have together.
///
/// Called from an effect over that list, in `Conversations.tsx`, beside the one
/// that lets go of the drag order the same way.
export function caughtUp(readAt: number): void {
  if (readAt !== answeredAt) {
    answeredAt = readAt;
    answered += 1;
  }

  const over = said();
  const read = Object.keys(over)
    .map(Number)
    .filter((id) => {
      const landed = over[id]!.landed;

      return landed !== undefined && landed < answered;
    });

  if (read.length === 0) return;

  setSaid((standing) => {
    const rest = { ...standing };
    for (const id of read) delete rest[id];

    return rest;
  });
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
  /// waits for it, and settles what the press said once it has been asked for.
  /// Whether it answered is the readings' own business — see [`Said.settled`].
  reread: () => Promise<unknown>;
};

/// Make one: what it says goes on the page now, and the request goes out behind
/// whatever is already in flight on the same Conversation.
export function eagerly<Outcome>(press: Press<Outcome>): void {
  const id = press.conversation;

  setSaid((standing) => {
    // Whatever a press before this one said, and this press over it — but not
    // when that one landed. This one has not, and an entry that came up landed
    // would be released by the very reads it is meant to be ahead of.
    const over: Said = { ...standing[id], ...press.says };
    delete over.landed;

    return { ...standing, [id]: over };
  });

  // Nothing in front of it: a moment long past rather than a press, so the one
  // being made now goes straight out.
  const queued = running.get(id) ?? Promise.resolve<number | null>(0);
  const mine = queued.then((at) => (at === null ? null : made(press)));
  running.set(id, mine);

  void mine.then((at) => {
    // The last press on this Conversation: a press made since is holding the
    // entry instead, and this one has no say in what becomes of it.
    if (running.get(id) !== mine) return;

    running.delete(id);

    // And what it said is now the list's to let go of, once a read lands that
    // is later than this — see [`caughtUp`]. Nothing to mark where the press
    // did nothing: [`undo`] has already taken what it said away.
    if (at !== null) marked(id, at);
  });
}

/// One press, once whatever was in front of it has landed. Answers the read
/// count it landed at, or `null` where it did nothing — which is both what the
/// press behind it waits on and what a read has to be later than to have caught
/// up with it.
async function made<Outcome>(press: Press<Outcome>): Promise<number | null> {
  try {
    const refused = press.refusal(await press.post());
    if (refused) {
      undo(press.conversation, refused);
      return null;
    }
  } catch (error) {
    undo(press.conversation, press.fell(error as Error));
    return null;
  }

  // The record is written by the time the press is answered, so the next read
  // to land is a read of what the press did. Taken here rather than after the
  // re-read below, which is one of the reads it has to be counted before.
  const at = answered;

  // And then the read itself, waited for here rather than set going: between
  // the press coming back and the read landing is exactly where a page swapped
  // for the server's old answer would flick back to it.
  //
  // A read that fell over is not this press failing — the press landed, and the
  // server did what it was asked — so nothing is rolled back and nothing is
  // said. What it leaves behind is a page still drawing what the press said,
  // which is right, and a read later on to let go of it.
  try {
    await press.reread();
  } catch {
    // Nothing to do about it here. The next read of either query corrects it.
  }

  return at;
}

/// The press behind an entry landed, at the read count given, so what it said is
/// the list's to let go of from here — see [`caughtUp`].
function marked(id: number, at: number): void {
  setSaid((standing) => {
    const over = standing[id];

    return over === undefined
      ? standing
      : { ...standing, [id]: { ...over, landed: at } };
  });
}

/// A press that did nothing: what it said comes off the page, and the reason is
/// said where the human is standing.
///
/// The whole entry rather than this press's half of it. Presses on one
/// Conversation go out in order and each waits for the read behind the one
/// before it, so anything an earlier press said is what the reading this leaves
/// behind was asked for — and anything a later one said assumed the state this
/// press has just failed to reach. A refusal is the server's own word about
/// where the Conversation stands, which is the one thing worth more than
/// anything the page had decided about it.
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
/// Nothing here outlives the reads catching up with it in an app that stays up
/// — an entry is made by a press and let go of by the list agreeing with it —
/// so this is about the page itself ending: what a press said belongs to the app
/// it was said in, and nothing should come back up under one mounted after it.
/// The toast layer is cleared for the same reason and in the same place, `Shell`
/// in `App.tsx`.
///
/// It is what clears an entry the reads never did agree with, too — a close the
/// server made and no read since has landed. Held while the app is up, which is
/// the point of holding it, and gone with the app.
export function forget(): void {
  setSaid({});
  running.clear();

  // The read count with them: it counts one app's reads of one list, and the
  // next app's are a fresh list read from nothing.
  answered = 0;
  answeredAt = 0;
}
