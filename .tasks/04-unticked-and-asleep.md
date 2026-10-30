# 04. Unticked and asleep

## What to build

ADR-0029's chain for a device that is both: **asked about as unticked**, and then
waited on.

Placement, finding no permitted device of the platform, asks about the unticked
ones (task 01) without dialling them — so the Set it puts is the unticked row
whether or not the device is awake. When *Move to nuc-win* is pressed:

- the device is ticked, and the move is tried;
- where the device answers, the work moves, as in task 01;
- where it does not, the Set is **locked with a line saying nuc-win was
  permitted** (task 03's lock line), and the asleep row's Set goes up in its
  place — *Run it here* still there to press — and is waited on, and settled by
  the device waking, exactly as task 03's.

## Acceptance criteria

- [ ] *Move to* on an unticked device that does not answer ticks it, locks the
      Set with the *permitted* line, and puts the asleep Set as the new wait.
- [ ] That device answering then moves the work and locks the second Set;
      *Run it here* on the second Set starts the work here instead.
