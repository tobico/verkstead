# 04. Clashes, the chips and the refusals

## What to build

What a Conversation holds is still the **name** it attached. This task makes
every reading of a name honest about what is live behind it.

**Each attached name reads one of three ways** off the live records:

- **one live declaration**: launched with it;
- **none**: the chip says the server is *gone*, as today, and the launch
  leaves it out;
- **two or more**, which only replication can bring about, uniqueness being
  enforced at the press: **the launch hands the session neither**, and the chip
  says the two **clash, until one is removed**. The human chose this wording: a
  server is never renamed (ADR-0021), so removing one is the only way out.
  Newest-declaration-wins was turned down.

**Attaching a name is refused** where nothing live declares it, as today, and
where it clashes too.

**The settings section draws both rows of a clashing pair**, each with its own
Remove, addressed by uuid. It says they share a name and that one has to go.
Removing either one, on any device, clears the clash everywhere once the
removal replicates.

The prompt's `# MCP servers` listing names only what the session was actually
handed.

Rewrite the **MCP server** term in `CONTEXT.md` in full: a replicated record
with a uuid in the database, the name still the identity the agent and the chips
see, unique among live declarations across the cluster, a deleted name declared
again being a new record, header values in `secrets.yaml` under the uuid copied
to every device, and a clashing name launching neither. Drop its *Becoming the
cluster's* pointer. Read `docs/copy.md` for every string.

## Acceptance criteria

- [ ] Two Verksteads apart each declare `docs`; once they meet, a session attached to `docs` on either is handed neither, and its chip says the two clash.
- [ ] Attaching `docs` while it clashes is refused, and so is attaching a name nothing live declares.
- [ ] The settings section draws both `docs` rows; removing one on either device clears the clash on both, and the next session gets the survivor.
- [ ] A chip whose name has no live declaration says the server is gone.
- [ ] The **MCP server** term in `CONTEXT.md` is rewritten and its pointer gone.
