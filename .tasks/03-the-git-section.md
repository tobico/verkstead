# 03. The Git section

## What to build

GitHub and git author becomes **Git**, at `/settings/git`, and takes Conflict
resolution in. The old slug and `/settings/conflicts` are no such page. The two
links on the workbench's share notice that point at the GitHub pane follow the
rename.

The pane reads, top to bottom: the GitHub token block as it is; the git author's
name and email; a **Conflict resolution** subheading whose select is labelled by
the subheading alone, offering Merge and Rebase and saving on pick as it does
today; a **Sharing** subheading over a native checkbox from task 02 labelled
**Share to pull request on Done**, saving on tick; then Ignored comments and
the pane's Save, which still writes the token and the author. The select gets
no second visible label under its subheading, and the sharing checkbox gets no
explanatory paragraph under it; the gist note goes.

The Conflicts card and its pane go, along with the descriptions of the two
strategies and the force-push warning. The Git card's summary stays what it is:
the saved token, the author, and the warnings about a missing scope or an
unverified token, which are computed and stay.

Tests: the conflicts web tests fold into the settings tests for the Git pane,
the sharing tests move from a switch to a checkbox, and the routes test learns
the new slug and the two retired ones. The design doc's settings block names
the github card.

## Acceptance criteria

- [ ] Picking Rebase on the Git pane saves at once and changes how the next conflicted pull request is resolved.
- [ ] The share checkbox saves on tick, and the pane's Save still writes the token and the author.
- [ ] The rebase warning and every static note are gone from the pane, and `/settings/git` opens it while `/settings/github` and `/settings/conflicts` are no such page.
