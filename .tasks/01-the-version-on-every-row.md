# 01. The version on the Device Reading and the Member row

## What to build

The **Device Reading** a device answers with gains the release it runs, read
off the binary (the workspace's `CARGO_PKG_VERSION`, the same value the Update
Notice compares against), and every Devices row draws it under the name — this
device's row as well as every member's.

- **On the wire:** `DeviceIdentity` gains an optional `version`, modelled on
  `kept_by`: `#[serde(default)]`, `None` being what a Verkstead from before this
  stage answers by saying nothing. An older member ignores the field it does not
  know, so nothing else changes on either end. Regenerate the viewer types.
- **In the store:** the member row stores it, through `ALTER TABLE` beside the
  other optional member columns (no migration machinery), and `Linking`
  carries it. Unlike `kept_by`, a recording **overwrites** it every time, `None`
  included: it is a reading, and a member downgraded to an older Verkstead
  really does answer none.
- **Every path that records a member carries it.** Today a member's identity is
  written off the exchange (dial back), the announcement, the join and the
  Annex join (`Peers::identity`, only called there) — check for any others.
  Missing one leaves a row with no version for a member that has one.
- **In the viewer:** the Devices section's `Row` draws one quiet line,
  *Verkstead 0.2.2*, under the name, as the Annex's Components row draws its
  own. No line where the version is `None`. *This device* is drawn off
  `DevicesView.this`, which is the same `DeviceIdentity`, so it comes along.

Copy follows `docs/copy.md`; names follow `CONTEXT.md`. Amend the **Device
Reading** entry in `CONTEXT.md`: it now says the release too, read off the
binary rather than the machine.

## Acceptance criteria

- [ ] A member one release behind shows that release on its row; a member that
      sends no version (a fixture answering without the field) shows no version
      line and is otherwise drawn as before.
- [ ] *This device*'s row shows this binary's version.
- [ ] A suite proves the version round-trips: answered by one device, recorded
      by the other at a join, read back in its Devices view; and an identity
      without the field deserializes to `None`.
- [ ] A database made before this task opens and gains the column.
