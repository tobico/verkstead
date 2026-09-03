# 05. The Windows msi

## What to build

The Windows desktop download becomes an msi installer — the human's own call
on the breakdown Set, replacing the portable exe now that the shim and the CLI
are two files. It carries the shim and the unified CLI exe: a Start-menu entry
launches the shim, the install is per-user by default (the app is unsigned and
elevation buys it nothing), and the install directory goes on the user's PATH
so `verkstead ask` works in a terminal. Built on the Windows release leg — the
WiX toolset is on the GitHub runners. The standalone CLI exe release asset
ships as it does today.

## Acceptance criteria

- [ ] The msi installs per-user, and its Start-menu entry opens the app with
      no console window.
- [ ] `verkstead guide` answers in a fresh terminal after the install.
- [ ] The release workflow uploads the msi where it uploaded the portable exe.
