# Discovery

Devices nobody has typed appear under **Discovered** in the **Devices** section
of the **Remote Access** pane, each with its name and the mark for its OS, the
address it was found at, where it was found — LAN or Tailscale — and one press
to **Add** that runs stage 02's **Join**. Two Verksteads on one LAN find each
other by mDNS, advertising and browsing `_verkstead._tcp.local` in-process; two
on one tailnet find each other by probing the peers `tailscale status` names.
**Members** are left out, and so are this device itself and any device a Join is
already pending for.

What it is for is the setup cluster mode was written for: a human with two or
three machines should not have to know any of their addresses to link them.
Discovery may not cross a Windows machine and the WSL on it — WSL2 sits behind
NAT unless mirrored networking is on — and that is a known limit rather than a
bug, with the typed address as what is left.

Roadmap stage: [03: Discovery](docs/roadmaps/cluster-mode/03-discovery.md)

## Tasks

- [x] 01: Advertising — [details](01-advertising.md)
- [ ] 02: Browsing and the Discovered list — [details](02-browsing-and-the-discovered-list.md)
- [ ] 03: The Tailscale probe — [details](03-the-tailscale-probe.md)
- [ ] 04: One-press Add — [details](04-one-press-add.md)
