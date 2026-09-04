# 05. Closing

## What to build

A tab can be closed on purpose and never by accident, and no terminal outlives
what it was opened for.

Close is on a menu rather than a control on the tab: the existing context menu,
opened by a right-click on the tab on a pointer device and by a long press on
the tab on a finger — the Conversations list already tells a long press from a
right-click, and this reuses that rather than a second idea of it. The menu
holds one row, **Close**.

Close asks a new endpoint, delete on `/terminals/{n}`, which ends the shell the
way a session is ended: hung up, then killed where it lingers. The tab then goes
by task 04's rule, so closing the only tab yields a fresh one.

Every terminal of a Conversation is ended when the Conversation closes, before
its Worktree is removed and beside the session that close already ends, so the
removal never has a shell standing in the directory. And every terminal goes
when the server does, held under the same process-group care the sessions'
sandboxes are, so none is orphaned.

## Acceptance criteria

- [ ] Right-click on a tab offers Close, and Close ends that shell and its tab
      goes.
- [ ] A long press on a tab on a touch device opens the same menu.
- [ ] Closing a Conversation with terminals open ends them and removes its
      Worktree cleanly.
- [ ] Stopping the server leaves no terminal shell running.
- [ ] A server test shows delete ends the process and takes the terminal off
      the register.
