# Pictures in the diff, fetched by blob id

A file a browser can draw is drawn in the diff as a picture rather than read
as code — in the Diff attached to a question set and in the commit pane, which
share one server-side renderer — and its bytes are fetched by git's own name
for them rather than stored.

A diff of an SVG is a wall of path data, and a diff of a PNG is one line
saying the binary files differ. What the human is being asked to approve is
how the picture looks, and the diff said nothing about it. Decided in the
grilling of 2026-10-23. The backlog that builds it is `.tasks/` on the
`svg-diff-previews` branch.

## Drawn as a picture

**A file is a picture by its extension** — `.svg`, and the formats the Code
pane already opens as one — and a picture file in a diff is drawn as a
picture: *Before* and *After* side by side for an edit, stacked on a phone, one
picture for a file added or removed; on a chequerboard, in a box of about a
third of the screen, and a tap opens it at full size in the one modal the UI
has. The markup does not appear on screen. A fold that showed the picture over
the hunks was considered and rejected: the brief asked for pictures *rather
than* code, and a reviewer who wants the attribute that changed has the Code
pane, where an SVG opens as the text it is. A merge commit's *Before* is the
first parent's. The summary row — path, status, the +/− tally for an SVG — is
as it was, and a rename with no change to the bytes draws as it did.

**Both diffs, through the one renderer.** A question set's attached Diff and a
commit's pane are the same HTML from the same renderer, so the picture section
lands in both at once, and the fold, the anchor and the table of contents are
untouched.

**Through an `img`, never inline.** The diff HTML is injected unsanitised —
the renderer escapes every byte of the diff and the markup around it is its
own — and an SVG inlined as markup would run whatever script a session wrote
into it. Inside an `img` nothing runs and nothing external loads, which is the
same reason the Code pane refuses to draw an `.svg` as a picture. The Code pane
is left as it is: an SVG opens there as the markup, which is what one edits.

## Fetched by blob id

**Nothing is stored for a picture.** Every file in a patch already names its
exact bytes: the `index <before>..<after>` line carries git's blob ids, full
ones once the diff is taken with `--full-index`, and one id per parent on a
combined diff. The repository holds the *before* blob always, and the *after*
blob the moment the file is staged or committed — which is what becomes of
work the human approves. While a set is still open and the file is only in
the worktree, the worktree copy is served as long as it still hashes to that
id. So the renderer writes a placeholder naming the repo, the path, the id and
the media type, and the viewer fetches the bytes as the fold scrolls near,
over the API it already uses, caching by id for the life of the page. Older
sets carry seven-character ids, which git resolves wherever they are
unambiguous, so they gain pictures too.

Three other sources were considered:

- **Captured with the set** — the server reading both sides of each changed
  picture as the set arrives and storing them beside the patch. It is
  faithful, and it was the first proposal; it was rejected on the human's
  question of whether the bytes could be fetched on demand instead. Base64 in
  JSON makes 2 MB of pictures 2.7 MB of record, a conversation's whole record
  moving to another device is refused past 64 MB, and the budget would have
  been shared with the transcript.
- **A picture store under the data directory**, like attachments — bytes
  copied at ask time, keyed by hash, served on demand. The record stays small,
  but the store is one more thing a transfer must carry and a cleanup must
  sweep, for bytes git already holds.
- **Live from the worktree and `HEAD`** as they stand when the diff is read.
  Faithful only while the set is open; it drifts the moment the session
  commits.

**What is lost.** An *after* picture of work that is never committed goes with
its worktree, and a blob staged and then unstaged can be pruned by git's own
housekeeping. In both cases the viewer draws what the patch still holds — an
SVG's hunks as code, one line for a raster — and says the picture is no longer
there. That is the honest reading of a repository that has moved on, and it
costs nothing to keep.

**A cap of 2 MB per picture**, the Code pane's own, asked of the object's size
before its bytes are read; over it the file draws as code with a line saying
why. No total per set: nothing is stored, so there is nothing for a total to
bound.

**The bytes go out as a download.** The endpoint serves `octet-stream` with
`nosniff` and an attachment disposition, as an attachment's bytes do, so an
SVG navigated to directly is a file saved and never a page that runs. The
viewer alone turns them into an `img`.

## Shares

A share is one file read with no server behind it, so the server fills every
picture in as it writes one, under the same per-file cap, as a `data:` URI in
the placeholder. The renderer takes a reader of blobs for this: handed none it
writes placeholders, handed the repository it writes the pictures in. A share
whose picture is gone draws what the workbench would — the hunks, or the line.
A share that said each picture did not travel was considered and rejected: a
one-off file can afford its pictures, and a share is read the way the workbench
is read.

## Consequences

- Two `git` reads gain `--full-index` and the viewer's fixtures are rewritten
  with full ids, which is a one-time change to committed files.
- One endpoint, scoped to a conversation and keyed by repo name, blob id and
  path; the worktree stand-in is the one read of a file it makes, bound as the
  Code pane binds one.
- The render crate reads the `index` line, which it ignored, and gains a
  reader-of-blobs seam the share uses.
- The per-file hunks of an SVG still travel in the HTML, hidden while a picture
  draws, so that the fallback and an offline share need no second render.
