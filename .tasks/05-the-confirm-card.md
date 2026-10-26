# 05. The confirm card

## What to build

The Update from task 03 goes through a confirm card first. Its question is
*Update <device>?* — *Update wolf?* — and its body names the version it will
update to and **the sessions running on this device**, saying they will be
stopped and resumed. Update goes ahead regardless: refusing while a session
runs, as the Annex's Update does (`running_there`), was not taken (ADR-0027).
The words are the deferred wording Set's (1381); follow `docs/copy.md`.

- The running sessions are read off this device's own Conversations, the way
  the status button reads a session's state; the Devices view (or the route
  the card reads) carries them by title so the card can name them. With none
  running the body says only the version.
- The card is the Devices list's existing confirm — `Confirm` in
  `web/src/settings/Remote.tsx`, which Unlink already uses.
- A restart already stops every session and startup re-drives each driven
  Conversation through the harness's own resume; prove that holds across this
  restart rather than assuming it.

## Acceptance criteria

- [ ] On wolf, the card reads *Update wolf?* with the version in its body.
- [ ] With two sessions running, the card names both, Update proceeds, and both are resumed after the relaunch.
- [ ] With none running, the card names only the version.
