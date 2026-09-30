# 02. `verkstead transfer`, and the move it asks for

## What to build

The agent's end of the call, following `verkstead done`'s pattern: **a request
the server writes down and something else acts on**, reached through the
Conversation-scoped base URL in `VERKSTEAD_SERVER`, so nothing names the
Conversation but where the call was made from.

- **The CLI verb** `verkstead transfer <device>` beside `done` and `waiting`,
  with the same `--server` flag (a base URL or `pipe://<name>`, env
  `VERKSTEAD_SERVER`). `<device>` is a device's **name or its id**. Accepted, it
  prints a confirmation on stdout — the work will move once this turn is over,
  so say anything left to say now — and exits 0. Refused, it exits non-zero with
  the reason on stderr and nothing is recorded.
- **The route** under the Conversation-scoped session API (`…/api/v1/transfer`,
  beside `done` and `waiting`, outside the workbench key's gate). Any session
  running for the Conversation may make the call — grillings included.
- **What it refuses, each by name:**
  - a name that matches no device of the cluster, or matches two (naming both,
    with their ids, so the agent can retry by id);
  - the device the work is already on;
  - a device **not permitted** — neither ticked on this Conversation (task 01)
    nor the drafting device;
  - whatever stage 09's **preflight** says the device lacks — unreachable, no
    matched Repo, harness absent — run synchronously before the call returns,
    in the same words the Transfer dialog shows, so the agent can pick another
    device or ask the human;
  - a Conversation that is not movable (stage 09's rule).
- **Recorded as stage 09's transfer request**, with *who asked* on it: the
  human's press or the session's call. **The later request wins** whichever
  asked, and the Timeline names whichever made the move.
- **The move is stage 09's mover, unchanged in what it carries**: it sees the
  session out and then moves the record, the checkout and the arrival, and the
  far end carries the conversation on (stage 10). The wording where the move is
  told reads **Transferred to B at the session's request** for a session-asked
  move — on the arriving copy's Timeline, which is the live record, and on the
  copy left behind.

In this task the mover waits for the session to end the way it does for a
press; ending it at the turn's end is task 03. So the proof here is a stub
agent that calls `verkstead transfer` and then exits.

## Acceptance criteria

- [ ] In the two-server transfer suite, a stub agent on A that runs
      `verkstead transfer B` (by name, and again by id) and exits is moved to B,
      carried on there, and B's Timeline says *Transferred to B at the
      session's request*.
- [ ] Off the list, ambiguous (both named), the current device, unreachable,
      and an unmatched Repo each exit non-zero with the reason on stderr and
      leave no request behind.
- [ ] A human press after the agent's request (or before it) is the one that
      moves, and the Timeline says who asked.
