# 02. Features

## What to build

The first section filled, and with it the two things the other two sections
reuse: the **carousel** every section is, and a **stylesheet of workbench
primitives** every screenshot is drawn with. After this task the Features
section shows three cards, each with a recreated screenshot and a paragraph,
and tasks 03 and 04 only add cards.

### The carousel

Each section is a heading and a carousel. On a desktop the cards run down one
side, each a clickable card carrying a **title** and a **subtitle**; beside
them, filling the rest of the width, is the pressed card's screenshot with its
paragraph underneath. Clicking a card switches the screenshot and paragraph.
The first card is pressed on load. The cards draw as the workbench's own card
buttons: `padding: 1rem; border-radius: 0.6rem; border: 1px solid transparent;
background: transparent`, filled `var(--card)` when pressed or hovered, the
title at 600 and the subtitle in `var(--ink-soft)`; `aria-pressed` carries the
state. Nothing advances on its own, ever. The switch between screenshots may
cross-fade briefly where motion is welcome and must cut cleanly where it is
not.

Below 60rem the carousel becomes an **accordion**: each card is a full-width
row, and the open one expands in place to show its screenshot and paragraph
under its title and subtitle, the others collapsed. One open at a time; the
first open on load. The same markup serves both shapes, the difference is CSS
and which element the script shows, so a resize across the breakpoint does not
lose the pressed card.

Keyboard: the cards are buttons, tab reaches them, Enter and Space press them.
The screenshot region is labelled by the pressed card's title. With the script
off, every card's screenshot and paragraph are visible in order, so the page
still reads.

### The screenshot primitives

One stylesheet (or one section of the site's stylesheet) of small classes
that draw workbench parts in the site's tokens, so every screenshot in every
task is made of the same pieces and reads as the same product. Each screenshot
sits in a **frame**: a `var(--card)` box with an edge border, 0.6rem radius, a
light `--lift`-style shadow, clipped, with the workbench's paper inside it.
Inside, the pieces are simplified but true: fewer elements than the real pane,
type a little larger than the real 0.85rem so it is legible at card size on a
phone, and the real shapes and colours. The screenshots are static HTML with
no script of their own and no images other than the hammer and the harness
marks (the four SVGs under `web/src/marks/`, copied into `site/`).

The pieces this task needs, drawn once and reused:

- **Pane head**: a bold title line with a soft 0.9rem repo or subtitle beside
  it, and a right-hand slot for a control. In the workbench a pane's head sits
  on paper with 1rem of vertical padding.
- **Wordmark**: the 2rem hammer and **Verkstead** at 1.4rem.
- **Sidebar row**: a card-button row, `padding: 0.85rem 1rem`, a 600 title
  that is the branch name, a soft 0.85rem line under it with the repo name,
  and at the right edge a **liveness mark** 0.75rem across: a solid accent dot
  (waiting on you), a 1.5px edge ring with its top segment soft (working), or
  a 1.5px accent ring (idle). Done rows at 45% opacity. The **New
  conversation** control above the list is a full-width card-filled, edge
  bordered, accent-text, 600, centred link.
- **Status button**: a pane-wide card box, `padding: 0.6rem 0.85rem`, a bold
  word (*Running*, *Waiting on you*, *Waiting on checks*, *Done*) with the
  lifecycle state in 0.9rem soft beside it and a chevron at the right; *Waiting
  on you* turns both accent.
- **Timeline card**: transparent at rest, 1rem padding, a head row with a
  1.1rem 600 kind (*Brief*, *Agent run*, *Question set*, *Commit*, *Pull
  request*, *Task list*, *Notice*) and soft details beside it.
- **Commit header**: the subject at 600, then a soft 0.85rem line with a repo
  chip (0.8rem, edge border, 0.4rem radius), a seven-character monospace SHA,
  *N files*, **+N** in `--added` and **−N** in `--removed` at 600.
- **Boxed message**: a card box with 1rem padding holding prose and, where a
  diagram is wanted, a small hand-drawn SVG of three or four labelled nodes
  joined by arrows, coloured like the diff (a `new` node outlined in
  `--added`, a `modified` one in `--accent`). Mermaid is not loaded; the
  diagram is drawn by hand so it is exact and weightless.
- **Diff file**: a card box with a summary row (monospace 600 path, stats at
  the right), a hunk header on `--hunk` in soft monospace, and lines in
  monospace with a soft line number, a `+` or `−` marker, and the added and
  removed washes.
- **Terminal**: a black box, 0.6rem radius, edge border, 0.5rem padding,
  light monospace text, a block cursor.
- **Primary button**: paper on accent, 600; **secondary**: card on edge; **quiet
  button**: accent text, edge border, 0.4rem radius.
- **Phone notification**: a small rounded card with the hammer at 1.5rem, a
  600 title and a soft one-line body, drawn as a system notification would
  be, for the *Work at your own pace* screenshot.

Write them as classes with a short prefix so they never collide with the
page's own styles, and keep them to what the screenshots use; a primitive
nobody draws with is weight.

### The three cards

The copy, exactly:

**Don't read the code**
*Review the decisions, not every line.*

> Experienced developers know AI output needs a human eye, and that reading
> every line of a fast-moving codebase is impossible. Verkstead presents each
> change as a written summary and a diagram above the diff, so you can judge
> the decisions that matter at a glance. Then a fresh-eyed review session reads
> the whole pull request and puts its findings to you as questions, each with a
> fix you approve before it is made.

**Work at your own pace**
*Run as many conversations as you like, and answer when it suits you.*

> Waiting on an agent drains your momentum, and juggling terminal tabs only
> splits your attention. Verkstead runs each piece of work as its own
> conversation and asks for you only when a decision needs you, with a push to
> wherever you are. When a run stops you get a notice, and can step in to
> resolve it whenever you have time. Everything else keeps going.

**Sandboxed by default**
*Every session sees only the files it needs.*

> Every agent session runs inside a sandbox with its worktree, its git
> directory, and nothing else of your machine. Bubblewrap on Linux, Apple's
> sandbox on macOS, a dedicated local account on Windows. Where no boundary is
> available, Verkstead refuses to run the session rather than running it bare.
> Let your agent run unattended, knowing it can never reach the wrong files.

### The three screenshots

All on the one example Conversation: repo `tidewatch`, branch
`rate-limiting`, agent Claude Code Opus 5.

1. **Don't read the code** — the commit details pane. Pane head *Commit*; the
   subject **Refuse requests over the limit with 429 and Retry-After**; the
   soft line with the `tidewatch` chip, SHA `b7e41c2`, *3 files*, **+58**
   **−4**. Then the *Message* section heading (1rem, 600, soft) over a boxed
   message: two short sentences of summary ("Requests past a key's limit are
   refused with 429 rather than queued. The limiter sets Retry-After from the
   window's remaining seconds, so a well-behaved client backs off without
   guessing.") and the small diagram: `middleware` → `limiter` (modified) →
   `429 response` (new). Then the *Diff* heading and one diff file,
   `src/limit.rs`, with a hunk header and five or six lines, two of them
   added. Cut the diff off at the frame's bottom edge so it is clearly longer.
2. **Work at your own pace** — the sidebar. Wordmark, *New conversation*, then
   four rows: `rate-limiting` / tidewatch with the working ring; `retry-backoff`
   / tidewatch with the solid accent dot; `metrics-endpoint` / tidewatch with
   the idle ring; `onboarding-copy` / docs-site at 45% opacity. Overlapping the
   frame's lower right corner, a phone notification: **Question set** / *Rate
   limiting for the public API, 3 questions*.
3. **Sandboxed by default** — the code pane's terminal. A tab bar on
   `--code-wash` with a file tab `limit.rs` and a pressed tab *Terminal*, then
   the black terminal showing

   ```
   $ pwd
   /work/tidewatch
   $ ls ~/Documents
   ls: cannot access '/home/you/Documents': No such file or directory
   $ cargo test
   running 14 tests ... ok
   $ █
   ```

   The point is in the second exchange; keep everything else quiet.

### Checking it

As task 01: headless Firefox at 1280 and 390, light and dark, screenshots
looked at before committing; the accordion open and closed at 390; a check
that pressing each card swaps the screenshot and the paragraph and that the
pressed card survives a resize across 60rem; no horizontal scroll at 360px.

## Acceptance criteria

- [ ] The Features section shows three cards with the titles, subtitles and paragraphs above word for word; the pressed card's screenshot and paragraph show beside the cards on a desktop and under the open row on a phone; nothing advances on its own; keyboard and script-off reading work.
- [ ] A prefixed set of screenshot primitives exists in the site's stylesheet, the three screenshots are built from them and from nothing else, and they are static HTML with only the hammer and the harness marks as images.
- [ ] The three screenshots match the descriptions above: commit details with summary, diagram and a diff; the sidebar with four rows and three distinct liveness marks plus the notification; the terminal with the refused `ls`.
- [ ] Light and dark both read well, there is no horizontal scroll at 360px, and the screenshots are legible at phone width.
