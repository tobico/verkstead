# 02. Declarations on the facility

## What to build

Every declared **MCP server** becomes a Replicated record (ADR-0028) on stage
01's facility, one per declaration, keyed by a **uuid**, out of `config.yaml`.
Its fields are the name, the URL and the header *names*. The values are still
secrets and stay in `secrets.yaml`; copying them across devices is task 03.

**The name stays the identity the agent and the chips see**, and a server is
still never renamed (ADR-0021). But deleting a declaration is a deletion stamp,
and **declaring a deleted name again mints a new uuid**: a new record, which is
what keeps *a removal is final* intact.

**Taken up from `config.yaml`'s `mcp_servers:` once, at serve time**, as
Instructions were in task 01. Decided with the human:

- **At the move only, a declaration's uuid is derived from its name**, a
  name-based (v5) uuid under a fixed namespace. So `docs` declared by hand on
  two devices becomes one record rather than a clash on every chip.
- It is **stamped older than any edit**, the device id deciding between two
  devices' taken-up copies, as for the Instructions.
- A name removed after the move stays removed when a late device takes up its
  own copy, because the merge keeps the deletion stamp. Every declaration made
  after the move gets a fresh random uuid.
- **An Annex takes nothing up** (it drops the key), and **the Keeper stops
  handing `mcp_servers:` in**. The Annex gets the declarations as a member.

**The header values are rekeyed from the name to the uuid in the same move.**
`mcp_headers` in `secrets.yaml` moves once from `name → header → value` to
`uuid → header → value`, so that even on one device, deleting and redeclaring
`docs` never inherits the old values. The Keeper still hands its
`mcp_headers` to its Annex wholesale until task 03. Under uuids, those keys
match the records the Annex pulls, so its sessions keep their headers through
this task.

**Everything reads the live records.** That means the settings section, the
Attach menu (and its *Add a server…* card, which saves through the settings
save), whether a chip is still *declared*, the refusal of an attach to a name
nothing live declares, and the launch.

The settings save maps what the page sends onto records:

- a new row is a new record;
- a URL or header-name edit is an edit of that record;
- a row gone is a deletion stamp, and its values leave `secrets.yaml`.

The page carries each row's uuid so that a save addresses records rather than
names. **Name uniqueness is enforced at the press** against every live record
this device holds, whoever wrote it. A pair arriving by replication is kept;
until task 04 draws it, the launch takes the first.

The kind gets its own *moved* Nudge, and a merge redraws the settings and the
Attach menu. Amend ADR-0021's *kept in `config.yaml`* passages with a pointer
to ADR-0028. Read `docs/copy.md` before touching any string.

## Acceptance criteria

- [ ] Two Verksteads: a server declared on A is on B's Attach menu, and a session on B attached to it is launched with it (headers aside, which is task 03).
- [ ] Deleting `docs` on B and declaring it again gives a new uuid; A sees the old record removed and the new one live.
- [ ] An installation with `mcp_servers:` and `mcp_headers` starts with the same servers out of the database, under name-derived uuids, with its header values rekeyed under them; a session attached to `docs` still gets its headers.
- [ ] `docs` declared on two devices before the upgrade is one record after it, with no clash.
- [ ] Declaring a name that a live record from another device already has is refused at the press.
- [ ] An Annex takes nothing up, and its sessions still get the declarations and their headers.
- [ ] ADR-0021 carries the pointer to ADR-0028.
