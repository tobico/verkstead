# 01. A member says what it is, and can be told

## What to build

A member says what it was installed as, every member's Devices row shows
whether it can be updated from here, and a member can be told to update itself
over the peer link. There is no button yet; task 02 adds it.

**What a member says.** Stage 04 put the install kind (`InstallKind`) on the
Devices view's `this_install` only, and left what a member says to this stage.
It now goes on the **Device Reading** (`DeviceIdentity`) beside `version`, as
`#[serde(default)] install: Option<InstallKind>`. It is read off the binary the
same way, so it is re-read at every Nudge stream take-up (stage 01's
`read_again` in the relaying freshness module) and stored on the member's row
the way the version is, with a store migration if the member row needs one. A
member that sends none is a Verkstead from before this stage. Regenerate the
viewer's types.

**Being told.** `/api/ui/devices` is one of the prefixes `KEPT_TO_ITSELF` in the
peer workbench router never serves to members, so stage 04's
`POST /api/ui/devices/this/update` cannot be relayed. Instead add a member's
own route on the Peer Listener, merged in `peer::workbench::served` beside
`news`, `repos` and the others, behind the Member Gate:

- `POST` — *update yourself*. It carries the release the pressing device
  expects, for the log and for task 03's comparison; the member's
  electron-updater fetches its own latest stable as stage 04 does. The member
  answers by doing exactly what its own row's press does: refuse by name with
  stage 04's `updating::refusal` (worded for a member, e.g. *crow was installed
  with nix, so it's updated by hand.*), or `Updating::begin`, which prints the
  stdout line the desktop app acts on. A second press while one is going
  reads the one already going.
- `GET` — the member's own Update as it stands: `ThisUpdating` (the percent)
  and its trouble line, so the pressing device can draw them (task 02).

Refusals follow the Annex's `running_there` model: a sentence naming the
device, in `docs/copy.md`'s voice. The progress and failure routes the desktop
app posts to stay local, under `/api/ui/devices/this/update/…`, and are never
served over the link.

**The rows** (decided in Set 1416):

- A member whose install can't update itself (nix, checkout, bare binary)
  shows stage 04's by-hand line (the `ByHand` component) worded for that
  member, not *this device*. It has no button and shows the line whether or
  not *update available* is up.
- A member that says no install kind (a release from before this stage) reads,
  when *update available* is up, *crow is on a release that can't be updated
  from here. Update it on crow.*
- **An Annex's row** (`kept_by` set) gets no button and no by-hand line,
  because its Keeper re-pins it to the Keeper's own release at every start.
  Instead it gets one line saying it follows its Keeper. Task 04's Update all
  passes it over.

Use `CONTEXT.md`'s vocabulary and `docs/copy.md`'s voice for every string.

## Acceptance criteria

- [ ] A member's Device Reading carries its install kind; a member's row in the Devices view has it after a stream take-up, and an older member's reads none.
- [ ] In the suite, telling a desktop member over the peer route makes it print stage 04's stdout line and its `GET` read *updating*; telling a nix member is refused with a sentence naming it, and nothing is printed.
- [ ] A nix member's row shows the by-hand line naming the member; an older member shows the *can't be updated from here* line; an Annex's row says it follows its Keeper; none of them has a button.
