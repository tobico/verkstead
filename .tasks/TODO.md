# Dependency installer

The wizard's first step installs what it names. Today it draws the install
command for every missing dependency and waits for the human to paste it into
a terminal on the server machine. After this backlog, an absent row carries a
checkbox where its tick would be, the human ticks what they want and presses
Next, and Verkstead installs the lot: one elevated package-manager run through
the desktop app's own password dialog, then the vendor installers as the user.
A second screen shows a progress bar and a status line while that runs, and a
third — the old instructions, cut down to the rows still missing — appears only
where something could not be installed automatically, with Next held until
every ticked row is detected.

Settled by the grilling: the privilege comes through the existing `Elevate`
seam, so only the desktop app with a screen ever elevates, and a server with
no way to ask sends every ticked row to the hint screen. One dialog per Next,
running the package manager directly, with nodejs and npm added where an npm
row is ticked. Claude's native installer and xAI's grok installer run as the
user, and a directory Verkstead installs into is written to `session_path` in
`config.yaml` and composed ahead of the server's own PATH — the amendment to
ADR-0016 made in the plan commit. Homebrew is installed where missing, after an
elevated step makes its prefix. Every Windows install goes through the runas
arm, and the Windows sandbox row becomes the session account: probed, gating,
made by the elevated verb, and the pipe re-opened with the grant. Cancel waits
for the unit under way and skips the rest. Only the gating rows are ticked by
default, nothing says in advance which rows need a step by hand, the install
screen moves on by itself, and every button in the wizard reads Next.

## Tasks

- [x] 01: A directory Verkstead installs into stays on the session PATH — [details](01-session-path.md)
- [x] 02: The run: distro packages through the elevated seam — [details](02-elevated-run.md)
- [ ] 03: The three screens — [details](03-screens.md)
- [ ] 04: The vendor installers as the user — [details](04-vendor-installers.md)
- [ ] 05: macOS — [details](05-macos.md)
- [ ] 06: Windows — [details](06-windows.md)
