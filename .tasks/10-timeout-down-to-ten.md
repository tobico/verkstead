# 10. The timeout down to ten

## What to build

Task 08 set the Windows job's `timeout-minutes` to fifteen, not the ten the
budget asks for, because nine legs of ten ran over ten minutes (Set 44,
Q1). This task takes it down to ten if task 09's figures allow.

Read the ledger's last row. If every leg's job came in under ten minutes,
change `timeout-minutes` to 10, and say in the comment at the head of the
ledger which run proved it. Also say whether the typical leg is near the seven
the budget aims for. If a leg is still over ten, leave the timeout at fifteen.
Then say in the ledger by how much the slowest leg missed and which lever is
left, and ask the human before going further. Either way, `docs/development.md`
should say what the timeout now is.

Trim the leg count if the figures leave room. A leg testing for under four
minutes is a runner doing little.

## Acceptance criteria

- [ ] `timeout-minutes` is 10 with the run that proved it named, or still 15 with the shortfall and the next lever written in the ledger
- [ ] The Windows job's comment and `docs/development.md` say what the timeout is and why
- [ ] The pull request's description carries the final figures
