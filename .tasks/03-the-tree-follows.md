# 03. The tree follows

## What to build

The Code pane hears the `files` Nudge for the Conversation it is drawn on, and
re-reads the folders it has expanded.

**It cannot come off the nudge table alone.** That table maps a kind to query
keys, and the tree's folder listings are not queries: they are held above the
pane with the tabs and the unsaved text, so that a swap to an Event and back
does not throw away the walk down to a file. So the pane takes out a
subscription of its own on the stream while it is drawn — one seam in the module
that already owns the stream, beside the invalidation the table does, rather
than the tree being rewritten as queries.

What a re-read comes to is drawn in place. A folder that has gone collapses and
takes its rows with it; a folder that has appeared under an expanded one is
drawn where it belongs; a folder nobody has expanded costs no read at all, which
is the whole reason the tree reads one folder at a time. A Nudge that lands
while somebody is typing a name into a row's field, or with a row's menu open,
leaves both where they are.

And the places the code says there is no watcher until this stage — the tree's
own account of why a collapse is the only honest moment, the files API's, and
the client's beside the folder and the delete — say what is true instead.

## Acceptance criteria

- [ ] `mkdir` in a terminal tab draws the folder inside an expanded parent
      without a press, and `rm -r` takes it and its rows away.
- [ ] An expanded folder that has gone is no longer drawn as expanded.
- [ ] A Nudge costs one read per expanded folder and none for the collapsed
      ones.
- [ ] A name half typed into a row's field is still there, with what was typed
      in it, after a Nudge lands.
