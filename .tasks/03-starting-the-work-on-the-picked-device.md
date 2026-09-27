# 03. Starting the work on the picked device

## What to build

*Start work* and *Save as draft* create the Conversation **on the device the
select names**, and the page lands on it there.

**What the compose page holds gains the device**, so the draft this browser keeps
knows which machine its Repo id, its companions and its Pairings belong to, and
is read back the same way it was written — a stored body naming something else
is dropped on the way past as it is today, and a stored device that is no longer
a member reads as this device, the same fallback the select makes.

**The replay goes through the Relay unchanged.** The Conversation is started
against its Repo on that device and then a request per touched field, exactly as
it is now: no batched create, no second set of validation rules, and every
refusal the far end's own in the words the composer says them in. The files held
in the page go up the same way, one request each, after the fields and before the
kickoff — what is attached freezes with the Brief. And the kickoff is that
device's: the grilling for work of its own, the adoption for a loaded stage, the
take-up for a loaded pull request.

**What the replay could not do is left against the Conversation and the device**,
rather than against a number. Ids collide by construction — every Verkstead
issues a Conversation 1 — so the handoff that carries a create's refusals to the
draft it made is keyed the way a sidebar row is, or the refusals from a create on
one machine are drawn on the composer of an unrelated Conversation on another.

**The page lands on the remote URL** — `/devices/{device}/conversations/{id}`,
which is the path a member's Conversation already stands at. From there the
composer, the Timeline, the grilling and the panes beside them are what they
always were, reading that device. And the Conversation appears in the Merged List
under that device, with its OS mark and name on the row.

**The Repos this page reads to draw names with follow the pick too**: the trigger
that says which repository the work is in and the rows the companions are drawn
as are read off the picked device's registry, not this one's. The two lists that
load work from somewhere else stay this device's own, which is why the select is
settled while either is loaded.

End to end from A's browser with B linked: pick B, pick a Repo on B, write a
Brief, press *Start work*, and the grilling is running on B with the page
standing at B's URL and the row in the sidebar under B.

## Acceptance criteria

- [ ] *Start work* with a member picked creates the Conversation on it, puts every
      touched field and every held file on it, kicks the work off there, and the
      page lands on `/devices/{device}/conversations/{id}`.
- [ ] *Save as draft* does the same and stops after the fields, and the new draft
      reads in the Merged List under that device.
- [ ] What the replay was refused is said on the composer of the draft it made,
      and not on a Conversation holding the same number on another device.
- [ ] The Repo named in the trigger and the companion rows are read off the picked
      device's registry.
- [ ] With this device picked, both presses do exactly what they did before —
      the same paths, the same landing.
