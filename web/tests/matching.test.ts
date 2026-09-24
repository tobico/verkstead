//! What the quick-open palette matches with, on its own: the subsequence and
//! the order it puts the answers in.
//!
//! `src/workbench/matching.ts` is what the palette in `Quick.tsx` draws its rows
//! from, and it is a function over two strings — so it is asked here rather than
//! through the pane, where what could be proved is that a list came back rather
//! than that the right thing is at the top of it.
//!
//! Every test here is about *what beats what*: the numbers themselves are the
//! module's own and worth nothing outside it, and a test asserting one would be
//! a test of the constants rather than of the order a human reads.

import { describe, expect, it } from "vitest";

import type { FileList } from "../src/api/types";
import { matching, scored } from "../src/workbench/matching";

/// Which of two paths a typed string is a better answer for, as a sentence
/// about the pair rather than about either.
function beats(better: string, worse: string, typed: string): boolean {
  const left = scored(better, typed);
  const right = scored(worse, typed);

  if (left === null) {
    throw new Error(`${better} did not match ${typed} at all`);
  }

  return right === null || left > right;
}

/// A root with files in it, spelled the way a Worktree is.
function root(repo: string, at: string, files: string[]): FileList {
  return {
    repo,
    path: at,
    files: files.map((file) => `${at}/${file}`),
    cut: false,
  };
}

describe("matching a typed string against a path", () => {
  /// The whole of what makes something a match: the letters, in the order they
  /// were typed, somewhere in the path.
  it("matches the letters typed in the order they were typed", () => {
    expect(scored("crates/server/files.rs", "csf")).not.toBeNull();
    expect(scored("crates/server/files.rs", "files")).not.toBeNull();
    expect(scored("crates/server/files.rs", "sfr")).not.toBeNull();

    // And out of order, or holding a letter the path does not, is no match.
    expect(scored("crates/server/files.rs", "fsc")).toBeNull();
    expect(scored("crates/server/files.rs", "zeal")).toBeNull();

    // Case is ignored, a path being typed in a hurry.
    expect(scored("web/src/workbench/Tree.tsx", "tree")).not.toBeNull();
    expect(scored("web/src/workbench/Tree.tsx", "TREE")).not.toBeNull();
  });

  /// And nothing typed matches everything, which is the palette as it opens.
  it("matches everything before anything is typed", () => {
    expect(scored("crates/server/files.rs", "")).toBe(0);
    expect(scored("", "")).toBe(0);
  });

  /// The start of a file's name beats the same letters scattered through a
  /// directory nobody meant, which is the whole of *preferring path segments*.
  it("puts a file's own name above the directories above it", () => {
    expect(beats("web/src/workbench/Tree.tsx", "docs/roadmaps/03-the-tree.md", "tree")).toBe(true);
    expect(beats("crates/server/files.rs", "crates/render/src/profiles.rs", "files")).toBe(true);
  });

  /// And a word typed as a word beats its letters falling where they may.
  it("puts the word typed above a subsequence of it", () => {
    expect(beats("src/matching.ts", "src/my/attaching.ts", "matching")).toBe(true);
    expect(beats("web/Code.tsx", "crates/render/src/conversations.rs", "code")).toBe(true);
  });

  /// And the heads of the segments beat the middles of them, which is what
  /// makes an initialism find the file somebody meant.
  it("puts the heads of segments above letters inside words", () => {
    expect(beats("crates/server/types.rs", "crates/server/facts.rs", "cst")).toBe(true);
  });

  /// And of two paths that matched the same way, the shorter is the better
  /// answer: there is less between the human and it.
  it("puts the shorter of two otherwise equal paths first", () => {
    expect(beats("src/tree.ts", "src/deep/down/tree.ts", "tree")).toBe(true);
  });

  /// A separator typed is a separator matched, which is how somebody says *the
  /// one under here* rather than *anything with those letters in it*.
  it("matches a separator typed as a separator", () => {
    expect(scored("web/src/workbench/Tree.tsx", "web/tree")).not.toBeNull();
    expect(scored("crates/server/tree.rs", "web/tree")).toBeNull();
  });

  /// And a path a boundary-first walk would have run out of is matched all the
  /// same: the preference is what orders two matches rather than what decides
  /// whether something is one.
  it("still matches where reaching for a segment head would run out of path", () => {
    expect(scored("attb/t", "tt")).not.toBeNull();
  });
});

describe("matching across the roots", () => {
  /// What the palette is handed: every file of every root that matched, with
  /// the root it is in on the row, best first.
  it("lists matches from every root, each with its root, best first", () => {
    const rows = matching(
      [
        root("verkstead", "/worktrees/verkstead", ["web/src/workbench/Tree.tsx", "Cargo.toml"]),
        root("askance", "/worktrees/askance", ["src/tree.rs", "README.md"]),
      ],
      "tree",
    );

    expect(rows.map((row) => [row.repo, row.under])).toEqual([
      ["askance", "src/tree.rs"],
      ["verkstead", "web/src/workbench/Tree.tsx"],
    ]);

    // And the path a tab is opened by is the whole of it, spelled the way its
    // root is: the row shows the part under the root and opens the file itself.
    expect(rows[0]?.path).toBe("/worktrees/askance/src/tree.rs");
  });

  /// And the root itself is no part of what is matched: the directory
  /// Verkstead keeps its Worktrees in is not something anybody is searching
  /// for, and every row would answer to it.
  it("matches the path under the root rather than the whole of it", () => {
    const rows = matching(
      [root("verkstead", "/worktrees/verkstead-code-pane", ["Cargo.toml"])],
      "worktrees",
    );

    expect(rows).toEqual([]);
  });

  /// Nothing typed is every file there is, in the order the roots gave them —
  /// git's own, which is what the palette stands open on.
  it("lists every file of every root before anything is typed", () => {
    const rows = matching(
      [
        root("verkstead", "/worktrees/verkstead", ["Cargo.toml", "README.md"]),
        root("askance", "/worktrees/askance", ["src/lib.rs"]),
      ],
      "",
    );

    expect(rows.map((row) => row.under)).toEqual([
      "Cargo.toml",
      "README.md",
      "src/lib.rs",
    ]);
  });
});
