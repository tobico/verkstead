# 04. The narrow window's walk

## What to build

The one-pane walk under 60rem, finished for a frame that always has the
conversations beside the one pane it shows.

**The *conversations* level goes.** Today a narrow window walks conversations →
timeline → details, with the page's pane state naming which of the three is
drawn, and the root address standing at the first. Now the conversations are
always beside the pane, so the walk is timeline ↔ details and the root address
is *nothing open*: the pane column is empty paper and the collapsed sidebar
stands expanded over it (Q14), which task 01 began and this task makes the
rule rather than a case. Arriving at a conversation still lands on the end of
its record, and a phone still lands on the timeline rather than the details;
Back out of the details still reaches the timeline, and where the record is
the one event and no timeline is drawn, Back out of the composer is still out
of the conversation — to the root, where the expanded pane now stands.

**The timeline's *← Conversations* goes** (Q15), and the settings pane's with
it (Set 1270 Q24): both pointed at a level that no longer exists, the
conversations being beside them at every width. The way back is drawn by the
pane head and hidden by the frame wherever the pane it goes back to is on
screen, so what changes is that the conversations pane is now always on
screen. The details panes' *← Timeline* and *← Settings* stay exactly as they
are, as does the app's header-bar arrow for them.

**The settings page** walks the same way: settings ↔ detail with the collapsed
sidebar beside both, and the gear reads as open while expanded.

Everything the workbench asks about *beside* — whether the frame is wide
enough for the maximise toggle to have something to hide, whether a press
should also walk a narrow window into the details — is asked against the new
bands: the toggle has the collapsed pane to hide at every width (task 01 drew
it; make sure what it reasons from is the new line), and the walk into the
details still happens under 60rem only.

The tests that walk a narrow window through its panes — the zero state, the
pathing, the settings page's walk — are where this is proven; rewrite the ones
that stand at a *conversations* level. Keep the module docs of the workbench,
the settings page, the frame and the pane head true to a walk of two levels.

## Acceptance criteria

- [ ] At a phone width the root address shows the expanded sidebar over empty
      paper; pressing a card lands on that conversation's timeline with the
      sidebar collapsed beside it; Back returns to the root with the sidebar
      expanded again.
- [ ] No *← Conversations* is drawn on the timeline or the settings pane at any
      width; *← Timeline* and *← Settings* on the details panes are unchanged.
- [ ] A conversation whose record is the one event opens on the composer at a
      phone width, and Back out of it reaches the root.
- [ ] The settings page at a phone width walks settings ↔ detail with the
      collapsed sidebar beside both.
