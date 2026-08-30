# 04. The updates renderer

## What to build

Grok Build's log made readable: its lines drawn as the conversation they record,
its bookkeeping folded away under its own names.

**Grok's log is not a line per thing said**, which is what both readers here
assume today. Claude writes one line per turn and codex writes one per drawn
item; Grok writes agent-protocol session updates — each line keyed on
`sessionUpdate` rather than on `type`, so every one of them currently falls past
both readers and stands in the conversation as raw JSON. The kinds are the
protocol's: the agent's message, its thinking, the human's turn, a tool call and
the updates to it, and grok's own `turn_completed` carrying the usage of a
finished turn.

Nothing has to be told which backend wrote a line: the kinds are disjoint
across the three, so a line falls to the reader that knows its own and to the
fold below all of them where none does. Keep it that way.

**The agent's prose arrives as a stream of chunks.** A single thing the agent
said is many `agent_message_chunk` lines, and drawn one at a time that is a
Transcript of fragments and a turn count in the hundreds. **The chunks are
linked and the pane joins them** — the answer the human took, and the same
answer the reader already gives a tool call and its answer, for the same
reason: a reading stops wherever the log had got to, so a batch can cut between
any two chunks, and a reader that merged them would have to hold a turn back or
send it twice. So each chunk carries what says which turn it belongs to, the
join happens in the pane over the whole record it has accumulated, and the
reading of one line stays a reading of one line.

Two things follow that are worth watching:

- **The row counts turns, not fragments.** The Timeline's count is the same
  reading as the pane's with the rendering left out, so whatever joins chunks
  has to leave the count counting what the pane draws as one. A count of chunks
  would put a two-sentence answer at forty turns.
- **The link has to survive a re-read.** A running session's Transcript is
  re-read incrementally and the viewer reconciles rows by their number, so a
  chunk arriving in a later batch has to land in the row its earlier siblings
  are already drawn in.

Everything else is ADR-0006's rule unchanged, and this is the third backend to
follow it: a whole line of a kind nothing here knows folds under the name the
log gave it, and a *block* of an unknown kind stays where it was said as the
JSON it is — a line folded silently is a line filed, a block folded silently is
a turn with a hole in it. Claude's and codex's renderings do not move.

## Acceptance criteria

- [ ] A real Grok session's log draws as the conversation it records — the
      agent's prose whole rather than in fragments, its thinking, the turns put
      to it, and its tool calls paired with their answers.
- [ ] The turn count on the Timeline row counts what the pane draws as one, and
      a Transcript re-read a batch at a time comes out the same as one read
      whole.
- [ ] A line of a `sessionUpdate` nothing here knows folds into bookkeeping
      under its own name; a block of an unknown kind stays in the turn it was
      said in; Claude's and codex's renderings are unchanged.

## What the real log turned out to be

**Grok writes what the agent said whole.** The premise above — that a single
thing the agent said is many `agent_message_chunk` lines — does not hold
against grok 1.0.13. Driven for real, in its TUI and headless, with a second
and a half between one fragment of a sentence and the next, grok's store wrote
**one line when the message was over** every time: the file was watched as it
grew, and it sat at two lines for eighteen seconds while the model streamed and
then took the whole message at once. `chunkId` in a line's `_meta` counts the
fragments grok saw; the line carries the message they made.

So none of the machinery the premise called for was built. There is no link
field on a turn, nothing joins anything in the pane, and a chunk is a turn —
which leaves the count counting what the pane draws as one, and an incremental
re-read the same as one read whole, because a line is still read on its own. A
release that starts writing the fragments would show as a Transcript of
fragments rather than as a hole: visible, and one reading away from fixed.

The rest of the log is as the task has it, and the kinds are the protocol's:
`user_message_chunk`, `agent_message_chunk` and `agent_thought_chunk` are the
conversation, `tool_call` is a call and the `tool_call_update` carrying a
finished status is the answer to it — the updates before that one are the call
still running, and fold. Grok's lines carry no `type` at all, which is what
tells them apart from Claude's and codex's.

## What is still waiting

There is no xAI account on this machine, so the log the fixture is taken from
was written by a grok run **outside Verkstead**: grok 1.0.13 installed under a
`GROK_HOME` of its own from `@xai-official/grok`, pointed by
`GROK_XAI_API_BASE_URL` at a stand-in xAI Responses API that streams slowly and
calls `run_terminal_command` and `todo_write`. Every line of the fixture is
grok's own, written by grok's own store; only the model behind it was not.
