# 05. OpenCode's records drawn, and the whole stage proved

## What to build

The renderer that turns the records task 04 stored into the conversation they
record, and then the whole stage run end to end on the real thing.

**One reader, a fourth backend's shapes.** The Transcript is read by one code
path that keys off what a line says it is, and the same reading serves the pane,
the Timeline row's turn count and the sentence a Notice quotes. This adds
OpenCode's kinds to it rather than a reader of its own.

What opencode writes is a small tagged set, and most of it maps onto turns that
already exist: the turn put to it, the assistant's own record — whose content is
a list of prose, reasoning and tool calls with the tool's state and result inside
it — a shell command it ran, and then its own bookkeeping: the agent it switched
to, the model it switched to, the system and synthetic text it wrote to itself,
and the summary it replaced a long conversation with. A closed list and a
fall-back past it, exactly as the three backends before it have: a kind nobody
here has heard of folds away under its own name rather than standing in the
conversation.

**Two backends now use the same words for different shapes.** opencode's records
are tagged `user` and `assistant` — the same two words Claude's lines carry —
but what hangs off them is not the same: opencode puts the assistant's content at
the top level where Claude puts it under a message, and a tool call carries its
own state and result rather than being answered by a separate line. A reader that
keys on the word alone will send opencode's records down Claude's arm and draw
every one of them as unreadable. Tell them apart on something that is actually
different, and write down what that is, because it is the kind of thing a fourth
backend's arrival will test again.

**Then prove the stage.** Everything OpenCode needs is built by here: the type
and its account, the line, the idle signature, the Guide's tailoring and the held
ask, the store reader and this renderer. What is left is running a Conversation
all the way through under an OpenCode Profile on a real provider account —
grilled, with a Set actually answered from a phone, and built — and reading the
result back off the Timeline as a human would.

**Update the vocabulary as this piece lands.** `CONTEXT.md` describes how a
Transcript is found and how idle is judged in terms of the backends that had
landed; OpenCode is a fourth answer to the first — found, but out of a database
rather than a file — and its Profile is a fourth account shape. The roadmap says
each stage updates the terms as its piece lands, and this is the stage's last
task.

## Acceptance criteria

- [ ] A real OpenCode session's Transcript draws as the conversation it was:
      prose, reasoning, tool calls and their answers, and the turns put to it,
      with opencode's own bookkeeping folded away and a kind this build has
      never seen folded under its own name.
- [ ] Claude's, Codex's and Grok Build's Transcripts draw exactly as they did,
      and the Timeline row's turn count and the quoted sentence are the same
      reading as the pane's for all four.
- [ ] A whole Conversation runs under an OpenCode Profile — grilled with a
      blocking Set answered away from the terminal, then built — and its
      Timeline reads back as a record of what happened.
- [ ] `CONTEXT.md` says how an OpenCode session's Transcript is found and how
      its idle is judged, in the vocabulary already there.
