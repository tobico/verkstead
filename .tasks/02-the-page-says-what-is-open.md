# 02. The page says what is open

## What to build

A new channel over the bridge on which the page tells the app what the open
conversation can go to — the push the Go menu (task 03) is greyed and enabled
by. **The page tells the app, not the other way round** (Set 1283 Q7): only the
page knows which conversation is open, which device it is on and what its
record holds, so the paths are built where the page already builds them, and
the app learns a list it can draw a menu from. It is a push, like the head's
colours — nothing comes back — and it is the app's window's alone: a browser
on this machine or a phone on the tailnet has no bridge, finds nothing to say
anything to, and says nothing.

**What crosses**, as the shape both sides read:

```ts
/// What the open Conversation can go to, in the Go menu's order, each with the
/// path the page routes to for it — or nothing where no Conversation is open.
type Where = {
  panes: Array<{
    pane: "timeline" | "code" | "backlog" | "roadmap" | "share";
    path: string;
  }>;
};
```

- **timeline** is the conversation's own path, which also closes whatever
  details pane is open. **code** and **share** are always there.
- **backlog** is there where the record has a task list still readable — the
  same rule the backlog card follows, a task-list event whose list is not gone.
- **roadmap** is there where the record landed one, and names the latest: the
  last roadmap of the last stage-list event, by its directory name, one item
  however many roadmaps the worktree holds (Q8c).
- Steer is an action rather than a place and is not among them; a pending
  steer is reached from the status button as it is today.
- Every path carries the device segment when the conversation is a member's,
  built by the page's own path builders, so the app appends nothing.

**When it is pushed.** Once when the page starts listening, on every route
change, and on every refresh of the record that changes the list — the
backlog landing mid-session is the case. Compare before sending: a push on
every poll of an unchanged record is noise the app would have to ignore. The
settings page, the compose page, the bare workbench and a no-such-page all push
an empty list.

**Both sides.** The channel is named beside the others in the bridge module,
the preload sends it, and the app's edge receives it and holds the latest
value for task 03 — logging a terse line, drawing nothing yet. What arrives is
checked against the shape by a module vitest runs before it is held, the way a
set from the page is checked before it reaches the settings file: a bridge
that trusted its renderer would only have moved the question. A malformed push
is dropped and said, not thrown on. The channel lists in both bridge tests
grow by one, and the bridge module's own account of how many channels it has
and which way each goes is brought up to date.

## Acceptance criteria

- [ ] A web test mounting the real route table over a stubbed bridge sees the
      list change as the page navigates: empty on the settings page, the five
      panes on a conversation with a backlog and a roadmap, three without
      them, and a member's conversation carrying its device segment in every
      path.
- [ ] The same record polled again with nothing changed pushes nothing; a
      backlog arriving on it pushes once.
- [ ] A browser page — no bridge — pushes nothing and draws nothing different.
- [ ] A desktop test shows a push of the wrong shape dropped and a right one
      held, through the pure module and not the edge.
- [ ] `pnpm test` passes in `web/` and in `desktop/`, and the two bridge tests
      agree on the new channel's name.
