# 03. The token copy

## What to build

A copy of a mirrored Author's token sits in each device's own `secrets.yaml`
under the uuid, so a Repo pointed at a mirror runs from it even when the home
device is quiet (ADR-0025: fetching at the moment of need was turned down).

- **A member hands a token over on a call of its own** on the Peer Listener,
  gated to members, in the shape of `/api/peer/v1/profiles/{profile}/account` —
  over the mutually authenticated link the account mirror's login already
  travels. It hands over **only its own Authors' tokens**: a uuid that is a
  mirror there, or unknown, is refused, as the account call refuses a mirror
  row; an Author with no token answers that it has none.
- **The asking device writes the copy as the mirror refreshes**: for each mirror
  whose home says a token is held, fetch it and write it under the uuid with
  the per-secret `with_` builder, the rest of `secrets.yaml` left as it was.
  The copy is **taken away** when the mirror goes, when its device is unlinked,
  and when the home stops holding one. A home not answering leaves the copy as
  it stands, and it is used.
- **Task 02's refusal lifts** once the copy is held: a session on a Repo pointed
  at the mirror gets that Author's token as `GH_TOKEN` and its name and email,
  and Verkstead's own git and `gh` on that Repo authenticate with the copy.
- **Nothing else crosses into the sandbox.** The member holds a copy of every
  home Author's token, but a session carries only its Repo's Author's.

Keep `CONTEXT.md`'s **Author** term and ADR-0025's account of the copy current
with what lands.

## Acceptance criteria

- [ ] A session on a member device under a mirrored Author carries that
      Author's token and name, and none of the home device's other Authors'
      tokens is anywhere in that session's sandbox.
- [ ] A token rotated at home is the one a session gets after the next
      refresh; with the home not answering, the copy held is used as it stands.
- [ ] The token call refuses a non-member, a uuid that is a mirror on the
      answering device, and an unknown uuid.
- [ ] Removing the Author at home, clearing its token at home, or unlinking the
      device takes the copy out of the member's `secrets.yaml`, leaving every
      other secret in it as it was.
