# 02. The Guide, the transfer help and the refusal

## What to build

Everywhere the agent reads about moving says what task 01's prompt section now
says: the list is every device of the cluster, the permitted ones are marked,
and a device that is not permitted is asked for.

- **The Guide's *Moving the work to another device*** (`crates/cli/guide/core.md`)
  stops saying a session's prompt names only the devices it may move to and that
  a prompt naming none may move nowhere. It says the prompt names this device's
  OS and every other device, which of them this Conversation may move to, and
  that for one it may not, the session asks the human with `verkstead ask` — the
  human can tick the device from the Set's Attach menu under *May be transferred
  to*, and `verkstead transfer` reads the list live, so it may be called once
  the Set is answered. Ticks are still the consent: moving to a permitted device
  needs no ask.
- **`verkstead transfer --help`** (the `Transfer` verb's doc in the CLI) drops
  "a prompt that names none is a session that may move nowhere" and says the
  same in brief.
- **The server's refusal** of a call on an unticked device (`transfers.rs`,
  `by_the_session`) stops pointing at the Transfer dialog and names the route an
  agent actually has: ask the human with `verkstead ask`, who can tick it from
  the Attach menu. Stage 04 later turns this refusal into a request; write
  nothing about that.

The words agree with the prompt section's; follow `docs/copy.md`. Update the
guide suite's phrase checks (`crates/cli/tests/suite/guide.rs`) and the
transfer refusal's tests to the new words.

## Acceptance criteria

- [ ] `verkstead guide`'s *Moving the work to another device* says the prompt
      names every device and marks the permitted ones, and says how to ask for
      one that is not permitted; it no longer says "may move nowhere".
- [ ] `verkstead transfer --help` says the same in brief.
- [ ] `verkstead transfer` on an unticked device is refused with words naming
      `verkstead ask` and the Attach menu's *May be transferred to*, and nothing
      moves.
- [ ] The Guide, the help and the prompt section use the same words for the
      same things.
