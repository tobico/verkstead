# 01. Device identity and the peer listener

## Goal

A Verkstead knows what it is and can be asked. At first start it invents a
device id and a self-signed certificate beside `workbench.key`, re-issuing the
certificate as its expiry comes near; it listens on a TLS peer listener of its
own, every interface, port 8423, that presents that certificate and asks the
caller for one without insisting on it; an un-gated identity endpoint on that
listener answers with the device's id, name, OS and addresses; and the **Remote
access** pane gains a **Devices** section whose list holds this device alone —
name with an OS icon, *this device*, its addresses, no Unlink. Demonstrable end
to end: start two servers, curl one's identity from the other's machine, open
Remote access on each and see a WSL read as *Linux (WSL)*.

## Decisions in force

- **A device is a random id and a certificate**, both made at first start and
  kept in the Data Directory beside `workbench.key`, read back at every start
  ([ADR-0020](../../adr/0020-cluster-mode.md), *A device is an id and a
  certificate*). The id is short enough to sit in a URL segment. The tailnet
  node name and the hostname were rejected as ids: one is gone off the tailnet,
  the other collides.
- **The certificate is renewed rather than issued once for ever**
  ([ADR-0020](../../adr/0020-cluster-mode.md), *A device is an id and a
  certificate*): an expired one is refused at the handshake, so a single
  long-lived certificate would take every link in a cluster down on one day,
  the only way back being to re-link every device by hand. A validity long
  enough never to matter was not taken for that reason. What belongs to this
  stage is the validity, the re-issue a good while before the expiry, and
  keeping both certificates over the changeover; the announcement of the new
  fingerprint to every member is stage 02's, there being no member to tell yet
  — the re-issue here simply has nobody to announce to and says so.
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
- **Devices is a section inside the Remote access pane**, a third one beside
  the `reach` and `key` sections already there — which is where the Brief put
  the linking, and linking is how this machine is reached as much as the serve
  and the key are. So there is no word in `WORDS`, no card and no route of its
  own: what the pane grows is a section and a reading, and the Remote access
  card's line says how many devices are linked beside what it already says.
  A settings section of its own was considered — the pane is long already, and
  Devices brings a list, Add, Discovered and a pending row — and was not taken:
  the Brief was specific, and the pane is built out of sections for this.
- **A device advertises all its addresses** — tailnet name and IP where
  Tailscale is up, LAN IPs — read off the machine at each answer rather than
  configured.

## Proposed tasks (provisional)

1. **Identity on disk** — the id and the certificate made at first start,
   read back after, with the platform's file mode.
   - A second start reads the same id; deleting the files makes fresh ones.
   - The certificate's fingerprint is stable and printable.
2. **The renewal** — the validity written down, a re-issue when the expiry is
   near enough, and the outgoing certificate kept beside the new one until
   nothing is owed an announcement.
   - A start with the expiry near re-issues; one with it far off does not.
   - Both fingerprints are printable over the changeover, and the listener
     presents the old one while any member is unacknowledged.
   - With no members, a re-issue completes at once.
3. **The peer listener** — a second axum server over rustls on the peer
   address, presenting the certificate and requesting a client certificate
   without requiring one, with the membership check as middleware over every
   route but the identity endpoint.
   - A call with no client certificate reaches the identity endpoint, and
     nothing else on the listener.
   - A call with an unknown client certificate reaches the identity endpoint
     too, and is refused by every gated route.
   - The NixOS module opens the option and the VM test binds it.
4. **The identity endpoint and the machine reading** — id, name, OS with WSL
   detection, addresses; the wire type exported to TypeScript.
   - Under WSL the OS reads *Linux (WSL)*; elsewhere the platform's own word.
   - Addresses list the tailnet name and IP first where Tailscale is up.
5. **The Devices section** — a third section in the Remote access pane, with
   its own reading, and the list holding this device's row.
   - The row carries the OS icon and *this device* and offers no Unlink.
   - The Remote access card's line says how many devices are linked.
   - The section reads nothing of the settings query, as the two beside it do
     not: what it draws is read off the machine.

## Re-verify at start

- The workbench key still lives in `crates/server/src/key.rs` with the file
  beside the settings files; the identity files follow the same shape.
- The one listener is still bound in `crates/server/src/lib.rs` around the
  `--listen` argument, and `rustls` is present only as a `reqwest` feature —
  a server-side TLS stack is new.
- The Remote access pane is still `web/src/settings/Remote.tsx`, drawn from
  `SettingsPage.tsx` as `RemoteCard` and `RemotePane`, and still built out of
  the `reach` and `key` sections with a reading of its own rather than the
  settings query — which is the shape a third section follows. Nothing is added
  to `WORDS` in `web/src/settings/openings.ts`.
- The hostname is still read only in `crates/server/src/onboarding.rs` and
  the platform word only in `crates/server/src/platform.rs`.
- `nix/module.nix` still has the single `listen` option and the VM test in
  `nix/vm-test.nix`.
- Whether anything in the tree generates a certificate already — `rcgen` or its
  like is new — and what validity it defaults to, since a default accepted
  without looking is exactly the silent expiry this stage is written against.
