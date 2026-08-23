# 11. Diff contents

## What to build

The contents treatment — the scroll-spy table of contents that askance built
and verkstead still ships, rendering as a margin sidebar where there is room
and a collapsible bar where there is not — returns to the places diffs are
read.

Two sites. The **commit diff in the details pane** gets it wired in for the
first time: a file list naming each fold in the diff, jumping to it, and
tracking the reader's place. The file paths already travel beside the
rendered diff HTML for exactly this purpose and nothing in the workbench
reads them today. And **question sets opened in the details pane** get their
contents back: it was deliberately suppressed there when the pane was one
narrow column, and the pane's new 60rem cap with centering (task 10) is what
makes the margin exist again.

Inside a pane, which of the two shapes appears is the pane's width's answer,
not the window's: a wide pane shows the sidebar in the margin beside the
capped content, a narrow one the bar. The Set page's existing contents are
the reference for behaviour — same entries, same scroll-spy, same jump
handling.

## Acceptance criteria

- [ ] The commit-diff pane shows a contents listing every file in the diff,
      jumping to and tracking the folds
- [ ] A question set opened in the details pane shows its contents again,
      matching the Set page's behaviour
- [ ] Both sites render the sidebar when the pane is wide enough for a
      margin and the collapsible bar when it is not
