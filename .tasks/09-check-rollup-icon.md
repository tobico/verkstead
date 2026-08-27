# 09. Check rollup and icon

## What to build

The pull request card gets a check-status icon on its right side, echoing the
icons GitHub uses: a green pass mark, a red failure cross, a pending dot while
anything still runs.

The server already fetches per-check name, status and link from GitHub, but
only the wrap-up watcher consumes it and nothing survives the poll. This task
makes the data durable and puts the aggregate on the wire:

- Each time the checks watcher polls, it persists the latest rollup for the
  pull request.
- The conversation view carries the aggregate so the card can draw it. The
  aggregate reads any-failed as failed, else any-running as running, else
  passed; a pull request with no checks stored shows no icon.
- The stored rollup may be stale on a conversation nothing watches (Done,
  Follow-up) — that is accepted; task 10 freshens it whenever the details pane
  is opened.

## Acceptance criteria

- [ ] While a wrap-up watches its checks, the card icon tracks them: failed,
      running and passed each draw their GitHub-style icon
- [ ] A pull request with no stored checks shows no icon
- [ ] The rollup survives a server restart
- [ ] The icon carries words for a screen reader
