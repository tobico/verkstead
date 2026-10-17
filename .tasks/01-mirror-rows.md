# 01. Mirror rows and seeing the one list

## What to build

Each device of a cluster learns the other devices' Authors and draws them
beside its own, as one list, the way Profile mirrors already work (ADR-0020
*Shared Profiles*, ADR-0025, the brief's *Decisions in force*).

- **A member answers its Authors** on a slim viewer route, `GET /api/ui/authors`,
  served to members through the viewer namespace the Peer Listener already
  mounts behind the member gate — the shape the Profile refresh reads
  `/api/ui/profiles` in (decided in planning, over a `/api/peer/v1/` route).
  It lists the one list: this device's own Authors with no device mark, and its
  mirrors marked with their home device. Each row carries the uuid, the display
  name, the git name, the email and **whether a token is held** — never the
  token, nor its last four.
- **Each device writes mirror rows** in its database, keyed by the uuid (the same
  everywhere, so there is no local id), holding those fields and the home
  device. Only rows the member answers with no device mark are taken, so a
  mirror is one hop from its home, never a mirror of a mirror.
- **The refresh runs on the Profiles' schedule**: at start, whenever a member
  says its Authors moved (a nudge of its own, announced when this device's
  Authors are saved or removed, and taken up on `Everything` as Profiles are),
  and whenever the membership moves. A member not answering keeps the rows it
  last gave; an Author removed at home is gone from what that device answers
  and its mirror goes on the next refresh; a device unlinked takes its mirrors
  with it. The post-refresh announcement stays local, as the Profiles' does.
- **The viewer draws the one list.** The Git section's switcher lists mirrors
  beside local Authors, each mirror carrying the device mark (the OS icon and
  device name, as a mirrored Profile's row does). A mirror picked in the
  switcher is drawn **read-only** with a line saying which device it is at
  home on — editing and removal arrive in task 04 (decided in planning). The
  Repos pane row and the Brief's Author fact name a mirror's display name the
  way they name a local one. The "drawn only while more than one Author
  exists" rule counts mirrors.

Nothing here lets a Repo point at a mirror yet — task 02 does — and no token
crosses — task 03 does. Keep `CONTEXT.md`'s **Author** term current with what
lands (the mirror row and where it lives).

## Acceptance criteria

- [ ] Linking two devices draws each one's Authors in the other's Git switcher,
      marked with the device they are at home on, and a mirror picked there is
      read-only and says where it is at home.
- [ ] `GET /api/ui/authors` answers no token or part of one, and a device that
      learns of another's mirror through it does not mirror it again.
- [ ] Removing an Author at home takes its mirror off the other device on the
      next refresh; unlinking the device takes all its mirrors; a member that
      is not answering keeps the rows it last gave.
- [ ] Saving an Author at home redraws the mirror on the other device without
      a restart.
