# 03. A sleeping device, and its wake

## What to build

The asleep case, and the device settling it by answering.

**Telling asleep apart.** Stage 03's `Unplaced::NoneCanTake` lumps together a
permitted device that did not answer its preflight (`Lacking::Unreachable`) and
one that answered lacking the Repo, a companion or the harness. Only the first
is asked about. A permitted device of the platform that is awake and lacking
stays the Notice naming what it lacks, and the work runs here (Set 1547, Q4).

**The asleep Set.** Where a permitted device of the platform is asleep and none
is clean, put Verkstead's own Set with the ADR's asleep row — *Task 03 wants
Windows. nuc-win is not awake, and the work moves there by itself once it is.* —
and *Run it here* as its only Option. No Option means *wait*: the open Set is
the wait (task 02). *Run it here* launches the work here.

**Asleep and unticked together** (Set 1547, Q3): a ticked device asleep while
another of the platform is linked, awake and unticked puts one Set saying both —
the sleeping one moving by itself once awake, the other not permitted — with
*Move to <unticked device>* · *Run it here*.

**The wake.** Nothing in the code says *a device woke*: `reachable` on a member
row is flipped by several paths (recording a member, a dial getting through) and
only when something dials. So the wait asks the device itself on a paced clock
(the server's `Pace`), and a member turning reachable hurries it (Set 1547, Q5).
When the sleeping device's preflight comes back clean while the Set is open, the
work moves there by the ordinary Transfer — the tick already was the consent —
and the Set is locked.

**A lock that says why.** A lock carries no line today, and the sheet says the
Set *was locked unanswered … and the agent was told it was locked*, which is
wrong for Verkstead's Set. Give a lock a line of its own, kept beside the lock
the way the asker is kept beside the Set, and draw it on the sheet and the
Timeline in place of that sentence: here, that nuc-win answered and the work was
moved there. A human's lock of an agent's Set reads as it does today.

## Acceptance criteria

- [ ] A Windows task with a ticked Windows device that does not answer puts the
      asleep Set with *Run it here* alone; *Run it here* starts the task here.
- [ ] The sleeping device answering while the Set is open moves the work there
      and locks the Set, the sheet and the Timeline saying the device answered.
- [ ] A ticked device awake but lacking the Repo is still the Notice; ticked
      asleep plus another unticked awake puts the combined Set.
