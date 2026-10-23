# 01. A picture is served by its blob id

## What to build

Two things that make a picture fetchable: every diff names its bytes in full,
and one endpoint serves them. Nothing is stored for either (ADR-0026, *Fetched
by blob id*).

**Full ids on every diff.** Both reads of git that feed the renderer — the
uncommitted Diff composed onto a question set as it arrives, tracked changes
and each untracked file against `/dev/null` alike, and the patch read for a
commit's pane and for a share — are taken with `--full-index`, so each file's
`index <before>..<after>` line names a whole blob id on each side rather than
a seven-character prefix. A combined diff names one id per parent before the
`..`. Nothing else about either read changes, and the fixtures `cargo test`
writes for the viewer are rewritten with the full ids in them.

**The endpoint.** `GET /api/ui/conversations/{id}/pictures?repo=<name>&id=<blob>&path=<path>`
serves the bytes a diff names, under the workbench key like every `/api/ui/`
route. `repo` is the name a Diff's block carries — the conversation's own repo,
or a read-write companion's — and resolves to that repository's worktree
exactly as the Diff was composed from it; a name that is neither is refused.
The blob is read out of that repository with `git cat-file blob`, which
resolves a short id from an older set wherever git finds it unambiguous.

**The worktree stands in for a blob not yet staged.** The after blob of a file
that is only in the worktree is not in the object database until the file is
staged or committed, so where `cat-file` finds nothing, the file at `path`
under that worktree is hashed the way git hashes a blob and served when the
hash matches `id` — matched as a prefix, so a short id matches too — and
refused when it does not. `path` is bound inside the worktree the way the Code
pane binds a file read: nothing outside it, and nothing under `.git`.

**The cap and the headers.** Over 2 MB — the Code pane's own cap, decimal, and
asked of the object's size before its bytes are read — is refused with a
sentence rather than served. What is served goes out as
`application/octet-stream` with `X-Content-Type-Options: nosniff` and a
`Content-Disposition: attachment`, the way an attachment's bytes do, so an SVG
navigated to directly is a download and never a page that runs. The viewer is
what turns the bytes into an `img`, in task 02.

**Off the runtime.** Reading git and hashing a file happen in a blocking task,
as the file read and the commit pane's render do.

Every refusal is one sentence in the plain register of `docs/copy.md`: a repo
this conversation has no worktree of, a picture that is no longer there, a
picture too large to draw.

## Acceptance criteria

- [ ] A committed or staged blob named by a set's Diff or a commit's patch comes back byte for byte, asked for by its full id and by a seven-character prefix.
- [ ] A file only in the worktree is served while it still hashes to the id in the Diff and refused once it has changed; a path outside the worktree or under `.git` is refused; a repo name the conversation does not carry is refused.
- [ ] A blob over 2 MB is refused with a sentence; served bytes carry octet-stream, nosniff and an attachment disposition.
- [ ] Every file in a set's composed Diff — tracked and untracked — and every commit's patch carries full ids on its `index` line, and the viewer's fixtures are rewritten with them.
- [ ] The server suite covers each of the above.
