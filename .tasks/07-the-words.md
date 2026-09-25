# 07. The words

## What to build

`CONTEXT.md` gains the term this stage introduces and loses the two readings this
stage falsifies, and `docs/development.md` says what `pnpm start` now puts on the
screen. Settled with the human (Q4): this stage amends only what it has actually
made untrue, and the platform rewrites stay stage 05's, which claims the **Log
Directory**, **Startup Registration** and tray entries for itself.

**The new term is the app's own desktop settings and the page that draws them**: a
JSON file of Electron's own under its user data, reached over a preload bridge that
exposes the platform, the settings and the startup registration, drawn only where
that bridge exists — so a phone on the tailnet never sees the section, nothing is on
the wire and nothing is in `config.yaml`. That is the split the per-device push
switch already made: a fact about the machine in front of the human, never sent to
the server. Written in the file's own form, with its own *Avoid* line.

**What has stopped being true**, and nothing else in either entry:

- **Startup Registration** says **Launch on Startup** is ticked and unticked on the
  tray menu. It is on the Desktop page now, and the tray menu is three items that do
  not include it.
- **Log Directory** says **View Logs** on the tray menu is what opens the file. It is
  that item *or* the Desktop page's button, which is why a desktop that gives
  Verkstead no icon loses nothing but the icon — and why the button exists at all.

**`docs/development.md`** had its running-it section put right by stage 02, which
made it `pnpm start` in `desktop/`. What it gains is what that now puts on the
screen: the tray, the close choice and the Desktop page, and the app's own settings
file being what a dev run writes. **Launch on Startup** is greyed on an unpackaged
run, which is worth the sentence there rather than in a glossary.

**And the Rust tray app is still what ships.** The trunk stays releasable
throughout this roadmap — the tray app keeps shipping on each platform until the
stage for that platform takes its release leg, and stage 08 retires it once none is
left. So nothing here retires anything, and the packaging, release and toolkit
sections are left exactly as they are.

## Acceptance criteria

- [ ] `CONTEXT.md` carries the term for the app's own desktop settings and the page
      that draws them, in the file's own form, and says nothing of it is on the wire.
- [ ] The two readings this stage falsified are amended and nothing else in those
      entries is touched.
- [ ] `docs/development.md`'s running-it section names the tray, the close choice and
      the Desktop page, and the Rust tray app is still named as what a release
      carries.
