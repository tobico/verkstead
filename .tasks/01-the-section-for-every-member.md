# 01. The section, for every member

## What to build

Every session launched on a Conversation of a clustered Verkstead — the fresh
prompt and the carried note of a session moved here alike — is primed with a
devices section that names **every member of the cluster**, not just the
permitted ones. Today the section (`skills::transferable`, fed by
`transfers::may_go_to`, applied at the launch point in `sessions.rs`) lists only
the drafting device and the ticks, less this one, and is drawn not at all where
that list is empty. That rule — *nothing where the list is empty* — goes
(ADR-0029, *Consent stays off by default, and is told*).

The shape, as settled with the human (Set 1533):

- **A head sentence naming this device**: its name, its id and its OS word, in
  the platform's own word as the Device Reading spells it
  (`crate::device::Devices::os`, *Linux (WSL)* under WSL). It gives *something
  this platform cannot do* a referent.
- **Then a list of the other members only**, each with name, id, OS word
  (`store::Member::os`) and a mark saying whether this Conversation may move
  there. "Permitted" keeps today's rule (`permitted_among`): the drafting device
  plus the ticks, never this device.
- **Ordered permitted first** — the drafting device, then the ticks in the order
  they were made, as `permitted_among` orders them today — **then the rest by
  name**.
- **No awake/asleep mark.** The prompt is a snapshot taken at launch, and
  `verkstead transfer` checks reachability itself.
- **What to do**: `verkstead transfer <device>` for a permitted one, as today;
  for one that is not permitted, **ask the human with `verkstead ask`** — they
  can tick it from the Set's Attach menu, under *May be transferred to*
  (ADR-0028), and the transfer reads the list live, so once the Set is answered
  the call can be made. This is the route that exists until stage 04 makes
  `verkstead transfer` a request; do not write that one.
- **Nothing at all on a single-device Verkstead** (no cluster, or no member but
  this one), as today.

Words that reach the agent follow `docs/copy.md` and `CONTEXT.md` (Permitted
Device, Transfer, Device Reading — none of the *Avoid* words). Keep the
section's wording such that task 02 can make the Guide say the same thing.

Rewrite the **Permitted Device** entry in `CONTEXT.md` for the part this makes
true — every session is told every device, permitted or not — and trim that
clause out of its *Being revised by ADR-0029* pointer, leaving the rest of the
pointer for the stages that make it true.

## Acceptance criteria

- [ ] A session on a Conversation with nothing ticked, still on its drafting
      device, on a cluster with a Windows member, is told this device's OS, is
      told the Windows member exists by name, id and OS, that it may not move
      there yet, and that it asks with `verkstead ask` and the Attach menu's
      *May be transferred to* tick.
- [ ] Permitted members are listed first in today's order and marked as such; a
      session carried onto another device names its drafting device as
      permitted; the carried note carries the section too.
- [ ] A single-device Verkstead's prompt carries no devices section and no word
      about transfer, as today.
- [ ] `CONTEXT.md`'s **Permitted Device** says every session is told every
      device, and its ADR-0029 pointer no longer claims that as pending.
