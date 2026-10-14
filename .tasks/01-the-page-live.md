# 01. The page, live

## What to build

The site, deployed, with everything but the three sections' contents: the
hero, the sticky nav, the download bar, three section headings with nothing
under them yet, the downloads table and the footer. After this task the page is
served at the Pages root, the share viewer is still served beside it at the
address Verkstead links to, and the README no longer says nothing is released.

### Where it lives and how it ships

`site/` at the repository root holds one `index.html`, one stylesheet and one
script, plus its own copy of the app icon (the 512px PNG under `assets/icons`,
and the 32px one as the favicon). No build, no package manager, no framework,
no web fonts: the page is what is in the directory.

The existing Pages workflow (`.github/workflows/pages.yml`) assembles `_site/`
from a single copied file, the share viewer. Extend it rather than adding a
second workflow: copy `site/` into `_site/` and the viewer beside it, trigger on
changes under `site/` as well as to the viewer and the workflow, and keep the
one `pages` concurrency group. The post-deploy check that reads the viewer back
over the public URL stays exactly as it is, and a second check of the same
shape reads the root page back and looks for its `<title>`, which is
`Verkstead` and nothing else on the site carries. Nothing in the workflow's
permissions changes.

### The look

Copy the viewer's tokens into the site's stylesheet, light and dark, and let
`color-scheme: light dark` plus `prefers-color-scheme` pick between them, with
no toggle, the way the viewer does:

| Token | Light | Dark |
|---|---|---|
| `--ink` | `#1c1a17` | `#ece7e0` |
| `--ink-soft` | `#6b645c` | `#a19a91` |
| `--paper` | `#faf8f5` | `#171614` |
| `--card` | `#ffffff` | `#201e1b` |
| `--edge` | `#e3ddd4` | `#35312c` |
| `--accent` | `#a34b12` | `#f08a3c` |
| `--code-wash` | `#efe8dd` | `#2e2b26` |
| `--added` / wash | `#2a7248` / `#e9f5ec` | `#79c48f` / `#1b2a20` |
| `--removed` / wash | `#b3382c` / `#fbeceb` | `#e0857a` / `#2d1f1d` |
| `--hunk` | `#f2ede6` | `#26241f` |

`--accent-deep` is `color-mix(in srgb, var(--accent) 88%, var(--ink))` and is
every filled button's hover, and `--lift` is `0 0.75rem 2rem rgb(0 0 0 / 22%), 0 0.15rem 0.4rem rgb(0 0 0 / 14%)` (55% and 40% in dark), the raised shadow. Body is `16px/1.5 system-ui, -apple-system,
"Segoe UI", sans-serif`; monospace is `ui-monospace, SFMono-Regular, monospace`.
Borders are `1px solid var(--edge)`; controls round at 0.4rem, cards and
containers at 0.6rem; bold is 600. Links are accent. Focus is
`outline: 2px solid var(--accent); outline-offset: 2px`. Transitions exist only
inside `@media (prefers-reduced-motion: no-preference)`. The page is calm:
generous vertical space between sections, a content measure around 60rem,
nothing animated that is not responding to the reader, and a 16px side gutter
on a phone with no horizontal scroll at 360px.

The hero's type may be larger than anything in the workbench, since this is a
page and not a tool, but the voice of the layout is the workbench's: paper
background, ink text, accent used sparingly and only where it means something.

### Top to bottom

**The nav** sticks to the top of the viewport. On the left, three tabs reading
**Features · Process · Platforms**, drawn exactly as the workbench's
Transcript/Screen switch: a flex row with `gap: 0.25rem; padding: 0.2rem;
background: var(--card); border: 1px solid var(--edge); border-radius: 0.6rem`;
each tab `padding: 0.35rem 0.7rem; font-size: 0.85rem; color: var(--ink-soft);
background: none; border: 0; border-radius: 0.4rem`; and an absolutely
positioned indicator (`top: 0.2rem; bottom: 0.2rem; background: var(--accent);
border-radius: 0.4rem`) that is moved with `translateX` to the pressed tab's
`offsetLeft` and sized to its `offsetWidth`, transitioning `transform` and
`width` over 0.1s ease-in-out where motion is welcome. The pressed tab's text
is `var(--paper)`. The pressed tab is whichever section is in view, tracked
with an IntersectionObserver (or a scroll listener) as the reader scrolls, and
a click scrolls smoothly to that section (respecting reduced motion) and
presses it. The tabs are real links to the section ids with `aria-current`,
so the page works with the script off. The indicator is re-measured on resize.

On the right of the nav, a smaller copy of the two download controls: the
primary as a filled accent button (paper text, 600, 0.4rem radius) and the
secondary as a quiet link. Below 40rem the tabs stay, the primary's label
shrinks to **Download**, and the secondary link is hidden.

**The hero** is the hammer, the name and the tagline, nothing else: the icon at
a generous size, **Verkstead** beside or above it in the body face, and under
them the one line

> Build **intentional** software with AI agents, **on your terms**

with the two bold phrases in `var(--accent)`.

**The download bar** follows the hero: a primary button and a secondary link.
Primary reads *Download for macOS*, *Download for Linux* or *Download for
Windows* and links straight at the asset; secondary reads *All downloads* and
jumps to the table. The platform is read in the browser, `navigator.userAgentData.platform`
first and the user agent string as the fallback: *macOS* → `Verkstead-universal.dmg`,
*Windows* → `Verkstead-x86_64.msi`, *Linux* → `Verkstead-x86_64.AppImage`.
Anything else, and any Linux whose user agent says `aarch64` or `arm`, and any
touch device (Android, iPhone, and an iPad that reports itself as a Mac with
touch points), reads plain **Download** and jumps to the table instead. Before
the script runs, and with it off, the button already reads **Download** and
links to the table, so nothing depends on JS being there. No size and no
version beside the label. The download addresses are the stable
`https://github.com/tobico/verkstead/releases/latest/download/<asset>` form,
which GitHub redirects to the current release, so the page never has to know a
version number.

**Three sections** follow, with ids the nav points at and headings reading
**Features**, **Process** and **Platforms**, each empty under its heading for
now. Give each enough minimum height that the nav's tracking can be seen to
work before the sections are filled.

**The downloads table** has four rows, each the platform, the file, and a short
note, with the file linked at its latest-release address:

| Platform | File | Note |
|---|---|---|
| Linux | `Verkstead-x86_64.AppImage` | needs bubblewrap and FUSE on the host |
| macOS | `Verkstead-universal.dmg` | unsigned: allow it under Privacy & Security, Open Anyway |
| Windows | `Verkstead-x86_64.msi` | unsigned: SmartScreen, More info, then Run anyway |
| Command line and NixOS | | the releases page, and the adoption guide |

The last row links to `https://github.com/tobico/verkstead/releases` and to
`docs/adoption.md` on GitHub at its *Getting it running* heading; the macOS and
Windows notes link to the adoption guide's own sections for each desktop app.

**The footer** is one calm line of links and one sentence: *Verkstead is early,
at version 0.1 and changing fast.* · GitHub (the repository) · MIT (the LICENSE
file) · Adoption guide. No star count, no badges.

### The README and the adoption guide

The README's **Status** section still opens *Early, private, and unreleased.
There is nothing to download yet.* Replace that paragraph with the truth: it is
released, the site is where the downloads are, and the rest of the section's
description of the four ways in stays. Its *Releasing* line near the end says
nothing has been released under this name yet; drop that clause. The adoption
guide's *Getting it running* section opens with the same claim ("Nothing has
been released under this name yet, so what follows is what a release
produces…"); rewrite that opening so it describes what the releases are rather
than what a release would be. Touch nothing else in either document.

### Checking it

There is no test suite for a static page; the check is a browser. Serve `site/`
locally (`python3 -m http.server` in the directory is enough) and read it in
headless Firefox, which is on the machine, at a desktop width (1280) and a
phone width (390), in light and in dark (`ui.systemUsesDarkTheme` in a profile's
`user.js`, or `prefers-color-scheme` emulated), with `--screenshot` for a look
and marionette's `ExecuteScript` where a number is wanted: that
`document.documentElement.scrollWidth` does not exceed the viewport at 360px,
that the pressed tab changes as `scrollTo` moves through the sections, that the
primary button's text and href are right for each platform string. Look at the
screenshots before committing; the page has to be calm to the eye, not only
correct.

## Acceptance criteria

- [ ] `site/` holds the page, its stylesheet, its script and its icons, with no build and no dependency, and the Pages workflow deploys it beside the share viewer, triggers on changes to it, and reads both the root page and the viewer back after deploying.
- [ ] The nav sticks, draws as the Transcript/Screen switch, presses the section in view as the reader scrolls, jumps on a click, works as plain links without the script, and on a phone keeps its tabs, shrinks the CTA to *Download* and hides the secondary link.
- [ ] The hero is the hammer, the name and the tagline with *intentional* and *on your terms* in accent; the download bar's primary reads *Download for* the detected platform and links the right asset, falls back to *Download* and the table on an unknown or unsupported one, and already does so before any script runs.
- [ ] The downloads table and the footer carry the rows, notes, links and sentence set out above, and nothing more.
- [ ] Light and dark follow the system; there is no horizontal scroll at 360px; motion is gated on `prefers-reduced-motion`.
- [ ] The README and the adoption guide no longer say nothing is released, and say nothing else new.
