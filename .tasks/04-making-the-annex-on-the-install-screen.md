# 04. Making the Annex on the install screen

## What to build

When the WSL row is ticked and WSL answers (task 02's "ticked by itself"),
Next makes the Annex as part of the wizard's install run, with the install
screen's progress bar and status line. The run's units, in order:

1. **Download** the NixOS-WSL image at the pinned release (`nixos.wsl` from
   the GitHub release that task 03's flake pins, about 550 MB) into
   `%LOCALAPPDATA%\Verkstead\wsl`. Check it against the release's
   `nixos.wsl.sha256`. The status line shows how far the download has got.
   Re-use a complete, verified file that is already there.
2. **Import** it with task 01's *make*, as `Verkstead`, with its disk in that
   directory.
3. **The firewall rule**, raised (decided): one inbound rule for the Keeper's
   peer port on the `vEthernet (WSL…)` adapter, which Windows Defender treats
   as a public network. The adapter exists only once the WSL VM has run,
   which is why this runs after the import and not in task 02's elevated
   step. On a fresh machine this is the second UAC dialog of the setup. Skip
   it when the rule is already there, and under mirrored networking.
4. **Write the files** from task 03 into the distro: the flake and
   configuration under `/etc/nixos`, `config.yaml` and `secrets.yaml` in the
   module's data directory `/var/lib/verkstead`, owned by the service user,
   with `secrets.yaml` mode 0600. Under `VERKSTEAD_ANNEX_BINARY`, the binary
   is copied in too. Write through *run inside* as root, not through
   `\\wsl$`, so the ownership is right.
5. **Rebuild** with `nixos-rebuild switch --flake /etc/nixos#<name>`. Its
   output's last line is the status line, and a failure keeps the tail of
   the output as the row's reason.

Steps 1, 2, 4 and 5 run **as the user**. The Windows arm has no as-the-user
units today (every row is `How::Raised`), so this adds them. The
progress-bar plumbing (marker files, `doing`, the 2-second poll) stays as it
is. The run is the in-memory `Installer`'s. A restart of the Keeper part way
through leaves the row absent and still ticked. Pressing Next again resumes
from whatever the probe finds (image present, distro present).

When the run is done, the row is present. The boot and link at every start
are task 05's. This task's run ends with the distro rebuilt, and the first
link comes from task 05 being in place.

## Acceptance criteria

- [ ] On a Windows machine with WSL and no distro, Next downloads, verifies,
      imports, adds the rule, writes and rebuilds, and the row reads present.
      This is proven on this machine for real, not stubbed, with
      `VERKSTEAD_ANNEX_BINARY` naming a musl build of this branch.
- [ ] The booted Annex reports the Keeper's hostname (`hostname` inside, and
      its identity's name), and its server answers on 127.0.0.1:8426 inside
      the distro.
- [ ] A hash mismatch, a failed import or a failed rebuild fails the row with
      a reason, and the hint screen shows it.
- [ ] The plan's units and their order, raised or as the user, are
      unit-tested over task 01's stub.
