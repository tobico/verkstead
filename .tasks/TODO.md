# The sidecar flag

`verkstead serve --desktop` runs the server exactly as `verkstead serve` does,
with the two differences a desktop app needs. Its one startup line names the
workbench address and no key, because the log under the app is a file on
somebody's desk that a menu item opens rather than a journal the machine lets
whoever it lets read. And a refused `tailscale serve` press raises the
platform's own password dialog instead of handing back a `sudo` line, because an
app has something a daemon has not: somebody at the machine to ask.

Rust only, with nothing of Electron in it. The graphical grant and the screen
probe move into the server crate as they are, the Rust tray app goes on working
through the moved ones, and `verkstead serve` without the flag is unchanged in
every respect — so the stage ships on its own and stage 02 has a sidecar to
start. The decisions are in
[ADR-0020](docs/adr/0020-electron-desktop.md) and
[ADR-0015](docs/adr/0015-open-boundary-and-workbench-key.md) as amended.

Roadmap stage: [01: The sidecar flag](docs/roadmaps/electron-desktop/01-sidecar-flag.md)

## Tasks

- [x] 01: The flag, and the line without the key — [details](01-the-flag-and-the-line.md)
- [x] 02: The grant and the screen probe, in the server crate — [details](02-the-grant-in-the-server-crate.md)
- [x] 03: The flag turns the grant on — [details](03-the-flag-turns-the-grant-on.md)
