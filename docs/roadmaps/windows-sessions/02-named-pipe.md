# 02. The named pipe

## Goal

On Windows the server listens on a named pipe beside its TCP socket, a session
is told to ask through the pipe, and `verkstead ask` does — proved on the
`windows-2025` job by a session whose ask never touches TCP. Linux and macOS
sessions ask exactly as before.

## Decisions in force

- **Why a pipe at all** ([ADR-0014](../../adr/0014-windows-sessions.md),
  grilling Q13): an AppContainer is refused loopback, and the exemption is an
  elevated command per machine an unsigned per-user install cannot ask for.
  Binding a tailnet or LAN address as well was rejected — whether the
  firewall counts a machine's own address as loopback is unverified, and a
  session would depend on an interface existing.
- **Its own stage, before the container**: it changes the CLI's public
  surface, and it can be proved on every platform's tests without a
  container. Stage 03's grilling starts with the transport already standing.
- **The server side**: a `tokio::net::windows::named_pipe` server accepting in
  a loop beside `axum::serve` on the TCP listener, each connection served by
  hyper's connection API over the same router. The pipe's name is derived
  from the Data Directory so two servers on one machine do not collide, and
  its security descriptor grants the container's identity when stage 03
  supplies one — until then, the user's own. The pipe is Windows-only; no
  Unix-socket twin, because nothing on the other platforms needs one.
- **`VERKSTEAD_SERVER` names the pipe** on a Windows session, in a spelling the
  CLI can tell from a URL — the Conversation-scoped base stays a base, so
  `{base}/api/v1/sets` still composes. The exact spelling is the stage's to
  choose; it should survive a human pasting it into a terminal.
- **The CLI side**: `--server` accepts that spelling, and `Client::new` builds
  the ureq agent with a transport of Verkstead's own for it. ureq 3 exposes
  this under `ureq::unversioned::transport::Connector`, which is outside its
  semver promise — pin ureq's minor and say so where the Connector is written.
  The long poll, the backoff and the YAML-comment chatter on stderr are all
  above the transport and unchanged.
- **`Reachable`** gains the pipe beside the socket address, and
  `Sandbox::surface` sets `VERKSTEAD_SERVER` from whichever the Platform's
  session asks through. The Executable's startup probe (`verkstead guide` in a
  session's environment) is unaffected — it asks nothing.

## Proposed tasks (provisional)

1. **The server listens on a pipe** — accept loop beside the TCP listener,
   the router served over each connection, the name derived from the Data
   Directory. Accepts: on Windows, a request over the pipe returns what the
   same request over TCP returns; a second server on another Data Directory
   opens a second pipe; the pipe is absent on Linux and macOS builds.
2. **The CLI asks through a pipe** — the `--server` spelling, the Connector,
   `Client::new` choosing it. Accepts: `verkstead ask` against a pipe posts
   the Set and long-polls the Response; a URL still works; the transport
   reports a missing pipe as the same "server down" failure the TCP one does.
3. **Sessions are told the pipe** — `Reachable` carries it, the Windows
   session's `VERKSTEAD_SERVER` names it. Accepts: the Windows end-to-end
   suite's stand-in agent runs `verkstead ask` through the pipe and the Set
   lands on the Timeline; the Conversation-scoped base still composes.
4. **The pipe's security descriptor** — a grant to an identity supplied by the
   caller, defaulting to the user's own, ready for stage 03's container.
   Accepts: a connection from another user is refused; the descriptor is one
   argument the container stage can fill.

## Re-verify at start

- ureq's version in `Cargo.lock` and whether `Connector` still lives under
  `unversioned`; whether a newer ureq stabilised it.
- How `lib.rs` runs `axum::serve` after stage 01 — the Windows job may have
  reshaped startup — and where a second listener sits in the shutdown story.
- `Reachable::at` and `asking_from`, and every place the server names its own
  address (the startup line, the desktop app's open-in-browser, the push
  notifier).
- Whether the `windows-2025` runner allows named pipes from a test process
  (it should; it is a plain Win32 API).
