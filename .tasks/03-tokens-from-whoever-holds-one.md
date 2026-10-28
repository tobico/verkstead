# 03. Tokens from whoever holds one

## What to build

An Author's GitHub token reaches every device with no home to fetch it from.

**The token route answers for any uuid this device holds a token for** —
`GET /api/peer/v1/authors/{uuid}/token`, which today answers only for the
device's own `config.yaml` Authors and `404`s a mirror — its own copy
included. `null` where it holds none, `404` where it holds no record of that
uuid.

**A token set or cleared is a write to the record**: `holds_token` changes and
the record is stamped, which is what tells the other devices to fetch. On a
pull that brings a record holding a token this device has no copy of — or a
newer stamp than its copy was fetched under — this device asks reachable
members for it, whichever holds one, and writes the copy into its own
`secrets.yaml` under the uuid as today. A record whose `holds_token` went
false, or that carries a deletion stamp, takes the copy away. A member not
answering leaves the copy as it stands, and it is used (ADR-0025).

The *token awaited* refusal stays for the window before a copy lands, but no
longer names a home device.

## Acceptance criteria

- [ ] A token set on B for an Author A made reaches A's `secrets.yaml` with A's maker — any third device — off.
- [ ] A token cleared on one device is taken out of every reachable device's `secrets.yaml` after the pull.
- [ ] A removed Author's token copy is gone from every device.
- [ ] A Repo on A pointed at that Author starts a session with the token in its environment once the copy has landed.
