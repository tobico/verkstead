# 05. The option and the VM test

## What to build

The peer listener becomes something a NixOS host configures rather than something
only a flag reaches: a **`peerListen`** option in the module beside the `listen`
option already there, passed to the unit the same way, and the VM test binding it
and reading the identity endpoint back over TLS.

The default is `0.0.0.0:8423`, as the flag's is — a peer listener has to answer
every interface, because the device dialling it may be on the LAN or on the
tailnet and neither is the loopback.

**The module opens the port on the host's firewall**, through an `openFirewall`
option that is **on by default**: a peer listener nothing can reach is a linking
that cannot happen, and a NixOS host firewalls by default, so a module that left
the port shut would ship a feature that silently does not work. A host that wants
it shut turns the option off.

The VM test is the one place a module option is proved rather than asserted, so
what it does is the whole point: it brings a Verkstead up through the module,
reads its own identity endpoint over TLS on the port the option named, and finds
the id, the name, the OS and the addresses in the answer. It is already the shape
for this — it runs the source build deliberately, and it already drives the
workbench's health check and a `verkstead ask` against the running server, so the
peer read joins what it does rather than needing a harness of its own.

Two things about running it here: the flake checks are only exercised on the
default branch, so `nix flake check` has to be built locally on this branch, the
VM check included, before this task is called done. And the VM build is slow —
declare the wait rather than letting the session look stopped.

## Acceptance criteria

- [ ] The module has `peerListen`, defaulting to `0.0.0.0:8423`, and the unit's
      command line carries it; the existing `listen` option behaves as before.
- [ ] `openFirewall` is there, on by default, and opens the peer port; turning it
      off leaves the port shut.
- [ ] The VM test reads the identity endpoint over TLS on the port the option
      named and finds the device's id, name, OS and addresses in the answer.
- [ ] `nix flake check` passes locally on this branch, the VM check included.
