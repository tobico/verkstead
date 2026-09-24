# 04. Quick open

## What to build

**An endpoint that lists a root's files**, which is git's own list rather than
a walk: what git tracks, plus what it does not track and does not ignore. One
ask answers every root the Conversation has, each list under the root it
belongs to, and it is read afresh every time the palette opens — there is no
watcher until stage 04, and a list read once would go stale the first time the
agent wrote anything.

**Capped per root**, because a monorepo answers with a few hundred thousand
paths and none of them would be read on a page. A root whose list was cut short
says so in the answer, so the palette can say it rather than quietly matching
over half a checkout. A root git will not answer about lists nothing, which is
the folder listing's own rule for its own reason.

**Ctrl+P opens the palette**, taking the browser's Print with it — VS Code's
own key, and Print is still on the browser's menu. It hangs where Ctrl+S and
the four keystrokes of stage 02 already hang, so it arrives whichever half of
the pane the hands were in.

**A field and a list of matches**, built out of the picker behaviour the app
already has rather than a dropdown written again. The match is on the page:
**subsequence, preferring whole path segments**, which is what VS Code's feels
like at this size — typing the start of a file's name should beat the same
letters scattered through a directory nobody meant. Each row names the file and
the root it is in, best first, because two roots can hold the same path.

Enter opens the pick as a tab in the active group, the arrows walk the rows,
and Escape leaves nothing behind — no tab, no pane state, and the focus back
where it was.

## Acceptance criteria

- [ ] Ctrl+P opens the palette, and typing part of a path lists matches from
      every root, each row naming its root, the best match first.
- [ ] Enter opens the pick as a tab in the active group; Escape closes the
      palette and leaves nothing behind.
- [ ] A root whose list was cut short says so in the palette, and a
      Conversation with no Worktree opens a palette that says there is nothing
      to search.
