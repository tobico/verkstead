# The named pipe

On Windows the server listens on a named pipe beside its TCP socket, a session
is told to ask through the pipe, and `verkstead ask` does. An AppContainer is
refused loopback and the exemption is an elevated command per machine that an
unsigned per-user install cannot ask for, so the transport a sandboxed Windows
session asks through has to be something other than TCP — and it stands here, a
stage before the container, where it can be proved without one.

What lands here: a named-pipe listener beside the TCP one serving the same
router, with the pipe's name off the Data Directory and its security descriptor
an argument stage 03 will fill; a `pipe://` spelling that `--server` and
`VERKSTEAD_SERVER` take as well as a URL, dialled through a transport of
Verkstead's own under ureq's `Connector`; and `Reachable` carrying the pipe so a
Windows session's environment names it. Linux and macOS sessions ask exactly as
before.

Roadmap stage: [02: The named pipe](docs/roadmaps/windows-sessions/02-named-pipe.md)

## Tasks

- [x] 01: A pipe beside the socket — [details](01-pipe-beside-the-socket.md)
- [x] 02: Asking through a pipe — [details](02-asking-through-a-pipe.md)
- [x] 03: Sessions ask through the pipe — [details](03-sessions-ask-through-the-pipe.md)
