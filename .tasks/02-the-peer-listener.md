# 02. The peer listener

## What to build

Devices talk over a **listener of their own**: TLS, every interface, port 8423 by
default (ADR-0020, *The peer listener, and mutual TLS*). This task binds it,
presents the certificate task 01 made, and puts one route on it — the identity
endpoint, answering the device's id and its certificate's fingerprint — so that
the listener can be reached and proved from another machine.

**A second server rather than a share of the first.** The workbench listener is
untouched: loopback, `tailscale serve` in front of it, plain HTTP to the browser,
the Workbench Key gate over it. Two things ruled out sharing it — the served
address carries Tailscale's certificate rather than ours, so a link pinned on a
fingerprint could never go through it, and the workbench port speaks plain HTTP
to the browser and the serve alike. A listener that sniffed the first byte for a
TLS handshake was considered and rejected as a trick where a port would do. The
Conversation-scoped session API stays loopback-and-pipe and outside this listener
entirely.

**How it is configured:** `--peer-listen` and `VERKSTEAD_PEER_LISTEN`, default
`0.0.0.0:8423`, alongside the existing `--listen`. The NixOS module's own option
is task 05.

**Client authentication is asked for and not insisted on.** A client certificate
is requested once per connection, before any path is known, so the handshake
cannot be the thing that decides which endpoints a caller reaches: it accepts
whatever certificate arrives — or none — and lets the request through. A verifier
that refused a non-member outright is what the join of stage 02 could never have
got through. The gate that acts on the certificate is task 03; here the handshake
simply must not refuse anybody.

Two things to know before starting:

- **The server's HTTP serve does no TLS.** The peer listener accepts connections
  and completes handshakes itself before handing each one to its router, rather
  than being a second call to the same serve helper the workbench uses. `rustls`
  is in the lock only as `reqwest`'s feature, so the server-side stack is a new
  direct dependency.
- **On Windows the one router is already served over two listeners** — the socket
  and the named pipe, raced against each other, either one ending being the
  server ending — while the other platforms serve the socket directly. The peer
  listener joins that arrangement, so whatever shape it takes has to be written
  for both.

The identity endpoint answers as a wire type, and every wire type in this tree
lives in the render crate, which is compiled to wasm for the browser and must
stay free of server-only dependencies. So the type goes there and is exported to
TypeScript with the rest; the reading behind it stays on the server side. Here it
carries the id and the fingerprint — task 04 grows it the name, the OS and the
addresses.

## Acceptance criteria

- [ ] A `curl` over TLS to the peer port from another machine reads the device's
      id, and the certificate the handshake handed over is the one the startup
      line printed.
- [ ] A caller that presents no client certificate completes the handshake and
      reaches the identity endpoint; so does one presenting an unknown
      certificate.
- [ ] `--peer-listen` and `VERKSTEAD_PEER_LISTEN` bind where they say, default
      `0.0.0.0:8423`, and the startup line says where the peer listener is.
- [ ] The workbench listener, the Workbench Key gate, `tailscale serve` and the
      loopback-and-pipe session API behave exactly as before, on Windows and
      elsewhere both.
- [ ] `CONTEXT.md` gains **Peer Listener**.
