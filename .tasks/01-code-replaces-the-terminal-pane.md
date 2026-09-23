# 01. Code replaces the Terminal pane

## What to build

The Terminal pane becomes **Code**, and nothing it does changes. The details
pane moves to `/code`, the icon beside Share on the Timeline's header becomes
the `code` icon with the label to match, and the word the pane is named by in
the openings module becomes `code`. The old path redirects to the new one, so a
link somebody saved or a browser that remembered it still lands somewhere.

This is a move rather than a change of behaviour. The pane still opens a shell
on load, still closes a tab from its context menu, and still stands on the same
register, the same sockets and the same refusal states. Those two rules invert
in task 02, and keeping them here is what makes this task's red suite mean one
thing.

The vocabulary moves with it, which is the whole of the record sweep: the
glossary and ADR-0019 already say what Code is, so what is left is every place
in the code and the documentation that still says *Terminal pane* — the module
docs of the pane and of the attached-terminal component, the Timeline's icon
label, and the comments in the workbench and the openings module that list the
word-named panes. The icon still says why it is disabled where there is no
Worktree.

The pane is still **not a record**: no Capture, no Event, nothing in a Share,
and every place the workbench says so of a terminal now says it of Code.

## Acceptance criteria

- [ ] Every terminal test passes against the pane at its new name and path,
      with no change to what any of them assert about behaviour.
- [ ] The Timeline's header opens Code with the `code` icon, and the pane
      stands at `/code` under its Conversation.
- [ ] The old `/terminal` path redirects to `/code` under the same
      Conversation, with a test standing on it.
- [ ] A grep for the old pane's path and title finds only the redirect.
