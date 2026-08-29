# Settings redesign

The workbench's three-pane layout becomes a shared frame that the settings
page stands on too: the conversations pane rides along everywhere, the
settings root draws as a middle pane of cards, and the settings modals become
details panes with paths of their own. Navigation deepens with it — every
details pane gets a URL nested under its Conversation or under `/settings`,
and a conversation card takes the human to the newest thing on the Timeline.
Along the way the sidebar's ⋯ menu dissolves into a gear IconButton and a
footer toggle, and Repos gain a details pane and a way to be removed.

Settled in the grilling on this Conversation: page-level navigations push
history and detail-segment changes replace it; the last *openable* Timeline
event is what a conversation card lands on; removing a Repo unregisters it
rather than deleting it, refused while live work is on it.

## Tasks

- [x] 01: IconButton and the sidebar — [details](01-iconbutton-and-sidebar.md)
- [x] 02: Extract the three-pane layout — [details](02-extract-three-pane-layout.md)
- [x] 03: Paths for conversation details panes — [details](03-conversation-detail-paths.md)
- [x] 04: Conversation cards land on the newest item — [details](04-land-on-newest-item.md)
- [x] 05: Settings on the three-pane layout — [details](05-settings-three-pane.md)
- [x] 06: Profile cards and panes — [details](06-profile-cards-and-panes.md)
- [ ] 07: The repo add pane — [details](07-repo-add-pane.md)
- [ ] 08: The repo details pane — [details](08-repo-details-pane.md)
- [ ] 09: Removing a repo — [details](09-removing-a-repo.md)
