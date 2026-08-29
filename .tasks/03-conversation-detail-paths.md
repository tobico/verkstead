# 03. Paths for conversation details panes

## What to build

The details-pane selection moves from a page-local signal into the URL, so it
survives navigating away and back. Today only the Conversation is in the URL;
after this, each thing the details pane can show has a path nested under its
Conversation:

- `/conversations/:id/events/:event` — a Timeline event with a full self
  (output, Question Set, commit, pull request, Brief, handoff, steer
  document, notice). The `events/` segment exists so ids can never collide
  with the word-named panes beside them (a bare `:event` segment was
  considered and rejected for that reason).
- `/conversations/:id/backlog` — the backlog, selected by word as today
- `/conversations/:id/roadmaps/:name` — a roadmap, by its directory name

The selection is derived from the URL rather than held beside it: one account
of what is open. An id or name that matches nothing on the loaded
Conversation leaves the pane empty, as a stale selection does today.

History rules, settled in the grilling:

- Navigations that change the page level — entering a Conversation, leaving
  to the list, entering `/settings` — **push**.
- Navigations that change only the detail segment — a Timeline card press,
  switching between details — **replace**, so walking between details never
  grows the history stack. Back from a detail leaves the Conversation
  entirely.

On a phone, which pane shows still follows the walk: opening a Conversation
shows the Timeline level, and a card press moves to the details level. A cold
load of a detail URL (reload, kept link) lands directly on the details pane —
the URL names the detail, so that is what is shown.

The server's SPA fallback already serves every non-`/api` path, so no server
change is needed.

## Acceptance criteria

- [ ] Pressing any Timeline card (events, backlog, roadmap) rewrites the URL
      to its path with replace, and the pane opens from the URL on a cold
      load too
- [ ] Back/forward respect the push/replace rules: switching details adds no
      history entries; Back from inside a Conversation leaves it
- [ ] On a phone, a cold load of a detail URL lands on the details pane, and
      the in-page walk (← Conversations / ← Timeline / Details →) still works
