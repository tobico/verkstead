# 03. Process

## What to build

The second section filled: five cards walking the example Conversation from
Brief to Done, and under the carousel a one-line strip naming the other four
Processes. The carousel and the screenshot primitives already exist from task
02; this task adds cards and screenshots to them. Where a screenshot needs a
piece the primitives do not yet have, add it to the shared stylesheet beside
the others, never inline in one screenshot.

### The five cards

The copy, exactly:

**Draft**
*Start with your idea, in plain words.*

> Verkstead starts with a clean text box. Write what you want in your own
> words, as rough as you like. There is no prompt engineering to do. The
> development process is built in through Verkstead's agent skills, so you are
> free to focus entirely on your problem.

**Grill**
*Settle every detail before any code is written.*

> Grilling is the single biggest change that separates intentional development
> from vibe coding. Your agent reads the code you have, then asks you about
> every technical detail that matters, with a recommended answer for each, and
> goes on only once everything is settled. Related questions arrive together
> as one question set you consider and answer as a whole, from your phone if
> you like.

**Build**
*Sized to the work, and run without you.*

> Your settled design becomes a handoff, a task list, or a staged roadmap,
> depending on its size, and each step runs in a fresh session with just the
> context it needs. Every commit arrives with a written summary and a diagram
> of what changed, so you can follow along without scanning every line.

**Wrap**
*A pull request, green checks, and a review you direct.*

> A job is rarely done just because every task is ticked. Verkstead opens the
> pull request, waits on CI, fixes what fails, resolves merge conflicts, and
> runs a fresh session to review the whole thing. Its findings come to you as
> one question set, so you choose what gets fixed and how.

**Done**
*Share the thinking, or carry on.*

> The record of every question and decision is yours to share with colleagues,
> as a single file or a link. And when there is more to do, steer the
> conversation into a follow-up and extend the same pull request.

### The strip

Under the carousel, inside the Process section, a quiet row headed **Not
everything is a feature.** and then four entries, each the Process's name at
600 with the picker's own description in soft text under it, side by side on a
desktop and two by two or stacked on a phone:

| Name | Description |
|---|---|
| Tinker | Make a series of simple one-off changes with minimal fuss |
| Investigate | Ask questions about a code base without committing any code |
| Review | Review an existing PR or branch, and fix any issues found |
| Fix merge issues | Fix up a PR which is unable to merge |

### The five screenshots

The example Conversation throughout: repo `tidewatch`, branch
`rate-limiting`, Claude Code Opus 5, pull request #128 *Rate limiting for the
public API*.

1. **Draft** — the compose page. Pane head *New conversation*. The composer
   box (card fill, edge border, 0.9rem radius, `padding: 1rem 0.75rem 0.85rem`)
   with the Brief typed into it: *Add rate limiting to the public API. Last
   night one client sent 40k requests in a minute and took the service down.
   Limit per API key, with a sensible default that can be changed per key.*
   Along the box's bottom edge the option row, each a two-line control with a
   small soft label over its value and a chevron: **Repo** `tidewatch`,
   **Process** `Develop`, **Agent** the Claude mark and `Opus 5`. Under the
   box, at the left a `+` and at the right *Save as draft* (secondary) and
   **Start work** (primary).
2. **Grill** — a question set sheet. Pane head with the set's title *Rate
   limiting for the public API*. The *Preface* heading and a boxed card with
   two lines of preface ("`POST /v1/keys` has no limit today. A limiter can
   land this week; where the counter lives is the one call that is yours.").
   The *Questions* heading and one question card with **Q1** in accent
   monospace hanging at its left and the text *Where should the counter
   live?*, then two option rows, each a radio, a soft number, the text and, on
   the first, an accent ★ at the right: `1 In-process counter ★` and
   `2 Shared Redis counter`; the first pressed, with the accent border. Under
   them a free-text box placeholdered *Q1 — Your thoughts*. A second question
   card, **Q2** *Default limit per key?*, cut off by the frame's bottom edge.
3. **Build** — the Timeline mid-run. The status button reading **Running**
   with *Implementing* soft beside it. The pinned task list card: head *Task
   list* · *rate-limiting* · *2 of 5 done* at the right; five rows with ☑ or
   ☐, the first two struck through and soft (*Count requests per key in a
   sliding window*, *Refuse requests over the limit with 429 and
   Retry-After*), the third current (*Make the limit configurable per key*),
   then *An admin endpoint to read a key's usage* and *Document the limit*;
   numbers `01`…`05` at the right in monospace. Below, two commit cards,
   each the head *Commit* with the `tidewatch` chip and a SHA, the subject,
   and the files/+/− line (`a19f3d7` *Count requests per key in a sliding
   window* · 4 files +121 −2; `b7e41c2` *Refuse requests over the limit with
   429 and Retry-After* · 3 files +58 −4), then an *Agent run* card with
   *14 turns* and the working ring.
4. **Wrap** — the Timeline wrapping up. Status button **Waiting on checks**
   with *Wrapping* soft beside it. The pinned pull request card: head *Pull
   request* · `#128` · `tidewatch` chip, and at the right a green check, an
   accent hollow circle (running) and nothing else; then the title *Rate
   limiting for the public API*. Below it a *Question set* card titled
   *Review of rate-limiting* with the solid accent dot at the right and three
   0.85rem rows: `Q1` *Retry-After rounds down to 0 on the window's last
   second* / *—*, `Q2` *The admin endpoint is unauthenticated* / *—*, `Q3`
   *Limit config is read on every request* / *—*.
5. **Done** — the Timeline settled. Status button **Done** alone, bold. A
   centred soft line *Wrapping → Done*. The pull request card as above but
   with two green checks. Over the frame's lower part, a small share dialog:
   a card with a 600 title *Share this conversation*, a soft sentence (*The
   brief, every question and answer, and every commit, as one file or a secret
   link.*), and two buttons, *Download file* (secondary) and **Copy link**
   (primary).

### Checking it

As the earlier tasks: Firefox at 1280 and 390, light and dark, every card
pressed in turn, the strip at both widths, no horizontal scroll at 360px, and
the screenshots looked at before committing.

## Acceptance criteria

- [ ] The Process section shows the five cards with the titles, subtitles and paragraphs above word for word, in that order, in the shared carousel.
- [ ] The five screenshots match the descriptions above and are built from the shared primitives, with anything new added to the shared stylesheet rather than inlined.
- [ ] The strip under the carousel carries the heading and the four Processes with the picker's descriptions, and lays out at both widths.
- [ ] Light and dark both read well, nothing scrolls sideways at 360px, and the screenshots are legible at phone width.
