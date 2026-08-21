# 05. Switch summaries and evidence

## What to build

The two places that quote a session in miniature start quoting what it said
rather than what its terminal last printed: the Timeline Event's summary
line, and the evidence an Interruption carries.

Both currently take the last line of terminal output and strip the control
sequences out of it, which yields whatever the agent's interface happened to
be drawing at that moment — a spinner, a box edge, a truncated status. The
latest thing the agent actually said is in the Transcript, and it reads as
prose because it is prose.

The escape-stripper stays exactly where it is and keeps its job: it is the
fallback for any session with no Transcript rows, which is every stub session
in the test suite and every session on a backend that leaves no log. This is
the last task because it is the one that makes the first four visible in the
Timeline rather than only in the details pane.

## Acceptance criteria

- [ ] A real session's Timeline Event summarises with the agent's own latest
      prose
- [ ] An Interruption raised on a real session carries prose as its evidence
- [ ] A session with no Transcript rows summarises exactly as it does today,
      via the stripper — stub-agent tests unchanged
- [ ] The summary keeps updating as the session runs, not only at its end
