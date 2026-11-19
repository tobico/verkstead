# 06. The wrap-up's frame

## What to build

**The wrap-up**, one component in the built-in Activities plugin with its
watchers declared as options — `checks`, `comments`, `review`, `mergeable` —
running concurrently inside it on task 05's event loop and reaching GitHub
only through the `forge` host function. This task builds the frame and the
mergeable watcher; tasks 07 and 08 move the other three in, the in-process
watchers running beside the component until then.

- **Started at the moment a pull request is recorded**: `wrapping.rs` still
  moves a Conversation into Wrapping on recording one (`store/pull_requests.rs`
  `record`, and every door into it — a backlog's finish, a roadmap, a take-up,
  a Tinker's ending); that moment starts the Activity until stage 11.
- **Opens the pull request where none stands**, launching a session on
  `verkstead/submitting` itself with the evidence `Landed` (one Activity never
  calls another), as the wrap-up's *no pull request* entry does today.
- Covers companions' pull requests and a walked stack (`covering`,
  `stacks.rs`), as now.
- **Mergeable and landing** (`merges.rs`, `merging.rs`): outcomes `merged` and
  `closed`, plus `steered`.
- **`given-up`** (Set 1838, Q4): the wrap-up's stops today — fix attempts
  exhausted, a companion committed in and left without a pull request — answer
  `given-up` instead of stopping, and the host maps it to what the stop did, so
  nothing on the workbench moves and a Process can map it in stage 11.
- Stateless across restarts: started again over the Timeline and the host's
  wrap-up records, it carries on.

## Acceptance criteria

- [ ] The merge and close tests in `crates/server/tests` pass unchanged, with the Conversation's ending fired by the component's outcome
- [ ] A take-up over a bare branch opens its pull request through the component's `submitting` session and carries on wrapping up
- [ ] What stopped a wrap-up before answers `given-up`, and the Conversation lands where the stop left it
- [ ] A server restarted mid-wrap-up starts the component again and it carries on
- [ ] The Plugins page's Activities row names *Wrap-up*
