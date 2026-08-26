# 12. Archive Closed conversations

## What to build

Let a Closed conversation be archived, hiding it from the sidebar. Settled by
grilling:

- **Closed only.** A Done conversation is closed first, then archived; no
  other state offers it.
- **Storage** follows the house pattern for new per-conversation facts: a
  side table whose row's presence is the flag (the STRICT conversations
  table is left alone), with a route to set it. Archiving is reversible, so
  no confirmation dialog.
- **The control** is a row in the conversation pane's ⋯ actions menu, beside
  Close, shown only when the conversation is Closed.
- **The list** stops showing archived conversations (the reveal toggle is
  task 13; until it lands archived ones are simply absent). The list
  endpoint carries or filters the flag; nothing leaves the Timeline —
  archiving touches only the sidebar list.
- **Vocabulary**: add Archive-for-conversations to CONTEXT.md beside the
  Locked entry task 11 wrote.

## Acceptance criteria

- [ ] Archiving a Closed conversation removes it from the sidebar list, and
      survives a reload
- [ ] The Archive row appears only on Closed conversations
- [ ] Store and route tests cover archive and its refusals; web tests cover
      the menu row
