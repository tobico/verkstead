//! Matching what somebody typed against the files of every root, for the
//! quick-open palette in [`./Quick`].
//!
//! **On the page rather than on the server.** The server answers with the
//! files a Conversation has and nothing about what was typed — one read when
//! the palette opens, and then every keystroke is matched here (ADR 0019, *The
//! tree*). A request per keystroke over a list the page is already holding
//! would be a round trip inside the gap between two letters.
//!
//! **Subsequence, preferring whole path segments**, which is what VS Code's
//! own feels like at this size: every letter typed has to be in the path in the
//! order it was typed, and what sorts one match above another is *where* the
//! letters landed. A letter starting a segment or a word inside one is worth
//! more than a letter in the middle of something; a letter straight after the
//! one before it is worth more than one further along; and the file's own name
//! is read as a string of its own where the letters are all in it, so that a
//! name beginning with what was typed beats the same letters scattered through
//! a directory nobody meant.
//!
//! **And a whole word typed beats a subsequence of it**, which is the case
//! nearly every search really is: somebody types `tree` meaning the file called
//! `Tree.tsx`, rather than meaning the four letters in that order somewhere
//! under `docs/roadmaps/`. So a path holding what was typed *as it was typed*
//! is lifted above one that only holds its letters, and a name that begins with
//! it is lifted again.
//!
//! **The match is over the part under the root**, rather than over the whole
//! path: the root is drawn beside the row as the Repo it is a checkout of, and
//! the directory Verkstead happens to keep its Worktrees in is not something
//! anybody is searching for — every row would match `worktrees` otherwise.
//!
//! Case is ignored, because a path is typed in a hurry and nobody holds shift
//! for a capital in a file name they are looking for. Nothing else is
//! normalised: a separator typed is a separator matched, which is how `web/tree`
//! says *the Tree under web* rather than *anything with those letters in it*.

import type { FileList } from "../api/types";

/// What a letter is worth where it starts a path segment or a word inside one.
///
/// The largest of the three, because it is the one that says somebody meant
/// this file: typing `cst` and landing on `crates/server/types.rs` is three
/// segment heads, and landing on the same three letters inside one word is the
/// coincidence the palette should draw underneath it.
const BOUNDARY = 10;

/// And where it follows the letter before it without a gap.
///
/// Under a boundary and well over a letter on its own: a run is what makes a
/// typed word a word rather than three letters that happen to be in order.
const RUN = 8;

/// And where it is neither: a letter matched is still a letter matched.
const LETTER = 1;

/// And where it is in the path exactly as it was typed, letter after letter.
///
/// The ordinary search: `Tree.tsx` typed as `tree`. Worth more than the run
/// bonuses the same letters earn, so a path holding the word beats one holding
/// only its letters however well those letters fell.
const WHOLE = 60;

/// And where the name *begins* with it, which is the answer somebody is usually
/// after when they type the start of a file's name.
const OPENING = 40;

/// How many rows the palette draws.
///
/// A palette is read rather than scrolled: what is not in the first few dozen
/// is found by typing another letter rather than by walking down. The rest are
/// matched all the same — this is what is drawn, not what is looked at.
export const MOST = 50;

/// One row of the palette: a file, the root it is in, and how well it matched.
export interface Found {
  /// The path as a tab is opened by it, spelled the way its root is.
  path: string;

  /// And the part of it under that root, which is what the row shows and what
  /// was matched: the row says which Repo it is in beside it.
  under: string;

  /// The Repo the root is a checkout of, which is what the row names it by.
  repo: string;

  /// What the match came to, which is the order the rows are in.
  score: number;
}

/// Every file of every root that matches what was typed, best first.
///
/// **Across the roots rather than within one**: the human is looking for a
/// file, and which checkout it is in is something the row tells them rather
/// than something they had to say first. Two roots can hold the same path, so
/// the root is part of the row.
///
/// Nothing typed matches everything, in the order the roots gave it — which is
/// git's own, tracked before untracked and each alphabetical. A palette that
/// opened empty would be a box with nothing in it to press.
export function matching(roots: FileList[], typed: string): Found[] {
  const found: Found[] = [];

  for (const root of roots) {
    for (const path of root.files) {
      // The part under the root, cut rather than split on a separator: a
      // Worktree on Windows is spelled with a `\`, and the server joined this
      // path onto that root itself — so what is left after the root and the one
      // separator after it is the path inside it.
      const under = path.slice(root.path.length + 1);
      const score = scored(under, typed);

      if (score !== null) {
        found.push({ path, under, repo: root.repo, score });
      }
    }
  }

  // Nothing typed is every file there is, in the order the roots gave them,
  // which is git's own: tracked before untracked and each of those in its own
  // alphabetical order. Sorting a list of equal scores would only put it in
  // another order nobody asked for.
  if (typed === "") {
    return found;
  }

  // Best first, and then by what the path *is*: a sort with ties in it would
  // put the same two rows in a different order on two machines, and a list that
  // reorders under a keystroke that changed nothing is a list nobody can aim at.
  found.sort(
    (left, right) =>
      right.score - left.score ||
      left.under.length - right.under.length ||
      left.under.localeCompare(right.under) ||
      left.repo.localeCompare(right.repo),
  );

  return found;
}

/// How well `text` matches `typed`, or `null` where it does not match at all.
///
/// The whole of the ordering the palette draws. Exported for the tests, which
/// are about what beats what rather than about what a list came to.
export function scored(text: string, typed: string): number | null {
  if (typed === "") {
    return 0;
  }

  const wanted = typed.toLowerCase();
  const within = text.toLowerCase();
  const at = within.lastIndexOf("/") + 1;
  const back = within.lastIndexOf("\\") + 1;
  const name = within.slice(Math.max(at, back));

  // The name first, and what is scored is the name alone: the head of a name
  // is a segment head, and the two bonuses below are read against the name
  // rather than against the path — which is the whole of how a file called
  // what was typed comes above one whose directories happen to hold the
  // letters. There is no bonus for being a name match beyond that, because a
  // name is not a better answer than a path somebody spelled out: `cst` typed
  // over three segment heads means that file, whatever some other file is
  // called.
  const inName = walked(name, wanted);

  if (inName !== null) {
    return (
      inName +
      (name.includes(wanted) ? WHOLE : 0) +
      (name.startsWith(wanted) ? OPENING : 0) -
      shortest(text)
    );
  }

  const anywhere = walked(within, wanted);

  return anywhere === null
    ? null
    : anywhere + (within.includes(wanted) ? WHOLE : 0) - shortest(text);
}

/// The letters of `wanted` found in `text` in the order they were typed, and
/// what that came to — or `null` where one of them is not there at all.
///
/// **Greedy, and boundary-first**: each letter is taken at the next place it
/// appears, except that a letter continuing the run is taken where it is and a
/// letter with a segment or word head ahead of it is taken there. Which is not
/// the best arrangement of the letters in every case — finding that is a
/// tableau over every letter against every letter, and this is run over every
/// file of a checkout between two keystrokes. What it is instead is the
/// arrangement a human would point at, in one pass down the path.
///
/// **And a second pass where that one ran out of path**, which is what keeps
/// the preference from costing a match: reaching ahead for a segment head can
/// step over the last of a letter — `tt` against `attb/t` takes the head and
/// then has no `t` left — and a plain walk from the left never does, having
/// skipped nothing. So the file is matched either way, and the boundary pass
/// is only ever what makes one match score better than another.
function walked(text: string, wanted: string): number | null {
  return pass(text, wanted, true) ?? pass(text, wanted, false);
}

/// One walk down the path, reaching for segment heads or not.
function pass(text: string, wanted: string, heading: boolean): number | null {
  let score = 0;
  let last = -2;
  let from = 0;

  for (const letter of wanted) {
    const found = heading
      ? taken(text, letter, from, last)
      : text.indexOf(letter, from);

    if (found < 0) {
      return null;
    }

    score += LETTER;

    if (found === last + 1) {
      score += RUN;
    }

    if (heads(text, found)) {
      score += BOUNDARY;
    }

    last = found;
    from = found + 1;
  }

  return score;
}

/// Where the next `letter` is taken from, searching from `from` — the run
/// first, then a segment or word head, then wherever it is.
function taken(text: string, letter: string, from: number, last: number): number {
  if (from === last + 1 && text[from] === letter) {
    return from;
  }

  for (let at = text.indexOf(letter, from); at >= 0; at = text.indexOf(letter, at + 1)) {
    if (heads(text, at)) {
      return at;
    }
  }

  return text.indexOf(letter, from);
}

/// Whether the character at `at` starts a path segment or a word inside one.
///
/// A separator either way it is spelled, and the punctuation a file name is
/// built out of: `code-pane`, `file_roots`, `Tree.tsx` are three words apiece to
/// the eye and should be three to the match. The start of the path is a head
/// too — it starts the first segment.
function heads(text: string, at: number): boolean {
  if (at === 0) {
    return true;
  }

  return "/\\-_. ".includes(text[at - 1] ?? "");
}

/// The nudge that puts a shorter path above a longer one whose letters fell the
/// same way.
///
/// Small enough that it never outweighs where the letters landed — a hundredth
/// of a letter's own worth apiece — and enough to settle two paths that are
/// otherwise the same answer: `src/tree.ts` is a better answer to `tree` than
/// `src/deep/down/tree.ts`, having less between the human and it.
function shortest(text: string): number {
  return text.length / 100;
}
