# 07. The words

## What to build

`docs/development.md`'s account of running Verkstead on the desktop, which from
this stage is `pnpm start` in `desktop/` rather than `cargo run -p verkstead-cli
-- desktop --data-dir .`. A fifth of that document is about the crate this
roadmap retires, and the part that goes wrong first is the part a developer
follows on their first afternoon.

**What replaces it.** The desktop app is the Electron project beside `web/`,
started with `pnpm start` from the dev shell; it brings the headless `verkstead`
up beside itself as `serve --desktop`, waits for its health and opens a window on
the workbench already logged in. The viewer has to have been built and the CLI
compiled first, which is the same two commands the quickstart already gives. A
checkout run points the sidecar at the checkout the way every other command in
that document does — the Data Directory is said through the environment here
rather than through a flag of the app's, because the app has none. An address
something is already listening on is still a dialog and a non-zero exit.

**What it puts on the screen is a window**, and that is the whole of it at this
stage: the tray, the close policy and the Desktop page arrive in stage 03, and
the section should not promise them early. The **Launch on Startup** account
belongs to the tray app for as long as the tray app is what ships, so what this
task touches is the running-it section rather than every paragraph with the word
desktop in it.

**And the Rust tray app is still what ships.** The trunk stays releasable
throughout this roadmap: the tray app keeps shipping on each platform until the
stage for that platform takes its release leg, and stage 08 retires it once none
is left. So the packaging, release and toolkit sections are left exactly as they
are — they are stages 05 to 08's to rewrite — and what remains of the tray app in
the running-it section is a sentence saying it is what a release still carries
and where its own account now lives.

**And CONTEXT.md is read back rather than assumed.** Its **Log Directory** and
**Workbench Key** entries were written for a desktop app that is now this one;
amend only what has actually stopped being true, and leave the vocabulary alone
where it still holds.

## Acceptance criteria

- [ ] Nothing in the running-it section tells a developer to start the tray app;
      `pnpm start` in `desktop/` is what it says, with what a checkout run does
      about its Data Directory.
- [ ] What the app puts on the screen reads as one window, with the tray, the
      close policy and the Desktop page left to stage 03.
- [ ] The packaging, release and toolkit sections are untouched, and the Rust
      tray app is still named as what a release carries until stages 05 to 08.
