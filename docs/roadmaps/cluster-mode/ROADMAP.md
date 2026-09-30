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

Stages 01, 02 and 03 are in order. 04 stands on 02. 05 stands on nothing here
and can land any time before 06. 06 stands on 04 and 05. 07 and 08 both stand on
04 and are reorderable with each other. 09 stands on 06, 07 and 08 — and so on
05, through 06. 10 and 11 both stand on 09 and are reorderable with each other.

What 09 stands on is easy to miss, so: it draws the live copy of a transferred
Conversation in **06**'s merged list, it carries a rank on the copy it makes,
which is **05**'s through 06, it builds its Transfer dialog from the device
select **07** exports, and it needs the Repo matching and mirror Profiles of
**08**. 11's ticks sit in that same select's panel, which it reaches through 09.

## Stages

- [x] 01: Device identity and the peer listener — [brief](01-device-identity-and-the-peer-listener.md)
- [x] 02: Linking — [brief](02-linking.md)
- [x] 03: Discovery — [brief](03-discovery.md)
- [x] 04: The relay — [brief](04-the-relay.md)
- [x] 05: Ranks — [brief](05-ranks.md)
- [x] 06: The merged list — [brief](06-the-merged-list.md)
- [x] 07: Drafting on a device — [brief](07-drafting-on-a-device.md)
- [x] 08: Shared Profiles — [brief](08-shared-profiles.md)
- [x] 09: Transfer — [brief](09-transfer.md)
- [x] 10: Resuming the harness — [brief](10-resuming-the-harness.md)
- [x] 11: The agent's call — [brief](11-the-agents-call.md)

## Left to do, from a rebase rather than from a stage

**A compose page cannot point at a pull request.** Stage 07 gave it a *Wrap up a
pull request* row — a pull request loaded into the box, drafted onto whichever
device would do the work — and `main` retired that level while this roadmap was
in flight: the Review Process reaches the same work through the **Target** field
instead. So the row, the band it drew over the box and the suite behind it went
with the level when the stack was rebased, and nothing replaced them.

What is missing is only the *loading*. A draft can still be pointed at a pull
request by typing into the Target field, and the device select above it still
says where the work will be done — so this is a way in that has gone rather than
a thing that cannot be done. Whoever picks it up decides whether the compose
page gets a row that fills the Target field from a list of open pull requests,
and whether that list is worth a `gh` call per registered Repo again.
