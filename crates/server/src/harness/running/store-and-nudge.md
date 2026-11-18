**Run `verkstead ask` as an ordinary foreground call.** It comes back as soon as
the server has stored the Set — there is nothing to wait for on this end — and
what it prints is the stored Set rather than a Response: the `id` it was stored
under, and when the server took it.

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

Then **end the turn.** Do any work that does not depend on the Answers first —
but don't speculate about what the human will say, don't start work the Answers
might throw away, and don't poll for them. Say what is now waiting on the human,
and stop.

Verkstead types a line into this terminal when the Response lands, and that line
is what starts the next turn. Fetch the Answers with the `id` the ask printed:

```
verkstead answers 42
```

Stdout is the Response YAML and nothing else, exactly as **Reading the Response**
below describes it. A Set nobody has answered yet is a non-zero exit rather than
something to idle on, so run this when the line has said the Answers are there.

A Set sent with `--deferred` is never nudged about and has no Answers to come
back for — see **Two kinds of ask**. Everything below about a failure that isn't
the Set holds for both.
