# 01. The backend interface and the WSL probe

## What to build

The Keeper's server reaches its Annex through an interface written once.
Under ADR-0024, a Docker or Lima Annex on a Mac, or a Windows VM beside a
Linux desktop, can later be another implementation of it. Only the `wsl.exe`
implementation is built now, and nothing of the others is written.

The interface:

- **make**: import an image as the distro named `Verkstead`, with its disk in
  a given directory (`%LOCALAPPDATA%\Verkstead\wsl` in production).
- **boot** and **terminate**.
- **run inside**: a command in the distro, either as root or as the service
  user (the NixOS module's `verkstead`), returning its output and exit status.
- **read a file**: a file inside the distro.
- **probe**: what state the machine is in, returned as a value:

```rust
enum Probed {
    NoWsl,               // wsl.exe absent, or "not installed"
    NeedsRestart,        // WSL turned on, the restart still owed
    VirtualizationOff,   // firmware virtualization disabled: WSL cannot run
    NoDistro,            // WSL answers, no `Verkstead` distro
    Present { running: bool, networking: Networking },
}
enum Networking { Nat, Mirrored }
```

It also needs a reading of the **two addresses** that task 05's link takes.
This only works while the distro is running:

- **The Annex's address.** Under NAT, the distro's own IPv4 on `eth0` at the
  Annex's peer port 8425. Under mirrored, the loopback at 8425.
- **`keeper_at`**, the address the Annex reaches the Keeper at. Under NAT,
  the gateway of `ip route show default`, run inside as root (decided: the
  `vEthernet (WSL…)` adapter's name has changed between WSL versions, so it
  is not read from the Windows side), at the Keeper's peer port. Under
  mirrored, the loopback at the Keeper's peer port.
- **The networking mode** comes from `wslinfo --networking-mode` run inside.
  Only the probe gives it; nothing infers it.

The pieces are pure functions over what `wsl.exe` and the commands inside
print, plus a thin process layer. Every child gets `.unseen()`, so no console
window flashes. `wsl.exe` prints UTF-16LE unless `WSL_UTF8=1` is in its
environment, so set that and decode defensively. The backend names the
program it runs rather than hard-coding `wsl.exe` on `PATH`, so a suite can
point it at a stub. The install suite does the same with package managers on
a stated `PATH`.

Telling *virtualization off* from *needs a restart* is the delicate part.
Where `wsl --status` alone does not tell them apart, use the optional
features' state (`VirtualMachinePlatform` pending) and the processor's
`VirtualizationFirmwareEnabled`. But a running hypervisor reports that last
one as false, so a machine with Hyper-V already on must not read as
virtualization off. Record what each case really prints in the tests'
fixtures.

Nothing calls the backend yet beyond its tests. Tasks 02, 04 and 05 are its
callers.

## Acceptance criteria

- [ ] The trait and the WSL implementation exist. Nothing outside the
      implementation names `wsl.exe`.
- [ ] Each of the probe's answers is read off a stub `wsl` the backend is
      pointed at: no WSL, needs a restart, virtualization off, no distro, and
      present-stopped and present-running under both NAT and mirrored. A
      Hyper-V-already-on machine reads as WSL answering, not as
      virtualization off.
- [ ] The two addresses are read off the stub under NAT (`eth0` address at
      8425, default-route gateway at the Keeper's peer port) and under
      mirrored (loopback at both ports).
- [ ] make, boot, terminate, run inside (as root and as the service user)
      and read a file each send the stub the command line they should, and
      UTF-16 and UTF-8 output both decode.
