# 05. Boot, link and terminate with the Keeper

## What to build

An Annex runs only while its Keeper runs (ADR-0024).

**At start.** Once the listeners are bound, the Keeper's startup spawns a
background task and never waits on it. Startup has no awaited background
work; it has fire-and-forget `tokio::spawn`s before the serve `select!`,
which never returns. Where the probe finds the distro present, the task:

1. **Boots it, and holds it.** WSL ends a distro shortly after its last
   `wsl.exe` client exits, so the boot is a long-lived `wsl.exe -d Verkstead`
   child doing nothing (decided). The child sits in a **Job Object with
   kill-on-close**, so when the Keeper's process dies for any reason, the
   child dies with it and WSL's idle timeout stops the distro.
2. **Waits for the Annex to answer**, the service on 8426 inside.
3. **Reads the two addresses** through task 01: the Annex's (NAT `eth0`, or
   the loopback, at 8425) and `keeper_at`. Both are read afresh at every
   boot, so a NAT gateway that moved across a reboot is right again.
4. **Runs the linking operation**, `Devices::keep(data_dir, address,
   keeper_at)`. This is stage 01's operation, it is idempotent, and its
   comment already expects this caller. It rewrites `kept_by` with `at`,
   dials the pinned Keeper's Join, points the forward, writes the direct
   address and announces. It takes the Annex's data directory as a path,
   which is reached through `\\wsl.localhost\Verkstead\var\lib\verkstead`.
   Prove that `device.pem` reads and the `kept_by` rewrite land readable by
   the service user. If they do not, give it a seam over task 01's
   read-a-file and run-inside.

**At stop.** On Windows, the desktop app stops its sidecar with
`TerminateProcess`, so no stop path in the server runs. The Job Object is
what ends the distro then, with no change to the app. Where the server does
hear a stop (Ctrl-C to a console `verkstead serve`, which reaches
`stopped()` in `lib.rs`), it also runs task 01's *terminate* before the
advertisement is withdrawn.

**A failure** (decided). An Annex that will not boot, will not answer
within the boot's time, or will not link puts a line on its row on the
Devices list saying why, and in the log. Notices belong to a Conversation and
there is no server-wide one, and the Components row is stage 04's. Hold the
reason where stage 04's row can read it later. The Keeper carries on either
way, and the Annex's rows read *unreachable* until it answers.

## Acceptance criteria

- [ ] After a restart of the Keeper, with nothing pressed, Devices lists the
      Annex reachable within the boot's time, on this machine and under both
      NAT and `networkingMode=mirrored` (`.wslconfig`, then
      `wsl --shutdown`).
- [ ] The Keeper's member row for the Annex has the direct address the probe
      read. The Annex's `kept_by.at` is the `keeper_at` it read. The Annex
      stays reachable from the Keeper with the Keeper's network offline.
- [ ] Quitting the desktop app leaves `wsl -l --running` without `Verkstead`
      within WSL's idle timeout. Ctrl-C on a console Keeper terminates it at
      once.
- [ ] A distro whose service will not start shows a line on its Devices row
      with the reason, and the Keeper's workbench is unaffected.
