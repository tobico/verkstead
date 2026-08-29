# 02. The waiting fold on ConversationView

## What to build

The Conversation pane's read (`ConversationView`) gains a `waiting` boolean —
whether something about this Conversation is waiting on the human — folded by
the same rule the sidebar row's (`ConversationEntry.waiting`) already is: an
ask left open, or driving that has stopped without the human. The server
assembles both views in the same place and a comment there already notes the
sidebar's fold is the rule to share; fold once and use it from both so the two
reads can never disagree about the same Conversation.

Regenerate the TypeScript wire types so the field reaches the viewer. Nothing
in the viewer consumes it yet — task 04 does — so this slice is demonstrated by
the field being on the wire and correct.

## Acceptance criteria

- [ ] `ConversationView.waiting` is on the wire, true exactly when the sidebar row's `waiting` is true for the same Conversation
- [ ] The fold is one piece of code used by both views, not two copies of the rule
- [ ] Server tests cover the fold on the Conversation view — an open ask, a stop, and a quiet Conversation that waits on nothing
