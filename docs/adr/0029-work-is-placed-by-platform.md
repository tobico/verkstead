# Work is placed by platform

Amends [ADR-0020](0020-cluster-mode.md)'s *The agent's call*, which told a
session about moving only where the human had ticked a device and refused a
call on any other; [ADR-0021](0021-parallel-stages.md), which read a stage's
`on windows` off its line and acted on it with nothing, naming *placing a stage
on a device by its platform, and what happens where no matching device is up*
as a grilling of its own; and [ADR-0028](0028-adding-from-an-answer.md), whose
Attach menu was the one place an answer could tick a device. All three stand
where they stood; this is the grilling ADR-0021 deferred.

What forced it: agents working a cross-platform application were fixing
Windows failures from Linux by pushing and reading CI, with a Windows device
linked to the cluster the whole time. Three things kept them there. Nothing was
ticked under *May be transferred to* because the human rarely thought to, and a
session whose prompt named no permitted device was told nothing about moving at
all. A stage line could say `on windows` and nothing placed it. And a device
that was asleep was a refusal rather than a question. Decided in the grilling
of 2026-10-30 (Sets 1524 to 1530). The roadmap that builds it is
[`docs/roadmaps/platform-placement/`](../roadmaps/platform-placement/ROADMAP.md).

## Consent stays off by default, and is told

**Nothing is ticked by default, and a tick still means automatic permission.**
Ticking every device on a fresh draft was offered and turned down: the point
of the ticks is that the agent may move without asking, and that is not a
thing to hand out unasked. What changes is that the agent can always *ask*.

**Every session's prompt lists every device of the cluster** — name, id,
operating system, and whether this Conversation may move there — and says
which operating system this device runs, so *what this platform cannot do* has
a referent. Today the prompt names the permitted devices alone, and an agent on
a Conversation with nothing ticked cannot ask for a Windows device it has never
been told exists.

## Declaring what the work wants

**`on linux`, `on macos` or `on windows` on a stage line is a preference**, not
a requirement: Verkstead tries hard to place the work there, and runs it where
it stands only when the human says so. A second spelling for a hard requirement
was offered and turned down. **A task line in `.tasks/TODO.md` carries the same
declaration**, read by the same `declarations::read` a stage line is. Only
the reading is shared: a task line's platform is read alone, with none of a
roadmap's all-or-nothing rule, and a word that is not one of the three is
refused when the backlog lands, at the `verkstead done` of the session that
wrote it.

**A staging or breaking-down session writes it only where the work can be
verified nowhere else**: code under a platform `cfg`, a platform's own
component — ConPTY, AppContainer, an installer, a bundle — or a suite that only
runs on that operating system. Naming a platform in prose declares nothing; a
change to shared code with a cross-platform suite declares nothing.

**The closing `proposal` says what the work wants.** It gains `wants`, a list of
the three platform words, each with the reason the work wants it:

    proposal:
      direction: roadmap
      rationale: |
        …
      wants:
        - platform: windows
          for: the ConPTY terminal stage and its tests

Any other word is refused, as it is on a stage line. The chooser draws the
*May be transferred to* ticks under the three directions, devices of the wanted
operating system first, and says when the cluster has no device of it. This is
the moment the human was already deciding how the work is built, so it is the
moment they are asked to let it move — the alternative, a tick hidden in an
Attach menu they had to think to open, is what nobody ticked.

## Placing declared work

**Verkstead moves declared work; the agent never calls `verkstead transfer` for
it.** A stage is made on its roadmap's home device and moved by the ordinary
**Transfer**; a task's session is launched after the Conversation has moved. An
inline run whose proposal wants one platform starts there.

**The device is the first ticked device of that operating system that is
reachable.** Where none of them is reachable, or none is ticked, the human is
asked — see below. A roadmap's stages inherit the roadmap Conversation's ticks
through the one inheritance funnel that already carries its Pairings and
companions.

**The work stays where it is.** It does not come home after the platform work,
before the finish or before the wrap-up: a wrap-up that had come home could be
stuck fixing a Windows check on Linux, which is the position this whole
decision exists to end. Coming home before the finish was picked once and taken
back for that reason.

**No device is ever inferred from a check's job name.** *Rust (windows-latest)*
is a name, not a declaration; the agent reads the failure and decides.

## Verkstead asks with a Question Set of its own

**Where the wanted platform has a device that is linked but not ticked, or
ticked but asleep, Verkstead puts a Question Set of its own**, the first Set
Verkstead has ever authored. A stop with a split Resume row and a Timeline card
with two presses were both offered; the Set was chosen because an open Set is
already what draws *Blocked on you* and reaches a phone, and because its Attach
menu already offers the tick. The Options are what the human can do about it,
and waiting is not one of them:

| Case | The Set says | Options |
| --- | --- | --- |
| Ticked, asleep | *Stage 03 wants Windows. nuc-win is not awake, and the work moves there by itself once it is.* | **Run it here** |
| Linked, not ticked | *Stage 03 wants Windows. nuc-win is linked but not permitted.* | **Move to nuc-win** · **Run it here** |
| The agent's call, not ticked | *The agent asked to move to nuc-win, which is not permitted.* | **Move to nuc-win** · **Keep it here** |
| The agent's call, ticked but asleep | *The agent asked to move to nuc-win, which is not awake, and the work moves there by itself once it is.* | **Keep it here** |
| No device of it linked | *Stage 03 wants Windows. No Windows device is linked.* | none — a Notice, and the work runs here |

**A device both unticked and asleep is asked about as unticked.** *Move to
nuc-win* ticks it, and what follows is the asleep case: the Set is locked, its
line saying the device was permitted, and the asleep row's Set goes up in its
place as the wait, so *Run it here* or *Keep it here* is still there to press.

**A device that answers before the human does settles the Set itself**: the
move runs and the Set is locked, its line saying the device answered, because
the tick already was the consent. A ticked device being asleep is a stop that
lifts itself.

**The open Set is the wait.** A *Wait for nuc-win* Option was written first
and taken back: answered, it would close the one Set that could still say
*Run it here*, leaving work waiting on a device that might never wake and no
press to refuse with. So the asleep case has no answer meaning *wait*: the
human wakes the device or presses **Run it here**, and either settles it.

**`verkstead transfer` on such a device is accepted as a request rather than
refused**: the turn ends, the same Set goes up, *Move to* ticks the device and
moves the work, and *Keep it here* resumes the session with a note saying the
work stays. A refusal that told the agent to ask with `verkstead ask` was
offered and turned down; the Attach menu route stays as it was. The words are
`docs/copy.md`'s: a verb on the button, a full sentence naming the device above
it, and the notification naming the branch and the device.

## Home drives a roadmap across devices

A roadmap's standings, chain, join queue and places are read out of one
device's database, and a stage moved elsewhere left a tombstone at home whose
state never changed — so the home scheduler would have waited on it for ever,
while the far device, holding that one stage's roadmap row, could have started
its siblings. This was true of a stage moved by hand before anything here, and
is fixed as a root of the roadmap.

**Home drives, and the stage reports back.** The device the roadmap's
Conversation is on makes every stage and keeps a tombstone for each one moved
away; the live copy tells home every lifecycle move, and home writes it onto
the tombstone, so standings and places are read where they always were. **The
far device's scheduler ignores a stage whose roadmap is not its own.** A finish
asks home over the link whether it may join and what the chain's top is, and
waits, said on the Timeline, while home is unreachable. Replicating stage
records as the cluster's was turned down as heavy and as two schedulers racing
for one stage; keeping every stage at home and letting only sessions roam was
turned down because every stage would end with a move home and the agent would
carry the rule.

## What the skills say

**CI confirms; it does not discover.** Where the build, the suite or the
failing check runs only on another operating system, the session moves there
and runs it, asking for the device where it is not permitted. The addressing
skill says a failing check whose job runs on another operating system is
reproduced there, never fixed from the log alone.

## Considered and rejected

- Ticking every device by default — the tick is automatic permission.
- A per-device setting prefilling the ticks — the same permission, one step
  further from the work it is about.
- `only on windows` as a hard requirement beside the preference.
- Starting declared work where it stands with a Notice and leaving the agent to
  move later — the human is asked instead.
- A stop whose Resume row splits in two, and a Timeline card with presses, for
  the gate — a Question Set already has the badge, the phone and the tick.
- Coming home before the finish — the wrap-up must verify where it fixes.
- Placing a fix session by the failing job's name.
- Stage records replicated across the cluster; stages that never move.
