**Run `verkstead ask` as an ordinary foreground shell call, and pass it a large
`timeout`.** The shell tool runs the command synchronously and hands back what
it printed, so there is nothing to background and nothing to poll — but it kills
a command that outruns the timeout it was given, and this one blocks until the
human answers, which may be hours: the whole point is that they are not at the
terminal. So say a timeout measured in hours rather than leaving it to the
default: `86400000` milliseconds is a day, and an ask answered in minutes comes
back in minutes whatever was passed.

The whole of this section is the blocking ask's. A deferred one waits for
nothing, so it needs no timeout of its own and there is no Response to read —
see **Two kinds of ask**. Everything below about a failure that isn't the Set
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

**Do whatever the answers cannot invalidate before the ask rather than while it
runs.** The call holds this turn where it stands until the Response lands, so
nothing else is happening in the meantime — and once it is out, don't speculate
about what the human will say and don't start work the answers might throw away.

The wait reconnects on its own through a dropped connection or a server that
went away, for as long as it takes, so nothing here is a failure to act on. What
it has to say about that goes to stderr and is written as a YAML comment — so
the output handed back still parses as the Response, which is what **Reading the
Response** below describes.
