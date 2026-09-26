# 01. Advertising

## What to build

This **Device** announces itself on the LAN, so that another Verkstead on the
same one can find it without anybody typing an address. The service is
`_verkstead._tcp.local`, registered in-process with the `mdns-sd` crate as the
server starts, and its TXT record carries the four things a row on somebody
else's **Discovered** list will be drawn from: the **Device Id**, the name and
the OS word out of the **Device Reading**, and the port the **Peer Listener**
answers on. The instance is named by the Device Id, because two Verksteads on
one machine are two devices and a hostname cannot tell them apart.

**Against the port the listener landed on** rather than the one the
configuration asked for. A `:0` is a port the operating system chose, which is
what a suite binds, and an advertisement naming any other number is one nothing
can be dialled at.

**And it can be turned off.** Advertising puts this machine's hostname, its OS
and its Device Id on a LAN that may not be the human's alone, so there is an
off-switch in the same shape the update check's is — a flag, its environment
variable, and a NixOS option beside `peerListen` that maps to it. On by default,
for the reason `openFirewall` is on by default: a discovery nothing can hear is
a feature that silently does not work, with nothing on either machine saying
why.

**And the host's firewall has to let the multicast in.** mDNS is UDP 5353 and a
NixOS host firewalls by default, so the rule the module already opens for the
peer port grows that port beside it — otherwise advertising on a NixOS host is
an advertisement nobody ever hears.

**Withdrawn on the way out**, which is the first ordered stop this server has
ever had. A signal it is asked to stop on withdraws the advertisement — the
goodbye that takes the row off every other machine's list at once — and then
lets the process end as it always did; there is still no graceful shutdown of
anything else, and nothing else waits on this. A killed server leaves the row to
run out on its own TTL instead, which is the same thing that covers a machine
whose lid shut, and the code should say so rather than imply the withdrawal is
the only way a row goes.

## Acceptance criteria

- [ ] `avahi-browse -rt _verkstead._tcp` on the LAN lists this device once, with the Device Id, the name, the OS word and the bound peer port in its TXT record.
- [ ] Two Verksteads on one machine, each with a peer listener of its own, advertise as two instances, each naming its own port.
- [ ] A suite browsing the same service reads the TXT record back, and finds nothing at all where the off-switch has been thrown.
- [ ] A server told to stop withdraws the advertisement before it ends, and the module's own check says what the new option defaults to and that the firewall rule carries UDP 5353.
