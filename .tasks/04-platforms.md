# 04. Platforms

## What to build

The last section filled, and the page read whole. Four cards about where
Verkstead runs and what it runs, their screenshots, and a final sweep of the
entire page at every width in both schemes now that nothing is empty. The
carousel and the primitives exist; this task adds to them the way task 03 did.

### The four cards

The copy, exactly:

**Your desktop**
*Linux, macOS, and Windows.*

> Verkstead Desktop runs with full functionality on all three. The workbench
> includes a code editor and a terminal that run inside the session's sandbox,
> so you can try the agent's work and finish it by hand without leaving the
> window.

**Your phone**
*Pop in and out from anywhere.*

> The workbench is fully responsive, and Verkstead's client-server design
> means the work carries on at home while you answer from the train. Switch on
> remote access over Tailscale, scan the code, and the next question finds you
> wherever you are.

**All your devices**
*Linked machines, one workbench.*

> Install Verkstead on more than one machine and they form a cluster. Open any
> one of them and drive them all. Move a conversation to the Mac to build the
> Mac side, branch and working changes included, and bring it back when it is
> done. Cross-platform development, from one place.

**Your choice of agent**
*Open source, and open to any harness.*

> Verkstead is MIT-licensed and works with Claude Code, Codex, Grok Build, and
> the open-source OpenCode, so you get the latest frontier models without
> being tied to one provider. Yes, it works with your Claude subscription. Your
> process lives in Verkstead, not in any one vendor's tool.

### The four screenshots

The example Conversation again where it shows; the cluster's three machines
are **forge** (Linux), **anvil** (macOS) and **lathe** (Windows).

1. **Your desktop** — the code pane. Pane head *Code* with an expand icon at
   the right. A 16rem file tree on `--code-wash`: `tidewatch` at 600, then
   `src/` open with `main.rs`, `limit.rs` (an accent **M** at the right),
   `keys.rs`, and `tests/` closed. Beside it a tab bar with `limit.rs`
   pressed and *Terminal* beside it, over an editor box on paper showing eight
   or so lines of plausible Rust from the limiter (a `struct Window`, a
   `fn allow(&mut self, key: &str) -> Decision`), with the same soft line
   numbers the diff uses and the viewer's token colours (`--tok-keyword`
   `#9b3f5e`, `--tok-name` `#2f6f8f`, `--tok-string` `#4a7a4f`, `--tok-comment`
   `#8d857a`, and their dark values `#e08aa6`, `#86bcd8`, `#9dc493`,
   `#8b8378`) applied by hand.
2. **Your phone** — a phone. This one screenshot is drawn in a phone-shaped
   frame, about 20rem tall and narrow, rounded, centred in the carousel's
   screenshot area: the Timeline at phone width. A back link *← Conversations*
   in accent over the pane head `rate-limiting` · *tidewatch*; the status
   button **Waiting on you** · *Grilling* in accent; a *Question set* card
   titled *Rate limiting for the public API* with the solid accent dot and its
   three `Q` rows; a *Brief* card clamped to three lines. A system
   notification from task 02's primitive may sit over the top of the phone
   frame, reading **Question set** / *Rate limiting for the public API,
   3 questions*.
3. **All your devices** — Settings → Remote access, the devices part. A
   heading *Devices*, then three rows, each an OS icon (draw Linux, Apple and
   Windows as simple inline SVG glyphs or text marks; no icon font is loaded),
   the name, and the addresses in soft monospace: `forge` *this device* ·
   `100.64.0.3, 192.168.1.20`; `anvil` · `100.64.0.5`, with a quiet *Unlink*
   button at the right; `lathe` · `100.64.0.9`, likewise. Under them, the
   status menu's row as it appears in a Conversation, to make the point: a
   card with the 600 title **Transfer to…** and the soft sentence *Move this
   conversation, its branch and its working changes to another device.*, and
   beneath it two choices, `anvil` with the Apple glyph and `lathe` with the
   Windows one.
4. **Your choice of agent** — the Agent panel from the composer. A popover
   card with three pickers stacked, each a small soft label over a row with a
   harness mark and words: **Grilling** · Claude mark · *Claude Code Opus 5*;
   **Implementation** · Codex mark · *GPT-5 Codex*; **Review** · OpenCode mark
   · *OpenCode Minimax M2.1*. Under the three, a soft 0.85rem line *No review*
   as the unpressed alternative for the last. The four harness marks are the
   SVGs copied into `site/` in task 02 (claude-color, codex, grok, opencode);
   use three here and let the Grok mark appear in the picker's open state if
   the composition wants a fourth row, as *Grok Build Grok 4.6*.

### The sweep

With every section full, read the whole page at 1280, at 768 and at 390, light
and dark, top to bottom: the hero, the bar, the three carousels with each
card pressed, the strip, the table, the footer. Check the nav's tracking now
that the sections have their real heights, that no screenshot overflows its
frame at any width, that the phone frame in *Your phone* is not taller than
the viewport at 390, and that nothing scrolls sideways at 360px. Fix what the
sweep finds, in whichever task's part it lies.

## Acceptance criteria

- [ ] The Platforms section shows the four cards with the titles, subtitles and paragraphs above word for word, in that order, in the shared carousel.
- [ ] The four screenshots match the descriptions above, are built from the shared primitives, and the *Your phone* one is drawn in a phone-shaped frame that fits the viewport at 390.
- [ ] The whole page has been swept at 1280, 768 and 390 in light and dark with every card pressed, the nav tracks every section at its real height, and nothing scrolls sideways at 360px.
- [ ] Only the hammer, the harness marks and hand-drawn inline SVG are used as images anywhere on the page; no icon font, web font, script library or stylesheet is loaded from outside `site/`.
