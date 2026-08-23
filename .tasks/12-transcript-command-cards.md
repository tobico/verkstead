# 12. Transcript command cards

## What to build

In the transcript detail view, a command execution and the result that
answers it stop being two separate collapsed rows. A matching pair merges
into **one expandable item, presented as a card**: collapsed, it carries the
tool name and the one-line description it carries today; expanded, it shows
the input and then the output, in that order.

Success stays quiet — no status word on a pair that worked. A failed pair
says so collapsed: a red "failed" in the summary line, using the same
stopped-red the failed result uses today, so failures are scannable without
expanding anything.

Matching is between a command turn and the result turn that answers it; a
command still awaiting its result, or a result with no visible command,
renders sensibly on its own. Every other turn kind — prose, the human's
turns, thinking, unread lines, the bookkeeping fold — is untouched. Turns
are parsed server-side and the client renders what it is given; whether the
pairing happens at parse or at render is the builder's call, but the
transcript's incremental, cursor-fed loading must keep working.

## Acceptance criteria

- [ ] A command and its result render as one collapsed card showing tool
      name and description, expanding to input then output
- [ ] Failed pairs show a red "failed" while collapsed; successful pairs
      carry no status word
- [ ] Unmatched command or result turns and all other turn kinds still
      render, and incremental transcript loading is unaffected
