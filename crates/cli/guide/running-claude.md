**Run `verkstead ask` as a background shell command** — in Claude Code, a Bash
call with `run_in_background: true`. The call blocks until the human answers,
with no timeout, and that may be hours: the whole point is that they are not at
the terminal. A foreground tool call here hangs the session. The harness wakes
the agent when the Response arrives.

The whole of this section is the blocking ask's. A deferred one waits for
nothing, so it is an ordinary foreground call and there is no Response to read
— see **Two kinds of ask**. Everything below about a failure that isn't the Set
holds for both.

Pipe the Set in on stdin — no file to name, and nothing left behind:

```
verkstead ask <<'YAML'
title: …
questions:
  - label: Q1
    text: …
YAML
```

Quote the heredoc delimiter (`<<'YAML'`, not `<<YAML`) so the shell leaves the
Set alone — backticks and `$` are ordinary characters in prose and in a diff.

While waiting, do any work that does not depend on the answers. Don't speculate
about what the human will say, and don't start work the answers might throw
away.

The wait reconnects on its own through a dropped connection or a server that
went away, for as long as it takes, so nothing here is a failure to act on. What
it has to say about that goes to stderr and is written as a YAML comment — so
the file a harness collects the two streams into still parses as the Response,
which is what **Reading the Response** below describes.
