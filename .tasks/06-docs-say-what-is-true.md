# 06. The docs and the viewer say what is true

## What to build

Stage 03 landed in full, which means the product does not carry an unsandboxed
note any more — it carries the AppContainer asserted as fact, in prose, in six
places. Every one of them is now wrong, and this is the task that corrects them
rather than removes anything.

`CONTEXT.md`'s **Sandbox** term is the one that matters most, because it is the
vocabulary everything else is written against. It names the AppContainer twice —
once as the third of three renderings, once as the reason a session's profile is
made the way it is — and it makes a claim about the network that has stopped
being true: that on Windows an identity is granted the internet and nothing
else, so loopback and the local network are refused. Under the account they are
not refused. The pipe is still there and still the transport a session asks
through, but it is no longer there *because the boundary cannot reach loopback*,
and the term should say why it stays rather than repeating a reason that has
gone.

The other five are `docs/adoption.md` — which also describes one AppContainer
profile made per Conversation, and which gains the elevated install step and
what a session can and cannot get to on this platform — `README.md`, the
viewer's `api/types.ts`, its build-cache settings pane and its setup
instructions.

Two things to get right rather than merely change. The **one account for the
installation** is a real weakening and adoption should say it plainly: a session
can reach every live Conversation's Worktree, and what it cannot reach is the
human's machine. And the **elevated step** is the first thing Verkstead has ever
asked a human to run as an administrator, so it belongs where a reader will meet
it before they need it, in the same voice the unsigned-installer warning is
written in.

## Acceptance criteria

- [ ] `CONTEXT.md`'s Sandbox term names the account, drops the AppContainer, and
  no longer says the network is refused on Windows — while still saying why the
  pipe is the transport
- [ ] `adoption.md`'s Windows section names the elevated step, says what a
  session can and cannot reach, and says that one account serves every
  Conversation
- [ ] `README.md` names the third rendering correctly
- [ ] The viewer's types, build-cache pane and setup instructions no longer
  describe an AppContainer or a refused loopback
- [ ] vitest passes on the corrected copy
- [ ] No file in the tree describes a Windows session as running in an
  AppContainer, except the ADR and the probes, which are the record of what was
  tried
