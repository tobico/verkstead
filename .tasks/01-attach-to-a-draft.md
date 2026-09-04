# 01. Attach a file to a draft

## What to build

The whole path once, on a saved draft: a press on the composer picks a file,
the file lands in the Conversation's own directory under the Data Directory,
and a pill under the Brief text says it is there.

**The record.** An `attachments` side table keyed by conversation, the way
every other per-Conversation fact is kept — the `conversations` table itself
is STRICT and left alone. Each row holds its own id, the file's name as it
stands on disk, its size in bytes, when it was added, and an **origin**: what
the file was attached to. The Brief is the one origin there is now; an Answer
to a Question Set is the one planned next, and it is a second value here rather
than a second table. Removal and every later reference go by the row's id, not
the name.

```sql
CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id),
    origin TEXT NOT NULL,          -- 'brief' for now
    name TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    added_at TEXT NOT NULL
) STRICT
```

**The bytes.** One flat directory per Conversation, `attachments/<id>/` under
the Data Directory, made on first use and shaped on the handoffs module: a root
beside the worktrees, one directory per Conversation named by its id, nothing
to keep in a table about which is whose. A file keeps its own base name.
Anything that is not a plain base name — a separator, a leading dot, an empty
name — is refused. A name already in the directory is not replaced: the
newcomer becomes `name-2.ext`, then `name-3.ext`, counting up over the
extension-less stem, with no spaces or brackets, so the path is one an agent
can type. Both files stay and both are records.

**The wire.** Upload is one request per file, the raw bytes as the body and the
name in the path, under the Conversation's own `/api/ui/conversations/{id}/`
routes — no multipart, and its own body limit of 32 MiB over the router's
default, the way the Question Set route raises its own. The upload answers with
the record it made, name as renamed. Removal is a POST by attachment id like
the rest of the API. Both are refused once the Brief is frozen, and an upload
over the limit is refused with a reason the composer can say. The upload is
scoped to the Conversation rather than to the Brief on purpose: what a file is
attached to is its origin, so an Answer's upload later is the same route with
another origin. The Conversation view carries the list — id, name, size,
origin — and the generated TypeScript types and the golden fixtures move with
it.

**The composer.** On a saved draft's composer, an `IconButton` with Font
Awesome's paperclip stands at the near edge of the row the Start work press is
at the far edge of — the draft composer has only the one press, so the
paperclip is left of it. Pressing it opens the browser's file picker, several
files allowed, each uploaded on its own. The attached files are a row of pills
between the Brief text and the Setup row inside the box, each showing the name
truncated with a remove press labelled for that file, in the house style of a
companion row. A pill on its way up is drawn dimmed until the record comes
back. A refused upload is said on the composer where refused fields are said.
Once the Brief is frozen the paperclip and the remove presses are not drawn.

Any file type. Folders are skipped.

## Acceptance criteria

- [ ] Attaching a file on a draft's composer puts it at
      `attachments/<conversation id>/<name>` under the Data Directory, a row in
      the `attachments` table, and a pill on the composer that a reload still
      shows.
- [ ] Removing a pill takes the row and the file; attaching the same name twice
      draws two pills, the second named `name-2.ext`.
- [ ] A file over 32 MiB, an upload to a Conversation whose Brief is frozen, and
      a name that is not a plain base name are each refused with a line on the
      composer, and nothing lands on disk.
