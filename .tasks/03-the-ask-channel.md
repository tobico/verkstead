# 03. The ask channel per agent type

## What to build

The same `verkstead ask` on every backend, treated differently by the server
depending on which backend sent it. Which channel a Set was asked on is the
backend's fact rather than the Set's: the CLI says nothing about it, and the
server knows the Conversation's session's agent type.

**A third state beside blocking and deferred.** A store-and-nudge Set is stored
as a deferred one is — the same row against the same Set, so the folding rule
reaches it unchanged — and marked as the kind of ask a session is idling on:

```
Blocking        no row            a session is idling on it
StoreAndNudge   row, so marked    a session is idling on it
Deferred        row, so marked    nobody is idling on it
```

**Four readers ask whether anybody is idling on a Set, and all four count the
new mark as open.** They exclude a Deferred Ask today by the row being there at
all, which is what has to become a question about the mark instead:

- `store::unanswered_set_since`, and through it `runner::asking` — so the quiet
  grace does not end a session waiting on its own store-and-nudge ask.
- `store::open_set`, and through it `runner::open` and Rescue — so a session is
  not prodded twice and stopped before the human has answered, leaving nothing
  to nudge.
- `store::conversations::proposals`, and through it `last_proposal` and
  `last_batch_proposal` — a Set that holds a session open holds its wrap-up
  open too, so a review is not settled over one.
- `sets::open(.., Open::Blocking)` — the Sets locked unanswered when a grilling
  relaunches over the session that asked, or a Conversation closes. The session
  idling on a store-and-nudge Set has gone in both cases, so it locks with the
  blocking ones rather than being left standing as a deferred one is.

`--deferred` goes on meaning an ask nobody is idling on, on every backend.

**The CLI has to be told not to wait.** It asks the same way everywhere, so
what says the Set was stored rather than waited on is the server's reply to the
ask: it grows a field saying so, and the CLI prints the stored Set and returns —
the id and when the server took it, exactly what `--deferred` prints today, and
the id `verkstead answers` is given. An older CLI that ignores the field opens a
wait, which is the shipped-together case and not one to design around.

**The Timeline and the badges say a deferred-shaped ask**, which they already
know how to do: nothing is holding a connection open on a store-and-nudge Set,
so what the human sees is the same as a deferred one. What differs is entirely
underneath.

## Acceptance criteria

- [ ] A stub-backend `verkstead ask` with no `--deferred` returns at once with
      the stored id, and its session is neither ended by the quiet grace nor
      rescued while the Set stands unanswered.
- [ ] An ask sent with `--deferred` on that same backend still ends its session
      as today, and a Claude ask still blocks until the Response arrives.
- [ ] A store-and-nudge Set left behind by a session that has gone is locked
      unanswered when its grilling relaunches or its Conversation closes, and
      holds a wrap-up's review open while it stands.
