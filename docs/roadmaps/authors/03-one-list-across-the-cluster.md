# 03. One list across the cluster

## Goal

Every device of a cluster draws one list of Authors. A Repo on a member device
can be pointed at an Author whose home is another machine, and its sessions
commit and push as that Author from a copy of the token this device holds. An
Author made or edited on any device lands on its home device; removing one
checks that device's own Repos; a Transfer onto a device whose Repo points at
another Author is refused by the preflight naming both; and Create repo on a
member offers the one list.

## Decisions in force

All from [ADR-0025](../../adr/0025-a-repo-has-an-author.md) and the **Author**
term in `CONTEXT.md`, borrowing [ADR-0020](../../adr/0020-cluster-mode.md)'s
*Shared Profiles* throughout.

- **An Author is at home on the device whose `config.yaml` and `secrets.yaml`
  hold it.** Every other device keeps a **mirror row** in its database — the
  uuid, the display name, the git name, the email, whether a token is held, and
  the home device — because a mirror is found rather than told, where
  `config.yaml` is for what Verkstead is told. The uuid is the same everywhere,
  so there is no local id and nothing that reads one.
- **Mirrors refresh device to device over the Relay**, at start, whenever a
  member says its Authors moved, and whenever the membership moves — the
  Profiles' schedule, and a member not answering keeps the rows it last gave.
  An Author removed at home is gone from what that device answers and the
  mirror goes on the next refresh; a device unlinked takes its mirrors with it.
- **A copy of the token sits in each device's own `secrets.yaml` under the
  uuid**, written as the mirror refreshes and taken away when the mirror goes
  or the home stops holding one. Fetching the token at the moment of need and
  keeping it nowhere was turned down: a quiet home should not stop a Repo here,
  and a rotation at home being stale until the next refresh is accepted. The
  copy travels over the mutually authenticated link the account mirror's login
  already travels, gated to members.
- **A mirror whose copy has not arrived refuses a Start by name**, naming the
  home device, the way a broken Profile mirror does, and a pipeline step that
  needed it halts with a Notice. Treating it as a tokenless Author was turned
  down. Once a copy is held, a quiet home means the copy is used as it stands.
- **A new Author is at home on the device whose settings page made it.**
  Editing a mirror is relayed to its home device as the ordinary edit of its
  own, token verification included, and the pane says which machine the press
  goes to; the mirror redraws off the refreshed row rather than off what was
  typed. A home that is not answering refuses the press by name.
- **Removal checks the home device's Repos only.** The home cannot see a
  member's Repos; a member's Repo left pointing at a mirror that has gone is a
  start refused by name there, under 02's vanished-Author rule. Asking every
  member first was turned down.
- **A Transfer whose far Repo points at another Author is refused by the
  Preflight**, naming both Authors, compared by uuid. Repointing the far Repo
  as part of the move was turned down; the fix is a repoint over there, under
  the live-conversation rule.
- **Create repo on a member device offers the one list**, read as the parent
  directory browse already is; the Author recorded is whichever was picked,
  home or mirror.

## Proposed tasks (provisional)

1. **Mirror rows and the member call.** A member answers its own Authors —
   without tokens — on the Peer Listener beside its Profiles and Repos; each
   device writes mirror rows keyed by uuid with the home device; the refresh
   runs on the Profiles' schedule; the settings view and every picker list
   local Authors and mirrors as one, the device mark on a mirror.
   - Linking two devices draws the other's Authors in this device's switcher
     and pickers, marked with the device they are at home on.
   - Removing an Author at home takes its mirror off the other device on the
     next refresh.

2. **The token copy.** A member hands a mirror's token over the link on a call
   of its own, gated to members; the asking device writes it under the uuid in
   its `secrets.yaml` as the mirror refreshes and takes it away with the
   mirror; a Repo pointed at a mirror runs its sessions and its server-side git
   and `gh` from the copy.
   - A session on a member device under a mirrored Author carries that Author's
     token and name, and the home device's other Authors' tokens are nowhere
     on the member.
   - A token rotated at home is the one a session gets after the next refresh.
   - A Repo pointed at a mirror whose copy has not arrived refuses a Start
     naming the home device, and a halted step leaves a Notice.

3. **Edits, verification and removal across devices.** A new Author is this
   device's own; an edit of a mirror is put to the home device as its own edit,
   the verification answered from there; Remove on a mirror is relayed home and
   checks that device's Repos; a home not answering refuses the press by name.
   - Editing a mirror's email from the member changes it at home and the mirror
     redraws from the refreshed row.
   - Removing a mirrored Author a home Repo points at is refused naming that
     Repo; one no home Repo points at is removed, and the member's Repo that
     pointed at it refuses its next Start by name.

4. **The preflight and Create repo on a member.** The Transfer preflight adds a
   finding for a far Repo pointed at another Author, naming both; Create repo on
   a member device offers the one list and records the pick there.
   - A Transfer onto a device whose matching Repo points at another Author is
     refused with both names, and repointing that Repo over there clears it.
   - Create repo with a member device picked offers every Author and the Repo
     made there records the one picked.

## Re-verify at start

- Assumes 01 and 02 landed: Authors keyed by uuid in the files, the Repo
  association in the store, the switcher, the pickers, the refusals, and every
  server-side call reading by the Repo's Author.
- Assumes Profile mirrors are still rows of the `profiles` table marked with a
  home device, refreshed by `crates/server/src/mirroring.rs` over the Relay,
  and that the account mirror's login still travels on
  `/api/peer/v1/profiles/{profile}/account` gated to members — the shapes the
  Author mirror and the token call copy.
- Assumes `secrets.yaml` is still written whole with a `with_` per secret, so
  a mirror refresh writing one token leaves the rest as they were.
- Assumes the preflight's findings are still the `Lacking` variants in the
  render crate's transfer module, read on the asking device off what the far
  end answers, and that the Repo match is still by origin URL.
- Assumes the Create repo card still remembers a parent per device and browses
  the picked device's home.
