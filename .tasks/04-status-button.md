# 04. The StatusButton

## What to build

A large two-line button at the top of the Timeline pane's sticky chrome —
below the pane head, above the pinned cards — drawn in every state of the
Conversation. Pressing it opens the conversation actions menu, and a
chevron-down at its right edge says so; the ⋯ trigger that opened that menu
from the pane head is removed, along with the header's Done/Closed word, its
Blocked-on-you/Stopped badge, and its Waiting-on-checks label — the StatusButton
is the one place this pane says status. (The jump-to-the-blocking-notice press
the old badge carried is deliberately dropped, not relocated: the notice is on
the record, and the record opens at its end.)

**Line 1** is a title/subtitle pair in the pattern the sidebar's conversation
card draws — status word bold, lifecycle state understated beside it (the
state words as `states.ts` spells them). The status word is drawn from facts
already on `ConversationView` (plus task 02's `waiting`), highest precedence
first:

| Status | Drawn from | Accent |
|---|---|---|
| Waiting on you | `waiting` | yes |
| Blocked | `blocked_on` set and not `stopped_by_hand` | yes |
| Stopped | `stopped_by_hand`, or `ready_to_resume` with no stop recorded | no |
| Waiting on checks | `waiting_on_checks` | no |
| Running | `working` | no |
| Driven | `driven` with no session at this instant | no |

A running session that has gone quiet says nothing extra — no idle word, no
ring — *Running* covers it. On Draft, Done and Closed none of the statuses
apply: line 1 is the bare state word with no subtitle, since status and state
would be the same word twice.

**Line 2** is regular text: the running session's profile name and prettified
model with no separator ("Work Fable 5"), read from task 03's fields through
task 01's `prettify`. On a stop that came from an exhausted usage window
(`resets` set) it reads the short form "Out of window until {resets}". In every
other moment with no session registered — including mid-drive between steps,
and on Draft/Done/Closed — it reads "No agent running".

The accent color marks line 1 only for *Waiting on you* and *Blocked*; every
other status is the regular text color.

## Acceptance criteria

- [ ] The button draws in the sticky chrome in every state, and its press opens the actions menu; the ⋯ and the three header badges are gone
- [ ] Line 1 follows the precedence table, accents only the two attention statuses, and collapses to the bare state word on Draft/Done/Closed
- [ ] Line 2 shows profile + pretty model while a session runs, the out-of-window line on a window stop, and "No agent running" otherwise
- [ ] Web tests cover the status precedence, the accent rule, and each of line 2's three readings
