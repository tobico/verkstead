# 03. The member coming back

## What to build

The second half of a member's Update, held by the pressing device: from
*updating…* to *restarting…* to the row showing the new version, or to a
trouble line.

- **restarting…** once the member's Nudge stream drops while its Update reads
  *updating…*. That drop is the member's app quitting to install (stage 04's
  `quitAndInstall`). Unlike this device's own row, where *restarting…* is the
  viewer's word, here the pressing device's server is still up, so the state
  is held there and drawn from the Devices view.
- **Back**: when the stream is taken up again, stage 01's take-up reads the
  member's identity again. If the version is now the one the press carried (or
  any version higher than the one it ran at the press), the record ends and the
  row shows its new version with nothing else.
- **Trouble**, each standing on the row until the next press:
  - it came back **on the version it ran**: *crow couldn't be updated: it came
    back on 0.2.2.*, naming the versions as stage 01's gap sentence does. The
    member's own relaunch check (stage 04's app reporting another version than
    the one it handed over to) may also have set its trouble line; read it with
    task 01's `GET` and say one line, not two;
  - it **did not come back within 10 minutes** of the press (Set 1416): a line
    saying crow didn't come back after its update. If it turns up later on the
    new version, the row shows that version and drops the line.
- What ends a record (back, trouble or the deadline) is what task 04 waits on,
  so give it one place to be awaited.

Suite: the member's stand-in app restarts the member on a new release
(`Reading::now_running` changes a running member's release), on the same
release, or not at all, with the deadline shortened by a test-only setting.

## Acceptance criteria

- [ ] With crow's stream dropped after the press, crow's row reads *restarting…*; taken up again on 0.3.0, the row shows 0.3.0 and no state word or line.
- [ ] Taken up again on 0.2.2, the row carries one trouble line naming both versions.
- [ ] A member that never comes back gets a trouble line at the deadline.
