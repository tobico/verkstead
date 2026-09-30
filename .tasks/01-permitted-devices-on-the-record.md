# 01. Permitted devices on the record

## What to build

The human's consent to the agent moving the work, written down per
Conversation (ADR-0020, *The agent's call*: **the ticks are the consent**, and
no confirmation follows them later).

- **A table per Conversation** of the devices of the cluster the work may be
  transferred to, by **Device Id**. The **drafting device** — the device in the
  Conversation's **Birth Key** — is always permitted and is never a row: it is
  implicit, so an agent that has moved can always come home. Nothing is ticked
  by default.
- **The ticks in the device select's panel**, under a heading *May be
  transferred to*: one tick per *other* device of the cluster (every member but
  the one the select is pointing at), in the `DeviceSelect` stage 07 exports
  from the workbench's Setup. Offered while drafting — on the compose page
  (held with the rest of what the composer remembers until the draft is
  created, and sent with it) and on a saved Draft's composer — and **the same
  ticks in the Transfer dialog** stage 09 draws from that select, where they
  change the list without moving anything. A cluster of one draws no ticks, the
  way it draws no select.
- **A device that leaves the cluster drops off the ticks**: unlinking removes it
  from every Conversation's list (or the list is read against the membership —
  whichever the membership code makes natural), and a ticked device that is
  gone is neither drawn nor listed anywhere.
- **The list crosses with the record.** Stage 09's transfer carries a
  Conversation's whole record as a slice renumbered on landing; the permitted
  devices go with it, so on the far end the list is the same set of devices.
  The device the work is now on is simply not offered there (it is where the
  work is), and the device it came from is still permitted if it was ticked or
  is the drafting device.

A human-pressed transfer ignores the list entirely — that stays stage 09's
press, unchanged.

Add the term to `CONTEXT.md` beside *Transfer* and *Birth Key*.

## Acceptance criteria

- [ ] Ticking a device in the select's panel on a draft, and in the Transfer
      dialog of a running Conversation, is stored on the server and drawn again
      on reload; the drafting device is never offered as a tick and always
      reads as permitted.
- [ ] Unlinking a member takes it off every Conversation's permitted list.
- [ ] A Conversation transferred to another device (two-server transfer suite)
      arrives with the same permitted devices, the drafting device still
      implicitly permitted there.
