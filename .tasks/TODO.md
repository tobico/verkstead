# 03. AppImage and the release legs

A `v*` tag builds `Verkstead-x86_64.AppImage` alongside the four bare CLI
binaries and attaches it to the GitHub Release, and the file runs the stage-02
tray on a stock distribution. The desktop binary is dynamic and carries its
toolkit inside the bundle rather than being made static; the CLI legs stay musl
exactly as they are.

What this stage lays down once, stages 04 and 05 reuse: the packaging assets —
a desktop entry and the launcher icons, generated rather than hand-drawn, in a
directory of their own outside the viewer's `publicDir` — and the release
plumbing that carries a desktop artifact from a build leg to a Release asset. By
the time the macOS dmg and the Windows exe arrive, each is a build leg and an
artifact name and nothing more.

Roadmap stage: [03: AppImage and the release legs](docs/roadmaps/desktop/03-appimage.md)

## Tasks

- [x] 01: Packaging assets and the AppImage — [details](01-appimage-bundle.md)
- [ ] 02: A tag ships the AppImage — [details](02-release-leg.md)
- [ ] 03: The install story and what a tag now produces — [details](03-install-story.md)
