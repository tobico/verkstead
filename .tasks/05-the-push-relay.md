# 05. The push relay

## What to build

A phone installed from one device hears from all of them. A member's push news
reaches the devices its cluster-mates push to, and is shown there with the
device leading the title — *B — the branch is done* — so one phone is enough for
a cluster and a lock screen says which machine the work was on.

**The member tells its members as the news happens**, over the Peer Listener and
behind the Member Gate, the way it announces a renewed certificate or a device
it has just let in. A hub cannot subscribe to a member's push: the
subscriptions a device pushes to are its own browsers, which is one of the three
prefixes never served over the link. And a **Nudge** is the wrong carrier by
construction — it says what kind of thing moved and never what it was, and a
notification is a sentence.

So the one place that already tells this device's own browsers about a piece of
work gains a second step: after the local push goes out, the news goes to every
member. Behind what it is announcing, never in front of it, exactly as the local
push is — the record is what matters and a machine that cannot be reached costs
a notification and nothing else.

**What travels is what the far end needs to title it and route a tap**: the
sentence the member would have shown, the repository under it, and which
Conversation on which device it is about. The receiving device writes the title
it shows — the device name and then what it was told — rather than passing a
sentence through untouched, and it bounds both the device name and the incoming
sentence the way a title already bounds the one thing in a notification that
another machine wrote. A news kind a newer member has and this one has not still
reads, because what arrives is prose rather than a variant to match on.

**And the tap opens the Conversation where the phone can reach it**: the hub
rewrites the path onto its own device segment, so a notification about B's work
tapped on a phone that only ever talks to A opens at `/devices/{device}/…` on A.
A path taken verbatim would open A's Conversation of the same number, which is
the id collision the whole stage is about.

**A member that is off at that moment costs a notification.** Nothing is queued
and nothing is retried, which is what a push service that cannot be reached
already costs, and it is the same bargain: the Timeline says it in full either
way, and this is only what reaches a pocket.

CONTEXT.md's **Relay** entry gains the news going the other way: what a member
tells its members when it has something worth a phone, and that the device
leads the title.

## Acceptance criteria

- [ ] A stop on B lights a phone subscribed to A with the device leading the
      title, and tapping it opens B's Conversation on A at that device's URL.
- [ ] A's own news is unchanged — no device leads a title about this device's
      own work, and the repository still stands under it.
- [ ] A member that is switched off when the news happens costs the
      notification and nothing else: the stop is on its Timeline and the local
      push went out.
- [ ] A device that is not a member is refused the announcement by the gate.
