# 04. The one-event layout

## What to build

While a Conversation's Timeline holds exactly one Event, the timeline pane is
not drawn at all and the brief pane expands to cover the timeline's space in
addition to its own regular width. The hiding is complete: nothing from the
timeline's head — title, share, status, pins — is carried over or kept in a
strip; the composer alone fills the widened pane. A second Event of any kind
brings the timeline back.

On a narrow screen, where the workbench walks one pane at a time, Back from
the brief pane goes straight to the conversations list — the timeline simply
is not there to walk through, in either direction.

The pane-width machinery should degrade gracefully: the divider between the
sidebar and the expanded pane keeps working, and returning to the three-pane
arrangement when the timeline reappears restores the remembered widths.

## Acceptance criteria

- [ ] A Conversation with exactly one Timeline Event draws two panes — the
      conversations list and the widened brief pane — with no timeline head
      furniture anywhere; a second Event restores the three-pane layout and
      its remembered widths.
- [ ] On a narrow screen the pane walk skips the timeline both ways: opening
      the Conversation lands on the brief pane and Back returns to the
      conversations list.
- [ ] The composer stays centered at its measure inside the widened pane.
