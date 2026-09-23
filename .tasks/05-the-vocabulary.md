# 05. The vocabulary

## What to build

CONTEXT.md is rewritten where it says a steer is a modal. The **Steer** entry
describes the press writing a pending steer, the item drawn last on the
Timeline, the form as its details pane saved as it is typed, Cancel leaving
the Conversation stopped, Resume and Close discarding the pending steer, a
second press selecting it, the waiting mark, and the record as the whole form.
The **Timeline** and **Event** entries say that a pending steer is drawn on
the Timeline without being an Event: it is the one thing on the pane that has
not happened yet, which is why it is drawn last and boards no Share. The
**Resume** and **Follow-up** entries are checked for the modal and corrected
where they name it. The docs under `docs/` are swept for the same — the
development guide and the steer roadmap's stages are history and stay as
written, but anything describing the workbench as it is now is corrected.

The module docs on the form, the actions menu, the Timeline and the server's
steering module were rewritten by the tasks that changed them; this task reads
them once more against the vocabulary and settles any word that disagrees with
CONTEXT.md.

## Acceptance criteria

- [ ] CONTEXT.md's Steer entry describes the item and the pending steer, and no
      entry describes a modal or a cancel that closes one.
- [ ] The Timeline and Event entries say what a pending steer is and why it is
      drawn last.
- [ ] `grep -i modal` over CONTEXT.md and the module docs of the steer form,
      the actions menu and the steering module names nothing a steer does.
