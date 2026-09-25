# 03. Discovery

## Goal

Devices you have not typed appear under **Discovered** on the Devices pane,
each with its name and OS icon, its address and where it was found — LAN or
Tailscale — and one press to Add that runs stage 02's join. Two Verksteads on
one LAN find each other by mDNS; two on one tailnet find each other by probing
peers. Members are left out of the list. Demonstrable end to end: start a
second server, open Devices on the first, press Add on the row that appeared.

## Decisions in force

- **mDNS in-process with `mdns-sd`**, advertising and browsing
  `_verkstead._tcp.local` with a TXT record of id, name, OS and peer port
  ([ADR-0020](../../adr/0020-cluster-mode.md), *Discovery*). Shelling out to
  avahi or dns-sd was rejected: nothing to install, one behaviour on three
  platforms.
- **Tailscale by probing peers**: the peer list from `tailscale status
  --json` — the existing reading parses only `Self`, so `Peer` is new — each
  online peer's port 8423 asked for its identity in parallel with a short
  timeout, each time the pane opens rather than on a schedule.
- **A discovered row** shows name with OS icon, address, source, and Add;
  members are left out rather than drawn as linked.
- **Discovery may not cross Windows and WSL** — WSL2 is behind NAT unless
  mirrored — and that is a known limit, not a bug; the typed address covers
  it.

## Proposed tasks (provisional)

1. **Advertising** — the service registered at start with the TXT record, and
   withdrawn at shutdown.
   - A second machine's `dns-sd -B` or `avahi-browse` sees the service.
2. **Browsing and the Discovered list** — a browse held while the pane is
   open, the rows merged by device id, drawn under Discovered.
   - A device that stops advertising leaves the list within its TTL.
   - A member never appears.
3. **The Tailscale probe** — `Peer` read from the status JSON, online peers
   probed on the peer port when the pane opens, results merged with mDNS by
   id.
   - A peer that is not a Verkstead answers nothing and is not drawn.
   - Source reads *Tailscale* or *LAN*, and both where both found it.
4. **One-press Add** — the row's Add runs the join with the discovered
   address and its advertised list.

## Re-verify at start

- Stage 02 landed: the join takes an address and produces a pending row.
- `crates/server/src/remote.rs` still runs `tailscale status --json` and
  parses `BackendState` and `Self` only.
- The Devices pane from stage 01 still reads through one hook; discovery may
  want a reading of its own so the member list is not re-fetched with every
  probe.
- Whether `mdns-sd` builds on all three platforms and under the Windows
  cross-check and Wine suite the repository keeps.
