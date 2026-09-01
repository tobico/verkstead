# 04. macOS bundle and dmg

A `v*` tag builds a universal `Verkstead.app` inside a dmg, and on a Mac the
tray runs, the viewer opens and sessions work. The desktop crate is GTK
throughout today and does not compile for an Apple target at all, so the first
half of this stage is its macOS arm — the event loop, the dialogs, the startup
registration — and then the bundle, the dmg and the release leg that carries it.

The second half ports the Sandbox. ADR-0012 said sessions stay Linux-only and
are ported later, platform by platform; this stage brings the macOS port
forward, and amends that ADR in place to say so. Apple's sandbox denies rather
than hides, so it is a second surface with the same intent rather than the same
one moved: a session on a Mac can see the machine and is refused it, and every
path that exists only inside a bind on Linux becomes a real one there. The
boundary is proved the way the Linux one is — by a probe run inside a real
sandbox — which is why CI gains a macOS leg to run it.

Roadmap stage: [04: macOS bundle and dmg](docs/roadmaps/desktop/04-macos-dmg.md)

## Tasks

- [x] 01: The desktop crate boots on macOS — [details](01-desktop-crate-on-macos.md)
- [x] 02: Launch on Startup on macOS — [details](02-launch-on-startup.md)
- [x] 03: The app bundle and the dmg — [details](03-bundle-and-dmg.md)
- [x] 04: The release leg — [details](04-release-leg.md)
- [ ] 05: A session runs inside an Apple sandbox — [details](05-session-in-an-apple-sandbox.md)
- [ ] 06: HOME, the account and the skills — [details](06-home-account-and-skills.md)
- [ ] 07: Companions, configured binds and the build cache — [details](07-companions-binds-and-cache.md)
- [ ] 08: A session outlives nothing — [details](08-a-session-outlives-nothing.md)
- [ ] 09: The install story — [details](09-install-story.md)
