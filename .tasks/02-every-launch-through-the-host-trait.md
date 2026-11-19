# 02. Every launch through the host trait

## What to build

Cut the runner over to the **host side** of the `activity` world in one move.
The host functions task 01 wrote become the Rust trait `bindgen!` generates for
the world's imports (async). One in-process implementation of it, per
Conversation, sits over what the server does today. Every driver in `runner.rs`,
and anything else that launches a session for a Conversation, launches and waits
through that trait and nothing else.

This is the host side only (Set 1803, Q2). The uncrossed drivers — the backlog
and stage runners, the wrap-up, the grilling and its tail, the inline run,
instruction steers, investigating — are **not** re-cast as in-process Activities.
Stages 09 and 10 rewrite them as components, and they call the same trait then.

- **The private `Prompt` enum goes**, and so does `launch`'s match over it. Its
  skill-and-role mapping becomes arguments to *launch a session*:
  `verkstead/<skill>`, the role (`Prompt::role`'s rule: Review for the review,
  Implementation for the rest, with `under`'s fallback kept), and the prompt text.
  - **The prompt is the caller's** (Set 1803, Q5). Each driver builds its opening
    and sections with the same `skills.rs` builder it uses today and passes the
    text.
  - The host appends what every session gets: memos (with the retiring rule that
    `Prompt::retires_memos` and `memos::Building` hold, which becomes an argument),
    previews, packs, companions, transfer, naming and the rest of what `launch`
    adds after the builder.
  - The words reaching every session must not change.
- **The evidence is an argument too.** What `Signals::expecting` registers today
  in each driver is what the launch hands the host. *Wait* returns what the
  driver selects over today: the session ended (and how), signalled and idle,
  stalled into the rescue, steered.
  - `launch_in_turn`'s Turn and stop check stay inside the host function.
  - The Driving registration rules stay as each driver holds them.
- **Git facts, notices and stop** are implemented here too, over what the
  runner reads and writes now: `own_branch_holds_commits`,
  `wrapping::committed_in`, `left_open` plus `stop`. Nothing calls them through
  the trait yet except where a driver naturally can.

## Acceptance criteria

- [ ] No session is launched for a Conversation except through the host trait's *launch a session*, and the `Prompt` enum is gone.
- [ ] Every prompt a session receives is word-for-word what it was: the skills.rs prompt tests and the sessions suite pass unchanged.
- [ ] The sessions suite is green at CI's default thread count.
