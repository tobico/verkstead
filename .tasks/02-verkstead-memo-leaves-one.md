# 02. `verkstead memo` leaves one

## What to build

The command and the route that put a memo on the record, from a session that
may.

**The CLI.** `verkstead memo` reads markdown on stdin, the way `verkstead ask`
does, and takes `--server` from `VERKSTEAD_SERVER` the way every subcommand
does. It refuses an empty memo and one over 8 KiB before sending anything, with
a line on stderr saying which. On success it prints the memo's Event id on
stdout as YAML — `id: 42` — and nothing else, so an agent can read it back; the
confirmation in words goes to stderr. A 404 from the server is a server older
than this command, said as such; any other refusal is the server's own words,
the shape `verkstead transfer` already has.

**The route.** One beside `done`, `waiting` and `transfer` under the
Conversation's agent namespace, taking the markdown as the body. It refuses:

- when no session is running in the Conversation;
- when the running session is not a building one — the server reads what the
  session was sent for off the same register the Done signal is checked
  against. **May:** a backlog task, the finish, a stage's planning, an inline
  run, a steered instruction. **May not:** a grilling (its tail after a pick
  included), a wrap-up's review, a fix, a comment round, the session sent to
  open a pull request, a follow-up, an investigation. The refusal names the
  kinds that may;
- an empty body, and one over 8 KiB, again — the CLI's check is a courtesy
  and the server's is the rule.

Accepted, it writes the Event and the side table row in one transaction,
stamped with the words task 01 defined for the kind it was sent for — the task
number read off the register, so *the session that worked task 03* says 03 —
and `left_in` set to the running session's own Event. It answers with the new
Event id.

## Acceptance criteria

- [ ] A task session's `verkstead memo` lands a memo on the Timeline stamped
      with its task number, and the command prints the id and exits 0.
- [ ] A wrap-up session's `verkstead memo` is refused with a non-zero exit and
      a message naming the kinds that may; nothing lands.
- [ ] With no session running, the route is refused by name.
- [ ] An empty memo and one over 8 KiB are refused by the CLI without a
      request, and by the server when sent by hand.
