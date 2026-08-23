# 02. Retire the Direction state

## What to build

With the pick riding the closing Set, nothing ever waits in Direction — remove
the state from the ladder. A Conversation moves Draft → Grilling → Implementing
→ Wrapping → Done, and the answered proposal Set is the Timeline's record of
the choice: the separate chosen-direction Event goes away, and so do the
standalone chooser, the endpoint that served it, and the state's refusal
variants.

The stored direction on a Conversation is re-documented as the latest pick —
what the human most recently chose on a proposal Set — rather than a value that
implies a state.

No compatibility path: a database migration collapses the retired state, and
any Conversation caught sitting in it (accepted proposal, no session, waiting
on the removed chooser) is moved to Aborted — it has no session to receive
anything and no chooser left to press. This is a single-user tool; in-flight
Conversations are finished or aborted before upgrading.

Sidebar and status wording follow: nothing ever reads "choosing a direction"
again — between the closing Set landing and its answer, the Conversation is
Grilling and blocked on you, which the unanswered Set already says.

## Acceptance criteria

- [ ] The Direction lifecycle value is gone from the store, with a migration;
      a pre-existing row in that state comes out Aborted.
- [ ] The standalone chooser, its endpoint, and the chosen-direction Timeline
      Event are removed from server, render, and web; the answered proposal
      Set is the visible record of the pick.
- [ ] A picked Response moves the Conversation Grilling → Implementing with no
      intermediate state observable anywhere (API, sidebar, Timeline).
