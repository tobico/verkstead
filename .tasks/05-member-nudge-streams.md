# 05. Member nudge streams on the hub

## What to build

The last thing between a remote Conversation and a local one: freshness. This
device holds **one Nudge stream to each of its members**, over the Peer
Listener, and re-announces every nudge that comes down one **locally, as the
same kind, under the device it came from** — so an open page invalidates the
device-keyed queries it drew and reads back what moved, without a poll and
without a reload (ADR-0020, *The opened device relays*).

The stream is held by the server rather than by the browser, which is what makes
one connection per member serve every page this device has open — and why a
phone on the tailnet hears about B at all.

**A Nudge has nowhere to put a device today.** It is a tagged enum of a kind and,
where the change belongs to one, a Conversation id, and the viewer's own table is
what maps a kind onto the queries it stands for. Putting the device on it is a
decision to make deliberately: it is what the client's table keys the
invalidation by, it is what tells a member's news from this device's own, and a
local nudge has to go on meaning exactly what it means now.

**Reconnecting is the behaviour rather than an error path.** A member restarts,
a laptop's lid shuts, a link drops: the stream is taken up again, and a stream
that has just come back knows nothing about what it missed. So coming back
announces enough under that device for a page drawing it to read back what it is
showing — the same reaction the browser's own stream makes of a reconnect, aimed
at one device's queries. There is no kind for *everything of this device* today;
whether that is a new one or an existing one said widely is this task's to
settle.

**Unreachable reads the way a dropped stream reads.** Nothing new is drawn for a
member that is not answering: the page keeps what it last read and goes stale,
exactly as it does when its own stream is down, and a press on it is refused by
name by the relay. Dimming a row and keeping its last rows in memory is the
merged list's, in stage 06, and the member's Conversation list is not kept here.

## Acceptance criteria

- [ ] A Set answered on B refreshes A's open page on that Conversation without a
      reload, and a session printing on B moves A's Timeline and Screen.
- [ ] A member's stream that drops is taken up again, and coming back announces
      enough under that device for an open page to read back what it missed.
- [ ] A local nudge carries no device and invalidates the local queries exactly
      as it does today; a member's nudge invalidates that member's and leaves the
      local ones alone.
