# 04. What the agent is told

## What to build

The agent learns the verb and the rule, and every building session sees what
stands. Three places, on the memo's pattern exactly.

**The Guide.** A section of `verkstead guide` headed *Suggesting a preview*,
beside *Leaving a memo for the sessions after you*, and the verb's own
`--help` quoted verbatim under *The CLI contract* as the memo's is. The rule it
states is the brief's: suggest a preview **the moment there is anything that
could be tried**, whether or not a manual test looks called for, so that it is
there whenever the human decides one is; keep it current as the way to run the
work changes, the same title replacing the command; retire one that no longer
runs. It says what the command is — one line, written for the shell the prompt
names, run in the conversation's own worktree as the human would type it — and
what is refused. It says a preview is for the human's hand and not a test
suite: *Run the server*, *Build and open the app*, *Run the tests* where
watching them is the point.

**The skills.** A short section in each building skill that teaches the memo —
implementing, instruction, next-stage and next-task — pointing at the Guide's
section, with one example in the skill's own voice. A task session is told to
suggest one as soon as its slice can be tried and not to wait for the finish;
the finish is told to make sure what stands still runs.

**The prompt.** Every building session's prompt gains a heading of its own
listing the standing previews, title and command, so a session keeps them
current rather than suggesting the same one again. Unlike the memos' heading,
this one is shown **when the list is empty too**, over a line saying none
stands yet and that the session should suggest one as soon as there is
anything to try — the heading is the hint. It names the shell the terminal
will run the command in, read the way the terminal reads it, so the agent
writes for the right one. Sessions that may not suggest one — a grilling, an
investigate session, a wrap-up — carry no such heading, as they carry no retire
line for a memo.

## Acceptance criteria

- [ ] `verkstead guide` prints the section and quotes `verkstead preview --help` word for word, and each of the four building skills points at it.
- [ ] A building session's prompt lists the standing previews with their commands and names the terminal's shell; with none standing it shows the heading over the hint to add one. A grilling session's prompt has no such heading.
- [ ] Nothing refuses a done signal, or says anything at one, over a session that left no preview.
