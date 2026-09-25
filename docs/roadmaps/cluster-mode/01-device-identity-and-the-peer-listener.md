# 01. Device identity and the peer listener

## Goal

A Verkstead knows what it is and can be asked. At first start it invents a
device id and a self-signed certificate beside `workbench.key`; it listens on
a TLS peer listener of its own, every interface, port 8423, that presents that
certificate and asks the caller for one without insisting on it; an
un-gated identity endpoint on that listener answers with the device's id, name,
OS and addresses; and the
settings page has a **Devices** section whose list holds this device alone —
name with an OS icon, *this device*, its addresses, no Unlink. Demonstrable
end to end: start two servers, curl one's identity from the other's machine,
open Devices on each and see a WSL read as *Linux (WSL)*.

## Decisions in force

- **A device is a random id and a certificate**, both made at first start and
  kept in the Data Directory beside `workbench.key`, read back at every start
  ([ADR-0020](../../adr/0020-cluster-mode.md), *A device is an id and a
  certificate*). The id is short enough to sit in a URL segment. The tailnet
  node name and the hostname were rejected as ids: one is gone off the tailnet,
  the other collides.
- **The name is the hostname read at each start, with an OS icon.** Nothing
  is typed. WSL is detected from the kernel release and reads *Linux (WSL)*,
  because Windows and its WSL share a hostname.
- **The peer listener is separate from the workbench listener**, TLS with
  mutual TLS over it. `--peer-listen` and `VERKSTEAD_PEER_LISTEN`, default
  `0.0.0.0:8423`, and a `peerListen` option in the NixOS module. The
  workbench listener, `tailscale serve` and the key gate are untouched, and
  the session API stays loopback-and-pipe.
- **Client authentication is optional at the handshake and membership is
  enforced per route.** A client certificate is asked for once per connection,
  before any path is known, so the handshake cannot be the thing that decides
  which endpoints a caller reaches: it accepts whatever certificate arrives —
  or none — and hands it to the routes, and middleware over the gated ones is
  what consults the member list. A verifier that refused a non-member outright
  would refuse the join that makes one, which is what stage 02 posts.
- **Three routes stand outside the member gate**, and they are the whole of the
  un-gated surface: the identity endpoint, which asks for no certificate at
  all; the join post, which is a non-member by definition and whose certificate
  is pinned into the pending request it creates; and the dial-back that answers
  a join, matched against the certificate that pending join is holding rather
  than against the member list. Everything else on this listener is a member's
  or is refused. Stage 01 has the first of the three and refuses the other two
  along with everything else, there being no join to hold yet.
- **Devices is a card and a pane like every settings section**, following the
  four-step pattern the Remote access section set: a word in the section list,
  a card, a `Match` in the details, a read hook.
- **A device advertises all its addresses** — tailnet name and IP where
  Tailscale is up, LAN IPs — read off the machine at each answer rather than
  configured.

## Proposed tasks (provisional)

1. **Identity on disk** — the id and the certificate made at first start,
   read back after, with the platform's file mode.
   - A second start reads the same id; deleting the files makes fresh ones.
   - The certificate's fingerprint is stable and printable.
2. **The peer listener** — a second axum server over rustls on the peer
   address, presenting the certificate and requesting a client certificate
   without requiring one, with the membership check as middleware over every
   route but the identity endpoint.
   - A call with no client certificate reaches the identity endpoint, and
     nothing else on the listener.
   - A call with an unknown client certificate reaches the identity endpoint
     too, and is refused by every gated route.
   - The NixOS module opens the option and the VM test binds it.
3. **The identity endpoint and the machine reading** — id, name, OS with WSL
   detection, addresses; the wire type exported to TypeScript.
   - Under WSL the OS reads *Linux (WSL)*; elsewhere the platform's own word.
   - Addresses list the tailnet name and IP first where Tailscale is up.
4. **The Devices section** — card, pane, route word, the list with this
   device's row.
   - The row carries the OS icon and *this device* and offers no Unlink.
   - The section's line on the card says how many devices are linked.

## Re-verify at start

- The workbench key still lives in `crates/server/src/key.rs` with the file
  beside the settings files; the identity files follow the same shape.
- The one listener is still bound in `crates/server/src/lib.rs` around the
  `--listen` argument, and `rustls` is present only as a `reqwest` feature —
  a server-side TLS stack is new.
- The settings section pattern is still `WORDS` in
  `web/src/settings/openings.ts`, a card and a `Match` in
  `web/src/settings/SettingsPage.tsx`, and a read hook over `useReading`.
- The hostname is still read only in `crates/server/src/onboarding.rs` and
  the platform word only in `crates/server/src/platform.rs`.
- `nix/module.nix` still has the single `listen` option and the VM test in
  `nix/vm-test.nix`.
