**Run `verkstead ask` as a background shell command** — in Claude Code, a Bash
call with `run_in_background: true`. The call blocks until the human answers,
with no timeout, and that may be hours: the whole point is that they are not at
the terminal. A foreground tool call here hangs the session. The harness wakes
the agent when the Response arrives.

The whole of this section is the blocking ask's. A deferred one waits for
nothing, so it is an ordinary foreground call, there is no wait to lose and no
Response to read — see **Two kinds of ask**. What the closing paragraphs say
about a failure that isn't the Set holds for both.

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

**If the harness reports the background command as stopped or killed, fetch the
Set rather than asking it again.** The wait is a shell command like any other
and a harness may stop one. Nothing about the Set goes with it: it is on the
Timeline, it is answerable, and the human may already have answered it. The ask
prints the id on stderr the moment the server accepts the Set, for this one
reason:

```
# verkstead: Question Set 581 is open, and this is the wait on it.
```

So run `verkstead answers 581`. That prints the Response in the same shape the
wait would have, or says the human has not answered yet — in which case wait to
be told, because Verkstead types a line into this terminal naming the Set when
the Answers land, whether or not a wait is still standing.

**Asking the same Set again is the one thing not to do.** A repeat puts a second
copy of the same Questions on the Timeline and asks the human to answer the same
thing twice. Where a repeat is genuinely the only way — the Set was locked
unanswered, or what is being asked has really changed — say in its `preface`
which Set it repeats and why, so the human can see at a glance that the two are
one question.

The exception is a stop with **no id printed at all**, which is the ask having
been killed before the server accepted anything. There is no Set and nothing to
fetch, so send it again as a first ask.
