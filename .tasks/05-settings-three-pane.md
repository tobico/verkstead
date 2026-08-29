# 05. Settings on the three-pane layout

## What to build

The settings page stands on the shared three-pane layout: the conversations
pane down the left exactly as the workbench draws it (New conversation, the
gear — now showing open — and the archived toggle all ride along, it being
one component), the settings root as the middle pane, and a details pane that
is bare paper until something on the root opens into it.

The settings root pane keeps the page's reading order — credentials, then
Agent Profiles, then Repos — but the credentials summary becomes a **github
card**: a `CardButton` carrying the token's state (its last four characters
and when it was saved), the git author's name and email, and the
missing-credential warnings. Pressing it opens the credentials form in the
details pane at `/settings/github`, replacing that modal; the form keeps its
write-only token field, its one-save-for-both semantics and its refusals. The
card reads as open while its pane is.

The update banner stays at the top of the settings pane when a release
waits, and the Notifications switch stays on the pane head's line — neither
is a card, both were settled to stay.

Detail selection under `/settings` follows the same rules as under a
Conversation (task 03): the path names the pane, detail changes replace,
entering `/settings` pushes. On a phone the walk is conversations →
settings → detail; the settings pane's back link says "← Conversations"
(consistent with the Timeline's, over the Brief's "back to workbench"
phrasing) and, like every pane-back, shows only on the one-pane layout —
on wider windows the conversations pane is simply present.

The Profiles and Repos sections keep their current rows and modals in this
task; their conversion to cards and panes is tasks 06–09.

## Acceptance criteria

- [ ] `/settings` draws all three panes on a wide window, with the shared
      widths and dividers, and walks one pane at a time on a phone with
      "← Conversations" as the way back
- [ ] The gear in the conversations pane reads as open, and opening a
      conversation from that pane still works from the settings page
- [ ] The github card opens the credentials form at `/settings/github` in
      the details pane — the modal is gone, every refusal still said — and
      the card reads as open while it is
