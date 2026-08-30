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
