# Linking

Two Verksteads become one cluster, and a third joins through either. On A, the
**Devices** section of Remote access grows an **Add**: type B's address, and A's
list shows a pending row, *Waiting for confirmation on B*, with A's own
fingerprint and Cancel. On B, every open workbench raises a modal — A's name, OS,
address and fingerprint, Allow and Deny — and B's phones get a push. Allow
exchanges certificates, B hands A every member and announces A to each of them
over the link it already holds, with no further press anywhere; both lists then
read the same. **Unlink** drops a device from the cluster for everyone, asked
once. A member that stops answering stays on the list, dimmed *unreachable*.

This is where a **Member** first exists. Stage 01 built what a device *is* and
the listener it is reached on, and left the membership as a stub — a count, with
*is this caller a member* answering no to everybody. This stage gives it a store
table to consult, the join that writes a row in it, and the announcement that
writes the same row on every other device in the cluster; and it finishes stage
01's **Changeover**, which has been completing at the start that began it for
want of anybody to announce a new fingerprint to. It is also where this tree
first *dials* a peer at all: stage 01 built the listener only. Demonstrable end
to end with three servers.

Roadmap stage: [02: Linking](docs/roadmaps/cluster-mode/02-linking.md)

## Tasks

- [ ] 01: The member store and the gate — [details](01-the-member-store-and-the-gate.md)
- [ ] 02: Dialling a peer — [details](02-dialling-a-peer.md)
- [ ] 03: Add, the join post and the pending row — [details](03-add-and-the-join-post.md)
- [ ] 04: The confirmation modal and its push — [details](04-the-confirmation-modal.md)
- [ ] 05: The exchange — [details](05-the-exchange.md)
- [ ] 06: The announcement — [details](06-the-announcement.md)
- [ ] 07: Unlink — [details](07-unlink.md)
- [ ] 08: The renewal announced — [details](08-the-renewal-announced.md)
