# 04. Render the conversation

## What to build

The details pane shows the conversation. Stored lines are parsed at render
time into a wire type the pane draws: the agent's prose as markdown, its
reasoning collapsed, its tool calls collapsed to a one-line summary each, and
the turns put to it.

Rendering is the server's, in the crate that already holds the parsers. The
browser is not handed a second implementation of markdown or of anything
else.

**Split on the content, not the line.** A tool result arrives inside a line
the log types as `user`, the same type a human turn arrives under. A renderer
that keys off the line's type alone shows tool output as though a person had
said it. What distinguishes them is the block inside.

**Three classes of line, not two.** Alongside the conversation itself, a real
log carries bookkeeping — mode changes, token reminders, attachments,
snapshots — which is roughly a third of every log and none of it anything a
reader came for. Bookkeeping lines fold into a single collapsed group in the
pane, expandable, so nothing is hidden and nothing is in the way. Lines that
are genuinely unrecognised keep ADR 0006's treatment: shown, as collapsed raw
JSON, so a format change announces itself instead of silently emptying the
pane.

A session with no stored lines falls back to the Capture view exactly as it
looks today. That fallback is the entire details-pane story for stub agents,
which is what keeps the test suite honest.

## Acceptance criteria

- [ ] A fixture log renders prose, reasoning, tool calls, and tool results,
      each recognisably itself
- [ ] A human turn and a tool result are visibly different things in the pane
- [ ] Reasoning and tool calls arrive collapsed; prose does not
- [ ] Bookkeeping lines appear as one collapsed group, expandable to the real
      content, and are not counted among the conversation's turns
- [ ] A line of a kind the renderer does not know shows as collapsed raw
      JSON — never as nothing
- [ ] A session with no stored lines shows the raw Capture as it does today
- [ ] Markup rendered from log content is sanitised
