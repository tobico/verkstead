# 02. A merge commit says what was resolved

## What to build

The merge commit a resolution session leaves behind stays on the Timeline — it
carries the conflict resolutions, which are real work — but it currently draws
as `0 files +0 −0` with an empty pane. That is because git says nothing about a
merge unless it is asked with `--cc`.

Both git reads that describe a commit ask for the combined diff:

    git diff-tree --no-commit-id --numstat --cc --root <sha>
    git diff-tree --no-commit-id -p --cc --root <sha>

`--cc` is safe to pass unconditionally: on anything with one parent, a root
commit included, the output is byte-for-byte the ordinary diff it is today. Only
a merge changes, and what it then reports is just the hunks that differ from
both parents — what the agent actually decided, rather than everything the base
brought in.

The renderer cannot draw a combined patch until task 03, and that is fine: a
patch it cannot parse falls to its existing *as it arrived* section, so the pane
shows the resolution as raw text in the meantime rather than nothing.

## Acceptance criteria

- [ ] A merge commit's Timeline row shows the file and line counts of the hunks
      that were resolved, not zeroes.
- [ ] An ordinary commit and a repository's first commit describe and render
      exactly as they did before, with the existing tests unchanged.
- [ ] The details pane of a merge commit shows the resolved hunks.
