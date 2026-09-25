# 02. The word on the wire, and where it is read back

## What to build

**A render `Process` enum with the viewer's words**, beside the render
`Lifecycle` and paired with the store's the same way — one word either side, and
one function where the two vocabularies are held to each other. `verkstead-render`
is what emits the generated TypeScript, so a render enum crosses the wire exactly
as a schema one does, and the generated types have to be regenerated and
committed or CI fails on the drift.

**The Process on the Conversation view**, beside the Pairings and the Direction.
A published share carries the same payload functionally updated, so the field
reaches a share without anything being written for it.

**A per-field endpoint beside the Pairing ones** that sets a drafting
Conversation's Process, answering in the body the way every other field on that
row does:

- the Process was recorded;
- there is no Conversation with that id;
- **it is past drafting, or its branch is already cut** — the same two questions
  the Repo switch asks, under the same one word, because a Conversation with a
  worktree is not one a dropdown rewrites;
- and **the Process's stage has not landed**, which is a refusal of its own
  rather than that one: the record holds all five and only Develop can be picked
  on, so four of them are refused by a name that says why. Every refusal is the
  server's; a check the browser made is a courtesy, and the endpoint is reachable
  without one.

Which Processes have landed is the server's list, and the picker's rows are the
viewer's — two places a later stage adds to, so each says plainly that the other
exists.

**Nothing is written at Start.** The Pairings this is frozen beside are frozen by
the refusal alone: every field endpoint refuses from the moment a worktree row
exists, and the start transaction writes no Pairing. So a row means somebody
picked, no row means the reading stands, and a Develop Conversation nobody
touched the picker on keeps no row at all.

**And the word is read back on the pane a frozen Brief opens**, as one more of
the setup facts it summarises — beside Repo, Branch and Base rather than in the
half that reports the worktree path and the three Pairings. The Process is a fact
about the work rather than about this machine, so a published share says it too,
which that half is precisely what does not. The sidebar row and the card are
untouched: the state word already says where the work is.

The viewer's word for each of the five goes in one place, the way the lifecycle
states' and the directions' do — the thing picked and the thing read back cannot
be allowed to come to be called different things.

## Acceptance criteria

- [ ] The view of a fresh Conversation says Develop, and one holding a pull
      request says Review.
- [ ] The endpoint records Develop on a Draft; refuses a Conversation past
      drafting and one whose branch is cut under the same word; and refuses each
      of the other four by the name that says their stage has not landed.
- [ ] A frozen Brief's pane says the Process among its Repo, Branch and Base
      facts, a share of that Conversation says the same, and nothing on the
      sidebar row or the card changed.
