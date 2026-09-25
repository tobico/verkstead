# Cluster mode roadmap

Several Verkstead servers on a human's machines link into a **cluster**, and
the workbench of any one of them drives all of them: every Conversation in one
sidebar, a draft started on whichever device should do the work, an Agent
Profile from one machine run on another, and a running Conversation moved
between machines — by the human, or by the agent when the work needs another
platform. The decisions and their why are in
[ADR-0020](../../adr/0020-cluster-mode.md); [CONTEXT.md](../../../CONTEXT.md)
gains its terms stage by stage as each lands.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Stages 01, 02 and 03 are in order. 04 stands on 02. 05 is independent of
everything and can land anywhere before 06. 06 stands on 04 and 05. 07 and 08
both stand on 04 and are reorderable with each other. 09 stands on 08. 10 and
11 both stand on 09 and are reorderable with each other.

## Stages

- [ ] 01: Device identity and the peer listener — [brief](01-device-identity-and-the-peer-listener.md)
- [ ] 02: Linking — [brief](02-linking.md)
- [ ] 03: Discovery — [brief](03-discovery.md)
- [ ] 04: The relay — [brief](04-the-relay.md)
- [ ] 05: Ranks — [brief](05-ranks.md)
- [ ] 06: The merged list — [brief](06-the-merged-list.md)
- [ ] 07: Drafting on a device — [brief](07-drafting-on-a-device.md)
- [ ] 08: Shared Profiles — [brief](08-shared-profiles.md)
- [ ] 09: Transfer — [brief](09-transfer.md)
- [ ] 10: Resuming the harness — [brief](10-resuming-the-harness.md)
- [ ] 11: The agent's call — [brief](11-the-agents-call.md)
