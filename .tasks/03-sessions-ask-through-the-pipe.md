# 03. Sessions ask through the pipe

## What to build

A Windows session is told to ask through the pipe, and does — proved on the
`windows-2025` job by a real `verkstead ask` whose Set lands on the Timeline
without touching TCP.

**`Reachable` carries the pipe beside the socket address.** It is the one thing
that says where Verkstead is reachable from inside a session, and its
Conversation-scoped base is what makes a session's Question Sets that
Conversation's. It gains the pipe as something a server may also have opened,
and hands out whichever the Platform a session runs on asks through — the pipe
on Windows, the URL as before on Linux and macOS. Its existing constructor stays
what it is: eight test call sites build one from an address alone and none of
them is about a pipe.

Nothing else in the server names what a session asks through. The startup line
logs the address it was told to listen on, the desktop app opens the human's own
browser on it, and the push notifier's audience is the push endpoint — none of
those is this, and none of them changes.

**The environment.** `Sandbox::surface` sets `VERKSTEAD_SERVER` from whatever
`Reachable` hands it, so a Windows session's names the pipe and every other
platform's names what it named before. The Executable's startup probe — a
`verkstead guide` run in the environment a session would get — is unaffected,
because it asks nothing of a server.

**The proof runs the real command.** The CLI crate already stands a session's
ask up end to end on Unix: a real repository, a real worktree, a real
Conversation, a real server, and the CLI run inside the real rendering with the
Set read back out of the store and answered through the browser's route. This
task's proof is that file's Windows sibling, with the server listening on a pipe
and the session's environment naming it. It belongs there rather than in the
server crate's Windows sessions suite, which cannot run the command at all: that
suite is a server-crate test, so what a session finds first on its `PATH` is the
test binary's own directory and there is no `verkstead.exe` in it. The
`windows-2025` job runs the whole workspace's tests, so the CLI crate's are on
it already.

**And the glossary.** `CONTEXT.md` says what a session reaches Verkstead
through; on Windows that is now a pipe, and the roadmap has each stage update
the terms as its piece lands.

## Acceptance criteria

- [ ] A Windows session's environment names the pipe, scoped to its
      Conversation, and `{base}/api/v1/sets` still composes onto it; a Linux or
      macOS session's names exactly what it did before.
- [ ] On the `windows-2025` job, `verkstead ask` run in a Windows session's own
      environment against a server listening on a pipe lands its Set on that
      Conversation's Timeline and takes the human's Response back, with nothing
      listening on TCP for it to have used instead.
- [ ] `CONTEXT.md` says what a session asks through on Windows, and the Unix
      suites are untouched and green.
