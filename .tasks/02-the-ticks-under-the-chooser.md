# 02. The ticks under the chooser

## What to build

On a clustered Verkstead, a proposal that wants a platform draws the *May be
transferred to* ticks under the three directions, and a tick there is the
same tick the Attach menu's device rows make (ADR-0028), recorded on the Set
and applied at Submit exactly as an addition is.

**What the server hands the chooser.** The Attach menu's device list
(`addable`) leaves out the devices already permitted and the one the work is
on, so it cannot tell *no Windows device linked* from *the Windows device is
permitted already*. The proposal's view therefore gains, per want, the
cluster's devices of that platform, each read through `platform::os_mark`
from its member's OS word (never matched as a string: *Linux (WSL)* is a
Linux device), each with its standing: tickable, permitted already (ticked
or the drafting device), or the device the work is on. And whether this
Verkstead is in a cluster at all.

**What the chooser draws**, only where the proposal wants something:

- In no cluster: task 01's line of prose and no ticks.
- In a cluster: a *May be transferred to* group of checkboxes, devices of a
  wanted platform first with that want's reason beside them, then the
  cluster's other tickable devices.
- A wanted platform with no device in the cluster: a line saying so — the
  work will run where it is — and nothing to tick for it.
- A wanted platform whose device is permitted already, or is where the work
  is: a line naming it (*nuc-win may already be transferred to*, *the work is
  on nuc-win*), and no tick.

**Recording.** A tick is an addition picked under no label — the comment
box's — through the same add-to-Set call the Attach menu uses; unticking
takes that addition off again. The comment box's chip row leaves out the
device additions the chooser draws, so the same device is never shown twice;
a device picked at the comment box's menu shows as ticked on the chooser.
Refusals (`NotAMember`, `WorkIsHere`, …) are said under the chooser in the
words the menu already uses.

**Applying.** Nothing new: the Submit applies the Set's pending additions,
so the device becomes permitted, and the Response's set-level `added` names
it. The answered record shows the picked direction and the device added.

Read `docs/copy.md` before writing any line here.

## Acceptance criteria

- [ ] A server suite test: a Set wanting `windows` on a cluster with a Windows
      member — ticking it and picking `roadmap` in one Submit permits the
      device, and the Response carries `direction: roadmap` and an `added`
      naming the device.
- [ ] Viewer tests: wanted devices drawn first with the reason; the no-device
      line with nothing to tick; the already-permitted / work-is-here line; no
      ticks and only the prose line in no cluster; a tick drawn on the chooser
      and not again as a comment-box chip.
- [ ] A proposal with no `wants` draws the chooser exactly as before.
