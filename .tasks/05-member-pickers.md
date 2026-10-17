# 05. Open repo and Create repo on a member offer its one list

## What to build

With a member device picked for a Draft, **Open repo** and **Create repo** offer
that device's one list of Authors — read from it as the parent-directory browse
already is, over the Relay — and record the pick there.

Today both cards read *this* device's settings whatever device is picked, so a
pick of a local Author is refused by the member as an Author it doesn't have,
and the GitHub tick follows this device's token. (Planning found Open repo has
the same gap the brief named for Create repo, so both are fixed here.)

- Both cards' Author picker reads the picked device's `GET /api/ui/authors`
  (task 01), so it lists that device's own Authors and its mirrors — which
  include this device's Authors, marked as at home here.
- The Author recorded is whichever was picked, home or mirror, resolved on the
  member against its own one list (task 02).
- Create repo's GitHub tick follows whether the picked Author holds a token
  *on that device* — its own token, or task 03's copy.
- The "drawn only where more than one exists" rule counts the picked device's
  list.

## Acceptance criteria

- [ ] Create repo with a member device picked offers every Author of the
      cluster, and the Repo made there records the one picked.
- [ ] Open repo with a member device picked does the same, and a pick of an
      Author at home on this device is no longer refused by the member.
- [ ] The GitHub tick on Create repo follows the picked Author's token on the
      picked device.
