# Cluster-owned records roadmap

Agent Profiles, Authors, declared MCP servers and the Instructions text stop
belonging to the device that made them and become the cluster's: one record
per thing, held by every device, replicated with a uuid, a timestamp and a
deletion stamp. What forced it was a four-device cluster holding four unnamed
Claude Profiles and four Authors with one human's name, every picker listing
all of them and a Transfer refused because two alike Authors were two uuids.
The decisions and their why are in
[ADR-0028](../../adr/0028-records-are-the-clusters.md), which amends ADR-0020
*Shared Profiles*, ADR-0021 and ADR-0025; [CONTEXT.md](../../../CONTEXT.md)
carries a pointer on each of the six terms it changes — **Author**, **Agent
Profile**, **Account mirror**, **Memory sync**, **Instructions** and **MCP
server** — and the stage that makes each true rewrites the term.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

01 is the facility with Authors as its first record kind, and everything
stands on it. 02 moves Profiles onto it and is the big one: every reference
becomes the uuid and the account paths become a per-device binding, with an
unbound device still fetching its login from a bound one, the way it fetched
from the home before — a stepping stone the human chose over folding 03 in.
03 replaces that fetch with the replicated login and stands on 02. 04 rewrites
the memory sync from home-and-back to union-and-fan-out and stands on 02 alone,
so 03 and 04 may land in either order. 05 puts Instructions and MCP servers on
the facility and stands on 01 only; it can land any time after it. The
cleanup helpers — *Repoint them all to…* and the hostname naming at the switch
— sit inside 01 and 02 rather than in a stage of their own, which the human
chose too.

## Stages

- [x] 01: Records are the cluster's — Authors first — [brief](01-records-are-the-clusters.md) — no dependencies
- [ ] 02: Profiles by uuid, bound per device — [brief](02-profiles-by-uuid-bound-per-device.md) — after 01
- [ ] 03: The replicated login — [brief](03-the-replicated-login.md) — after 02
- [ ] 04: Memory unioned in and fanned out — [brief](04-memory-unioned-in-and-fanned-out.md) — after 02
- [ ] 05: Instructions and MCP servers — [brief](05-instructions-and-mcp-servers.md) — after 01
