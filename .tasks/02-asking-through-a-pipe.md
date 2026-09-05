# 02. Asking through a pipe

## What to build

`verkstead ask` and `verkstead answers` reach a server over a named pipe as well
as over a URL. `--server` and `VERKSTEAD_SERVER` take either, and which one they
were given is read off the spelling.

**The spelling is `pipe://<name>`**, and the rest of the base composes onto it
the way a URL's does: a Conversation-scoped base is `pipe://<name>/conversations/7`
and `{base}/api/v1/sets` is still what the client asks. Chosen over Windows' own
`\\.\pipe\<name>` because a human pastes this into a terminal and backslashes
are the shell's, and over hiding that spelling behind a scheme for the same
reason. The CLI tells the two apart by the scheme and by nothing else.

**ureq never sees it.** ureq refuses any URL whose scheme is not http or https —
it asks the scheme for a default port and there is none — so the spelling is
parsed by the CLI, which keeps the pipe's name for the transport and dials a
placeholder http URL carrying the path. Nothing resolves that host: the agent is
built with a resolver of Verkstead's own that answers without asking anybody, and
a connector that opens the pipe and pays the address no attention.

**The connector is outside ureq's semver promise.** `Connector` and `Transport`
live under `ureq::unversioned::transport`, whose own documentation says it does
not yet follow semver — so **pin ureq to its minor** where the manifest today
says a major, and say why both there and where the connector is written. What a
`Transport` owes is buffers, a write of the whole output buffer, a wait for
input, and an honest answer about whether it is still open; ureq's own TCP
transport is the shape to mirror.

**The deadline is the part that does not come free.** A pipe opened as an
ordinary file has nothing like a socket's read timeout, and the client's whole
retry story stands on one: the long poll asks the server to hold for thirty
seconds inside a sixty-second request timeout, and a wait that overran is a wait
to reopen. Rebuild that deadline on the pipe — Windows has overlapped I/O and a
way to cancel an outstanding operation — rather than shipping a Windows ask that
a wedged server hangs forever.

**Nothing above the transport changes.** The submit that does not retry, the
reconnecting long poll, the backoff, the statuses read as answers, and the
retries reported on stderr as YAML comments are all as they are, and are what
the pipe is asked to satisfy rather than something to re-decide.

On Linux and macOS a `pipe://` spelling is refused with a line saying pipes are
Windows', rather than failing as an unparseable URL.

## Acceptance criteria

- [ ] On Windows, `verkstead ask` against a pipe posts the Set and long-polls
      until the Response lands, and `verkstead answers` fetches one by id —
      printing what the same run against the same server's URL prints.
- [ ] A URL still works on every platform, the default is still the loopback
      URL, and a `pipe://` spelling on Linux or macOS is refused with a line
      saying why.
- [ ] A pipe nothing is listening on fails the way a refused TCP connection
      does: the submit says the server could not be reached and exits non-zero,
      and the wait retries on the same backoff with the same stderr comment.
- [ ] A request that overruns the client's timeout over a pipe is given up on
      and reopened, as it is over TCP, and ureq is pinned to a minor with the
      reason written down.
