# Instructions and MCP servers

The Instructions text and the declared MCP servers become the cluster's, on the
Replicated record facility stage 01 built, and leave `config.yaml`. The
Instructions are one replicated text, newest write wins, cleared being an empty
text. A declaration carries a uuid; its name is still what the agent and the
attached chips see, unique among the live declarations across the cluster, and
declaring a deleted name again is a new record. Header values are kept in
`secrets.yaml` under the declaration's uuid and copied to every device as Author
tokens are.

Demonstrable with two Verksteads: declare on A, attach on B, and the session on
B is handed the server with its headers. A name that replication leaves with two
live declarations launches neither, and its chip says they clash until one is
removed ([ADR-0028](../docs/adr/0028-records-are-the-clusters.md), amending
[ADR-0021](../docs/adr/0021-declared-mcp-servers.md)).

Roadmap stage: [05: Instructions and MCP servers](docs/roadmaps/cluster-owned-records/05-instructions-and-mcp-servers.md)

## Tasks

- [x] 01: Instructions on the facility — [details](01-instructions-on-the-facility.md)
- [x] 02: Declarations on the facility — [details](02-declarations-on-the-facility.md)
- [x] 03: Header values everywhere — [details](03-header-values-everywhere.md)
- [x] 04: Clashes, the chips and the refusals — [details](04-clashes-the-chips-and-the-refusals.md)
