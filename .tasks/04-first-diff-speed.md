# 04. Speed up the first diff

## What to build

Opening a diff detail pane for the first time can be much slower than later
opens. Diagnosed on the server: syntax highlighting loads a lazily-built
syntax-definition set — a few megabytes, deserialized on the first request
that needs it, synchronously, on an async runtime worker — and the
parse-and-highlight render of a diff also runs inline on the async handler
(only the git call is on the blocking pool).

Two settled fixes, and only these — wider changes (render caching, size
caps) were deliberately left out of this batch:

1. Force the syntax set during server startup, off the request path, so no
   user request ever pays the load.
2. Run the diff render (commit diffs and set diffs alike) on the blocking
   pool, so a large diff cannot stall unrelated requests.

## Acceptance criteria

- [ ] The first diff opened after a server start takes about as long as the
      second
- [ ] Diff rendering no longer runs on the async runtime's worker threads
- [ ] Startup does not block serving on the warm-up
