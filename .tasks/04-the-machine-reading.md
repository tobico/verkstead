# 04. The machine reading

## What to build

The identity endpoint's answer grows the three things read off the machine rather
than configured: the device's **name**, its **OS**, and its **addresses**
(ADR-0020, *A device is an id and a certificate*, and *Addresses*). All three are
read at the moment they are answered rather than held from startup, because a
laptop moves between the LAN and the tailnet and DHCP moves everybody.

**The name is the hostname, and nothing is typed.** It is read at each start, the
way the install run already reads one for its status line — that is the only place
in the tree that reads a hostname today, and the reading belongs somewhere both
can reach rather than being written a second time. A machine that will not say
what it is called has a fallback already established there.

**The OS is a word, and the word does not exist yet.** The platform enum has three
arms and no human reading of any of them, so this task writes one. **WSL is
detected from the kernel release and reads *Linux (WSL)***, because a Windows
machine and its WSL share a hostname and the OS is the only thing that tells them
apart — which is the setup this whole roadmap was written for. Elsewhere it is the
platform's own word.

**The addresses are every address this device has**, in the order a peer should
try them: the tailnet name and IP first where Tailscale is up, then the LAN IPs.
The tailnet half comes from the same reading of `tailscale status --json` that the
Remote access pane already stands on — a machine with no Tailscale, or one not up,
simply contributes nothing to the list rather than failing the answer. The address
typed at link time, in stage 02, is only the first one known; this list is what
makes a moved laptop still reachable.

The wire type lives in the render crate with every other view type and is
exported to TypeScript, since the Devices section in task 06 draws it. Keep the
reading itself on the server side: the render crate compiles to wasm and must stay
free of server-only dependencies.

## Acceptance criteria

- [ ] Under WSL the OS reads *Linux (WSL)*, detected from the kernel release;
      on every other platform it is that platform's own word.
- [ ] The addresses list the tailnet name and IP first where Tailscale is up, then
      the LAN IPs — and a machine with no Tailscale, or one not up, answers with
      the LAN IPs alone rather than failing.
- [ ] The identity endpoint answers id, name, OS and addresses, and the name is
      the hostname read off the machine with nothing configurable about it.
- [ ] The wire type is exported to TypeScript, and the hostname is read in one
      place rather than two.
