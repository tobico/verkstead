# 01. The sidecar streams its notices

## What to build

A local stream of every notice the server pushes to its phones:
`GET /api/ui/push/notices`, a server-sent-events stream under the push prefix,
which is one of the prefixes never served over the cluster link, so a member
can never hold it. Each event is the same JSON a phone's service worker is
handed — the path a tap opens, the title, and the repo where there is one.

The one function every push already passes through announces each notice on a
broadcast channel before it goes looking for subscribed phones — today it
returns early when none is subscribed, and a desktop with no phone paired is
exactly the install this is for. That covers everything in one place: a Set
arriving, a stop, a session gone idle, an account out of window or signed out,
a pull request, the three roadmap moves, Done, a device asking to join, and a
member's relayed news with the device leading its title. The channel is held
the way the push module already holds the cluster it owes news to, keyed by
the store, since the places that send news hold a pool and nothing else.

A stream nobody holds costs nothing and queues nothing; a reader that falls
behind has its stream ended and reconnects, as the Nudge stream does. The
phones' own push is untouched.

## Acceptance criteria

- [ ] A stream held open while a Set is asked sees one event carrying the
      Set's Timeline Event path, its title and its project
- [ ] The same stream sees a Conversation reaching Done, and a member's news
      relayed over the Peer Listener with the device in front of the sentence
- [ ] Nothing is announced to a phone that was not announced to the stream,
      and the push delivery suite passes unchanged
- [ ] The route is refused over the link and without the Workbench Key
- [ ] CONTEXT.md's push and Nudge wording names the stream as the third
      carrier of news, beside the phones' push and the Nudge
