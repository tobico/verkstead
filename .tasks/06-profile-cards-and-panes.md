# 06. Profile cards and panes

## What to build

The Agent Profiles section of the settings pane becomes cards, and its modal
becomes two details panes.

Each saved Profile is a `CardButton` showing its name, its models, and the
broken-pair warning where its pair has gone bad; the mounted paths and the
agent type come off the card and move into the details pane, where there is
room for them. Pressing a card opens the edit form at
`/settings/profiles/:id`, filled in with what the Profile says, and the card
reads as open while its pane is.

The **Remove** button moves into that pane beside the edit form — a row's
inline Edit/Remove pair is gone with the rows. Removal keeps its refusals
(a Profile a Conversation is set to run under is refused with the reason),
said in the pane.

Adding a Profile becomes a plus IconButton (Font Awesome solid `plus`) on
the section's heading line, replacing the quiet text button: pressing it
opens the same form blank at `/settings/profiles/new`, and the IconButton
reads as open while that pane is. Save and refusal behaviour is unchanged —
every named refusal is still said beside the form.

With both panes standing, the Profiles modal is deleted. The detail paths
follow the settings page's navigation rules from task 05.

## Acceptance criteria

- [ ] Profile cards show name, models and any broken-pair warning; pressing
      one opens the filled-in form (with paths and agent type shown) at
      `/settings/profiles/:id`
- [ ] Remove lives in the edit pane and still refuses with a reason when the
      Profile is in use
- [ ] The plus IconButton opens the blank form at `/settings/profiles/new`,
      reads as open while it is, and the modal is gone with every refusal
      still said
