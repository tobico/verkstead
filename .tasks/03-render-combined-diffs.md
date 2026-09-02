# 03. The renderer reads combined diffs

## What to build

The diff renderer opens a file only on a `diff --git ` line, so a combined diff
— which opens with `diff --cc` — is not parsed at all and falls to the *as it
arrived* section as raw text. Teach it the combined format, so a merge commit's
resolved hunks draw as files and hunks like any other diff.

Three things differ from a unified diff, for a merge of N parents:

- The file opens `diff --cc <path>` rather than `diff --git a/<p> b/<p>`. The
  `---` and `+++` lines that follow still correct the path, as they do today.
- The hunk header carries one range per parent plus one for the result, fenced
  by N+1 `@` characters: `@@@ -1,3 -1,3 +1,3 @@@`. The number of `@` is what
  says how many parents there are, and so how many marker columns each line has.
- Each line carries N marker columns rather than one, each ` `, `-` or `+`.

The lines collapse rather than keeping their provenance: **any `+` in any column
is an added line, any `-` is a removed one, and all spaces is context.** The text
is the line with its N markers stripped.

The hunk's end is found by the same book-keeping the unified path uses,
generalised: a line with no `-` in any column is present in the result and
spends the result side's count; a line whose column for parent *i* is not `+`
spends that parent's count. The hunk is over when every count is spent.

Nothing about an ordinary unified diff changes — same parse, same output, same
tests.

## Acceptance criteria

- [ ] A two-parent patch renders as files and hunks rather than falling to the
      *as it arrived* section.
- [ ] A line added relative to either parent draws as added, one removed
      relative to either draws as removed, and one unchanged in both draws as
      context.
- [ ] Every existing diff renders byte-for-byte as it did before, with the
      existing test suite unchanged.
- [ ] A patch with three-parent hunks parses rather than being mistaken for
      content.
