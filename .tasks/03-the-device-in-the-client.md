# 03. The device in the client

## What to build

The viewer learns an optional device, and a remote Conversation becomes an
ordinary page. Opening `/devices/{device}/conversations/{id}` on A draws the
Conversation that lives on B — its Timeline, the details pane over every leaf it
already has, its Sets — and every ordinary press on that page lands on B.

Four things take the device, and they are one seam each:

- **The route table.** `/devices/:device/` above the Conversation and every leaf
  nested under it. The leaves draw nothing today — what is open is read off the
  URL — so this is a parent above a parent rather than a second list of them.
  Local URLs keep their shape: `/conversations/:id` is untouched, because this
  device is where most of the work is and a device segment on every URL would
  say nothing.
- **The path helpers.** Where a Conversation stands, where one of its panes
  stands, and what a path says is open all take an optional device and read one
  back. A path built with no device is the path it is today, character for
  character.
- **The fetch helpers.** One place where `/api/ui/…` becomes
  `/api/ui/members/{device}/…`, so that each call site says which device it is
  for and nothing else composes a path. The socket helper takes it the same way,
  though nothing answers a relayed socket until the next task.
- **The query keys.** A Conversation-scoped key carries the device, so B's
  Conversation 4 and this device's Conversation 4 are two entries in one cache
  rather than one entry drawn twice. The nudge table that names which keys a kind
  stands for follows them.

**The cache hazard is the point of the third criterion.** Ids are each device's
own and they collide by construction — every Verkstead issues a 1 — so anything
keyed by a bare Conversation id reaches the wrong Conversation the moment two
devices are in play. That is the query keys and it is also the Code pane's own
subscription to `files`, which is held outside the cache and keyed by
Conversation today.

**The Nudge stream stays local and gains nothing.** The page listens where it
always did, on this device's own stream; making that stream carry a member's
news is the last task's, so a page opened on a remote Conversation reads fresh
when it is opened and on coming back, and not yet on a Set answered over there.

**And there is no way in but the URL.** Nothing in this stage puts a remote
Conversation in the sidebar — the merged list is stage 06 — so the demonstration
is a URL opened by hand, and the sidebar goes on listing this device's own.

## Acceptance criteria

- [ ] Opening `/devices/{device}/conversations/{id}` on A draws the Conversation
      that lives on B — Timeline, panes, Sets — with A's cookie and nothing else
      in the browser.
- [ ] Every ordinary press on that page lands on B: answering a Set with a file
      on an Answer, the actions menu, a steer, and the Repo dropdown's Open and
      Create.
- [ ] Navigating between a local Conversation and a remote one keeps both caches
      and neither draws the other's rows; a local URL and a local call are
      unchanged.
