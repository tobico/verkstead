# 02. The renderer marks a picture and the viewer draws it

## What to build

A file with a picture's extension — `.svg`, and the formats the Code pane
draws: png, jpg, jpeg, gif, webp, avif, bmp, ico — is drawn as a picture
wherever a Diff is read: the question set's attached Diff and the commit pane,
which share one renderer and one injected markup (ADR-0026, *Drawn as a
picture*).

**The renderer.** It reads each file's `index` line — unified, and combined
with one id per parent — and keeps the before and after ids and the file's
media type, decided by extension. For a picture it writes, where the hunks
would be, a picture section the viewer draws from: a placeholder for each side
that exists — before for an edit or a deletion, after for an edit or an
addition — carrying the block's repo name, the path, the blob id and the media
type, with its side's word on it, *Before* and *After*. A merge's before is the
first parent's id. An SVG's hunks are still written under the section, as they
are today, marked so the stylesheet hides them while a picture is drawn; a
raster has none. The summary row is as it is today: the path, the status pill
and the +/− tally for an SVG. A file renamed without a change to its bytes has
no `index` line and draws as it does today, and a file over the cap draws as
code with a line saying why. The markup names the bytes; it never carries them.

The decision-rich part, a placeholder's shape — the names are the renderer's,
and so live beside the other renderer names in the global diff stylesheet:

```html
<figure class="diffPicture" data-repo="verkstead" data-path="assets/mark.svg"
        data-blob="<full id>" data-type="image/svg+xml">
  <figcaption>Before</figcaption>
</figure>
```

**The viewer draws over it**, the way it draws a Diagram over the server's
source block: one routine, run over the injected markup by both the Set sheet
and the commit pane, fetches each placeholder's bytes from task 01's endpoint
as its fold scrolls near — never before, and never twice: fetched bytes are
cached by id for the life of the page — and puts an `img` in the figure over an
object URL. A pair sits side by side, each half taking half the width, and
stacks Before over After under the phone width the Diff's stylesheet already
breaks at. Each picture sits on a chequerboard of two greys from the theme's
own tokens, one pair for each theme, in a box no taller than about a third of
the viewport, the picture fitted inside it and never drawn larger than its own
size. Never inline markup: an SVG goes in as an `img`, so nothing in it runs
and nothing it references is fetched.

**When the fetch fails** — the blob gone with a worktree that was never
committed, a repo taken off the registry, the cap — an SVG's hidden hunks are
shown in the section's place, and a raster's figure becomes one line saying
the picture is no longer there. A picture that fetches but will not draw is
left to the browser's own broken-image mark.

**Fixtures.** The Diff in the server test that writes the viewer's fixtures
gains an edited SVG and an added PNG, with full ids, so the viewer's tests draw
the server's own markup; the picture bytes themselves come from a stub in the
viewer's tests.

The table of contents, the wrap switch, the per-file fold and its anchor, and
what copying a hunk yields are untouched.

## Acceptance criteria

- [ ] An added, edited and deleted SVG and an added and edited raster each draw as pictures in both the Set sheet and the commit pane — an edit as a labelled Before and After pair, side by side on a wide pane and stacked on a phone; a merge commit's Before is the first parent's.
- [ ] A picture is fetched only once its fold scrolls near and only once per id, and is drawn through an `img` on a chequerboard in a bounded box, never as inline markup.
- [ ] A picture that cannot be fetched shows the SVG's hunks as code, or a one-sentence line for a raster; a rename without a change, a file over the cap and every file that is not a picture draw exactly as today.
- [ ] The render crate's tests cover the `index` line in both shapes and the picture section; the viewer's suite covers the drawing, the pair, the lazy fetch and both fallbacks against regenerated fixtures.
