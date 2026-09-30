//! How much of a checklist its card shows: the entries the work is at, five
//! rows of the list around them, and a mark wherever the rest of it is.
//!
//! A backlog and a roadmap are both pinned above the record, where they stay
//! for the whole of a Conversation, and a card that grew with the list would
//! push everything the human came to read off the screen. What is worth seeing
//! there is where the work is — the entries being worked, what they came out of
//! and what they go into — so the card keeps a window of five over them and the
//! details pane keeps the whole list.
//!
//! **Where the work is may be several places.** A backlog is worked a task at a
//! time, and so was a roadmap until its stages ran side by side; three stages
//! running is three places, and none of them is worth hiding to hold the window
//! at five, so the window is told which entries are in flight and keeps every
//! one of them. A list with none falls back to the first entry that is not done,
//! which is the one place a backlog ever has and is where a roadmap with nothing
//! running is read.
//!
//! Shared by the task list and the stage list because they are the same card
//! one level apart, and a window that read differently on the two would be two
//! ideas of where the work is.

/// How many entries a card draws where it can. Five is what fits above the
/// record on a phone without the record itself going under the fold.
///
/// A floor rather than a ceiling in the one case they part: more than five
/// entries in flight are more than five rows, because a row is what says an
/// entry is being worked and the card is read for exactly that.
export const WINDOW = 5;

/// A run of neighbouring entries the card draws whole, with however many of
/// the list are out of sight in front of it.
export interface Stretch<T> {
  /// How many entries are hidden between the stretch before this one and this
  /// one — or before the first of them, at the head of the list. None only at
  /// that head: two stretches with nothing between them are one stretch.
  hidden: number;

  /// The entries to draw, in the list's own order.
  entries: T[];
}

/// A list cut down to what its card shows: the stretches of it in view, and how
/// many entries are out of sight after the last of them.
///
/// Stretches rather than the one slice this was, because the places the work is
/// need not be neighbours: two stages running with a stage nobody has started
/// between them is two stretches, and what sits between them is said the way
/// the ends of the list are.
export interface Window<T> {
  /// The runs of the list in view, in the list's own order. One of them is the
  /// ordinary case — a window over a list worked in one place is a slice of it.
  stretches: Stretch<T>[];

  /// How many entries are hidden after the last of them. More than none is
  /// what puts an ellipsis row under the list.
  after: number;
}

/// The window of `entries` over the places the work is: every entry `inFlight`
/// says is being worked, the neighbours that fill the window out to five, and
/// what is hidden between and around them.
///
/// `done` is what the window falls back to where nothing is in flight — the
/// first entry that is not done, which is where a backlog is worked and where a
/// roadmap with nothing running is read. A list with every entry done is at its
/// end: stage lists outlive their completion, and the last five are what a
/// finished list is looked at for.
///
/// The neighbours are what make one place read the way a list does: the entry
/// being worked, what the work just finished and what it is about to start say
/// as much about where it is as the entry itself. They are held inside the
/// list's ends, so the head of a backlog shows its first five and the end of one
/// shows its last five rather than a half-empty window hanging off either end.
export function windowed<T>(
  entries: T[],
  done: (entry: T) => boolean,
  inFlight: (entry: T) => boolean = () => false,
): Window<T> {
  if (entries.length <= WINDOW) {
    return { stretches: [{ hidden: 0, entries }], after: 0 };
  }

  const stretches: Stretch<T>[] = [];
  let previous = -1;

  for (const at of chosen(entries, done, inFlight)) {
    const last = stretches[stretches.length - 1];
    if (last && at === previous + 1) {
      last.entries.push(entries[at]!);
    } else {
      stretches.push({ hidden: at - previous - 1, entries: [entries[at]!] });
    }
    previous = at;
  }

  return { stretches, after: entries.length - previous - 1 };
}

/// Which entries the card shows, as places in the list and in its order: every
/// one in flight, and the neighbours the rest of the window goes on.
function chosen<T>(
  entries: T[],
  done: (entry: T) => boolean,
  inFlight: (entry: T) => boolean,
): number[] {
  const at = places(entries, done, inFlight);
  const shown = new Set(at);
  let spare = Math.max(WINDOW - at.length, 0);

  // The gaps between them go first, narrowest first: an entry hidden between
  // two in view costs the row it would have been drawn in and says less than it
  // would have said, so what is left of the window closes whichever gaps it can
  // afford. Whole ones only — a gap half closed still needs its mark.
  for (const gap of gaps(at)) {
    if (gap.length > spare) continue;
    for (const hidden of gap) shown.add(hidden);
    spare -= gap.length;
  }

  // And whatever is still spare goes either side of them, shared evenly and
  // held inside the list: what one end has no room for, the other end takes.
  const first = at[0]!;
  const last = at[at.length - 1]!;
  const after = Math.min(
    spare - Math.min(Math.floor(spare / 2), first),
    entries.length - 1 - last,
  );
  const before = Math.min(spare - after, first);

  for (let n = 1; n <= before; n += 1) shown.add(first - n);
  for (let n = 1; n <= after; n += 1) shown.add(last + n);

  return [...shown].sort((one, other) => one - other);
}

/// Where the work is in the list, as places in it: the entries in flight, or —
/// for a list with none, which is every backlog and a roadmap nothing is running
/// in — the first entry that is not done, and the last entry of a list where
/// every one of them is.
function places<T>(
  entries: T[],
  done: (entry: T) => boolean,
  inFlight: (entry: T) => boolean,
): number[] {
  const flying = entries.flatMap((entry, at) => (inFlight(entry) ? [at] : []));
  if (flying.length > 0) return flying;

  const next = entries.findIndex((entry) => !done(entry));
  return [next === -1 ? entries.length - 1 : next];
}

/// The runs of hidden entries between the places the work is, narrowest first,
/// which is the order the window can afford them in.
function gaps(at: number[]): number[][] {
  return at
    .slice(1)
    .flatMap((to, n) => {
      const from = at[n]! + 1;
      return to > from
        ? [Array.from({ length: to - from }, (_, step) => from + step)]
        : [];
    })
    .sort((one, other) => one.length - other.length);
}
