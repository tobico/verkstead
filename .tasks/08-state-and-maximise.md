# 08. State above the pane, and maximise

## What to build

**Code's state is held above the details pane.** The pane's frame draws one
thing at a time and unmounts what it is not showing, so today's terminals are
read back from the register on every open. Code's tabs, its buffers and its
unsaved text have to survive being swapped for an Event and back — so what
holds them lives above the frame's switch, per Conversation. The device's
storage that survives a *reload* is stage 02 of the roadmap; what this task
delivers is the swap, plus the browser's own warning on leaving the page with
dirty text.

A terminal whose shell has ended since is dropped rather than drawn: the
register is still what says which shells are live.

**And a maximise toggle in the pane's header**, which hides the sidebar and
the Timeline so the editor has the window. Off when Code first opens, and
remembered per device beside the pane widths — the same storage, namespaced
the same way. Below the breakpoint where the details pane already has the
window there is nothing for it to hide, so it is not drawn.

## Acceptance criteria

- [ ] Opening an Event and coming back to Code finds the same tabs, the same
      active one, and the unsaved text still in it.
- [ ] Leaving the page with dirty text warns, the browser's own way.
- [ ] The maximise toggle hides the sidebar and the Timeline, and the choice
      survives a reload.
- [ ] On a window narrow enough that the details pane already has it, the
      toggle is absent.
