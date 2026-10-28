# 03. Header values everywhere

## What to build

An MCP declaration's header values reach every device, as an Author's token
does since stage 01 (read the Authors kind's `copied`, `wanted` and `fetched`,
and the token route). There is no home to fetch from: they travel from whoever
holds them.

**The record says which values are held, never what they are.** It carries a
digest of each header's value, the way an Author record carries
`token_digest`. Setting, rotating or clearing a value is therefore a write to
the record, which is what tells the other devices to fetch.

**A new gated peer route**, behind the Member Gate beside the token route,
answers the values this device holds for a declaration's uuid: a `404` where it
holds no record of that uuid, and nothing where it holds no values. There is no
such route today.

**After every pull**, a device whose `secrets.yaml` holds no copy, or a copy
whose digest is not the record's, asks its members in turn for the values,
writer first, and keeps the first answer whose digests match. It writes the
copy under the uuid. A record whose values were cleared, or that carries a
deletion stamp, takes the copy away. A member not answering leaves the copy as
it stands, and it is used. Because the values are keyed by uuid, deleting
`docs` and declaring it again never has the old removal wipe the new values,
whichever order a device pulls the two in.

**The Keeper stops handing `mcp_headers` to its Annex** (its `secrets.yaml`
`working_as` copy). The Annex now fetches them as a member. Fix the Annex
term in `CONTEXT.md` to match.

Values are never returned to a page, a view or a log, and never put on the
record (ADR-0021).

## Acceptance criteria

- [ ] Two Verksteads: a header set on A for a server declared on A reaches a session launched on B, attached there.
- [ ] Rotating the value on B reaches A's `secrets.yaml` after the pull, and A's next session sends the new one.
- [ ] Delete `docs` on A and declare it again with a new header, then let B pull both at once: B keeps the new value, and the old uuid's copy is gone.
- [ ] A removed declaration's values are gone from every device's `secrets.yaml`.
- [ ] The values reach C through B when A, which set them, is off.
- [ ] The Keeper no longer writes `mcp_headers` into its Annex, and the Annex's sessions still get them.
