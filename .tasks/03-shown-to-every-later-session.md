# 03. Shown to every later session

## What to build

Every session started on a Conversation after a memo was left is told it in
its prompt — the whole list of unretired memos, oldest first — until it is
retired.

Appended where every session's prompt is finished, the one place every launch
goes through, so a grilling relaunch, a steered session and every step get it
alike and a prompt builder added later cannot forget it. It goes after the
kind's own sections and before the companion repositories listing, so that
the prompt reads documents, then what earlier sessions left, then the facts of
the machine. The shape, which the Guide in task 06 quotes:

```
# What earlier sessions left for you

**Memo 42**, left by the session that worked task 03.
Retire it with `verkstead memo --retire 42` once it no longer holds.

<the memo's markdown, as written>
```

One block per memo, a blank line between them, and nothing at all — no heading
— where the Conversation has none or every one is retired. Unlike a deferred
Set's answers, nothing is marked delivered: the memos are read off the record at
every launch and a memo is not spent by being read.

Two places deliberately get nothing. A carried Conversation's note — the prompt
a session resumed after a Transfer is primed with instead of the ordinary
one — says nothing of memos: that session was launched after they were left
and has already read them. And a Stage's sessions read the Stage's own memos,
which is already what reading off the Conversation gives.

## Acceptance criteria

- [ ] A memo left by one session appears in the prompt of the next session on
      the Conversation, under the heading and in the shape above, and a second
      memo follows it oldest first.
- [ ] A relaunched grilling's prompt and a steered session's prompt carry it
      too.
- [ ] A Conversation with no memos launches a session whose prompt has no
      heading for them.
- [ ] The note a carried Conversation is primed with after a Transfer is
      unchanged.
