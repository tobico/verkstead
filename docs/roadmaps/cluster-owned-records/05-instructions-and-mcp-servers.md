# 05. Instructions and MCP servers

## Goal

The Instructions text and the declared MCP servers are the cluster's, on the
facility of 01, out of `config.yaml`. The Instructions are one replicated text,
newest write wins, cleared being empty. A declaration carries a uuid; its name
is still what the agent and the attached chips see, unique among the live
declarations across the cluster, and declaring a deleted name again is a new
record. Header values are copied to every device's `secrets.yaml` as Author
tokens are. Demonstrable with two Verksteads: declare on A, attach on B, the
session on B is handed the server with its headers.

## Decisions in force

From [ADR-0028](../../adr/0028-records-are-the-clusters.md); this stage
rewrites the **Instructions** and **MCP server** terms and amends
[ADR-0021](../../adr/0021-declared-mcp-servers.md)'s *kept in `config.yaml`*.

- **Both leave `config.yaml` for the database**, for the reason Authors did: a
  record other devices write is a thing Verkstead finds. Sandbox binds stay
  where they are, being paths on one machine.
- **Instructions: one text, newest wins**; a per-device text with a
  copy-to-all press was turned down. Cleared is an empty text, not a tombstone.
- **A declaration gets a uuid**; keying by name and letting a redeclaration
  beat the tombstone was turned down, so *a removal is final* holds here too.
  The name stays the identity the agent and the chips see and is never renamed.
- **What a Conversation holds is still the name**; a chip whose name has no
  live declaration says the server is gone, and attaching a name nothing live
  declares is refused. A name with two live declarations, which only
  replication can bring about, launches neither and its chip says the two
  clash until one is renamed or removed; newest-declaration-wins was turned
  down.
- **Header values are secrets**, kept in `secrets.yaml` **under the
  declaration's uuid** rather than its name, copied to every device over the
  gated route, taken away with the tombstone — so deleting `docs` and
  declaring it again never has the old removal wipe the new values, in
  whichever order a device pulls the two; fetching them at launch from a holder was
  turned down.
- **Uniqueness at the press** among live names against the whole table; a
  pair arriving by replication is kept and drawn.

## Proposed tasks (provisional)

1. **Instructions on the facility.** A singleton record; the settings page
   saves it as a write; the root build reads the table; a serve-time rewrite
   moves the `config.yaml` key once.
   - Text saved on A is in the next root built on B.

2. **Declarations on the facility.** A record per declaration with a uuid;
   `config.yaml`'s `mcp_servers:` moved once; the settings section and the
   Attach menu read the live names; uniqueness at the press.
   - A server declared on A is on B's Attach menu; deleting and redeclaring a
     name on B is a new record A sees as gone then new.

3. **Header values everywhere.** `mcp_headers` in `secrets.yaml` rekeyed from
   the name to the uuid once; the values route answers any declaration this
   device holds values for; copies written on news and taken away with the
   tombstone; a session on any device is handed them.
   - A header set on A reaches a session launched on B.
   - Deleting `docs` on A and declaring it again with a new header: B, pulling
     both at once, keeps the new value.

4. **The chips and the refusals.** *Gone* reads off no live declaration of the
   name; attaching a dead name is refused; a name with two live declarations
   is a clash on the chip and launches neither.
   - Two devices apart each declare `docs`; once they meet, a session
     attached to `docs` is handed neither and the chip says they clash.

## Re-verify at start

- Assumes 01 landed: the facility and its peer route.
- Assumes Instructions and `mcp_servers:` are still read out of `config.yaml`
  in `crates/server/src/settings.rs` and written into each root by the Built
  Root, and that header values are `secrets.yaml` entries under the name.
- Assumes `conversation_mcp_servers` in `crates/store/src/mcp_servers.rs`
  still holds the name.
- Assumes the Attach menu's *Add a server…* card still saves as the settings
  page does.
