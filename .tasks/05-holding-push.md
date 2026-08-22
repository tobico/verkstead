# 05. A Hold nobody came back to

## What to build

A Hold that has stood for a while tells the human's devices about it, the way a
Question Set does. One push per subscribed device, once. Handing back before
the interval passes sends nothing, and a Hold that goes on being held does not
push again.

A push notice can only name a Question Set today, and the service worker opens
the Set page from it. It grows a way to name somewhere else, and for a Hold
that is the held Conversation — so a phone woken by one lands on the session it
is holding rather than on a Set it is not about. A Question Set's push still
opens that Set.

Nothing about it reaches the Timeline, and nothing about it releases the Hold.
It is a reminder; the hand-back is still the only way back.

## Acceptance criteria

- [ ] A Hold left standing for the interval pushes once to every subscribed
      device; a hand-back before then pushes nothing, and a Hold that keeps
      standing pushes no second time.
- [ ] Tapping the notification opens the Conversation whose session is held.
- [ ] A Question Set's push still opens that Set.
- [ ] Nothing about the push or the Hold reaches the Timeline.
