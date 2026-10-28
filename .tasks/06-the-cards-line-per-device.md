# 06. The card's line per device

## What to build

The Profile card carries **a line for every device of the cluster** (this
device and each member, as the Devices pane lists them), saying how that device
runs the Profile:

- **bound**, with the path its binding holds, in the harness's account shape
- or ***runs from a copy***, for a device with no binding

The bindings replicate, so every device draws every device's line from what it
holds. A member not answering is drawn from its last binding, the way the rest
of its rows are. ADR-0028 turned down showing this device's binding alone, and
showing nothing per device.

The card shows no paths today, and its device badge went in task 04. These
lines are the only place a device is named on a Profile. A device's binding of
a Profile never comes from another device's paths, so no path is offered to be
edited from another device. On this device's own line, the form edits this
device's binding. The *Use this machine's own account* press on an unbound line
is stage 03's, not this one's.

Before writing any words on screen, read `docs/copy.md`, and `CONTEXT.md` for
the vocabulary.

## Acceptance criteria

- [ ] The card on A says A is bound at its path and B runs from a copy, and
      B's card says the same.
- [ ] A device linked later gains its line on every card; an unlinked one
      loses it.
- [ ] A Verkstead in no cluster draws its card as before, with no device
      lines.
