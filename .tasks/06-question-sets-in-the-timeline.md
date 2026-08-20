# 06. Question Sets in the Timeline

## What to build

The grill loop closes. A session inside the sandbox puts a Question Set to the
human, the Set lands on its Conversation's Timeline, the human answers it in
the workbench or on the phone, and the session — which was idling — carries on.
Brief to questions to answers to more questions, driven from the GUI alone.

What attributes a Set to a Conversation is a **conversation-scoped
`VERKSTEAD_SERVER`** injected into the sandbox, so the bundled CLI says which
Conversation it is asking from explicitly. Nothing is inferred from the project
name or the branch — those are derived for display, and two Conversations on
one Repo would be indistinguishable by them.

Blocking asks only. The session idles until the Response arrives, exactly as
every ask does in askance today; Deferred Asks are stage 05 and nothing here
should anticipate them.

The Set becomes a Timeline Event, summarised in the Timeline as the design's
table of number, question and answer, with the full answer-set document in the
details pane. The rendering that draws a Set already exists and is not being
rewritten — this is that rendering reached through a Conversation rather than
through a standalone list.

The phone answers the same Sets through the same route, so a Set answered on
the phone unblocks the session just as one answered in the workbench does.

## Acceptance criteria

- [ ] A session in the sandbox submits a Set that lands on its own
      Conversation's Timeline, with no inference from project or branch
- [ ] Two Conversations grilling the same Repo at once each receive only their
      own Sets
- [ ] The Timeline summarises a Set as a number/question/answer table; the
      details pane shows the whole document
- [ ] Answering in the workbench unblocks the waiting session
- [ ] Answering on the phone unblocks it identically
- [ ] A full loop — brief, questions, answers, further questions — completes
      without leaving the GUI
