# 08. The Activity interface, and follow-up rounds across

## Goal

A plugin may provide an **Activity**: a wasm component the server starts with
a Conversation's facts, hands events to, lends a fixed set of host functions,
and watches end in one of the **outcomes** its manifest declares. The
`activity` provides-block and its WIT world are defined here, the existing
runner code is put behind an in-process Rust trait shaped exactly like that
world, and **follow-up rounds** cross the boundary as the first built-in
component: a Conversation steered into Follow-up, or a Tinker started from a
Draft, runs its rounds under the component with the sessions suite passing
unchanged. Nothing anybody can see on the workbench moves; what is
demonstrable is a `plugin.yaml` naming an Activity, the Plugins page listing
*Follow-up rounds* as a built-in, and a session launched by a component that
reached the server over nothing but the host functions.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Activity*, *Capabilities and
  trust*.** An Activity is a component rather than a Rust type, and the
  built-in ones cross too — the human's words were that the built-ins should
  become wasm plugins *to prove out the boundary*. A host interface that only
  third parties use is one nobody exercises until the first third party does,
  and finds wanting then. So every host function below is one a built-in needs
  before it is one a stranger might.
- **What an Activity is started with**: the Repo, the branch, the base, the
  Worktree's path, the Companion Repos, the Process and its roles' Pairings,
  the Brief, and the Timeline so far as data. It is **stateless across
  restarts** (Set 1767, Q31): on a server restart the host starts it again with
  the same facts and the Timeline as it now stands, and the Activity derives
  where it was. A key-value store per Activity per Conversation was offered and
  turned down — the rule that **files in the Repo are the truth** stands, which
  is how the backlog runner already works, and a Timeline is a record of
  everything the host did on the Activity's behalf. An Activity that needs more
  memory than git and the Timeline give it is one asking the wrong question.
- **The host functions** (Set 1767, Q30, option 3 — the widest offered):
  - *launch a session* under a named skill as `<pack>/<skill>` (stage 07's
    naming) and a named role, with a prompt and the **Done-signal evidence**
    it expects — [ADR-0018](../../adr/0018-a-session-says-it-is-done.md)'s
    `Evidence` generalised: what the Activity registers is what `verkstead
    done` is checked against, the way `Signals::expecting` registers it per
    driver today;
  - *wait on its Done signal*, and be told the session stopped, stalled or
    was steered instead;
  - *read the Worktree's git facts* — commits past the base, the branch's
    name, whether anything is uncommitted — read-only;
  - *call the Forge*, which at this stage is a host function wrapping
    `github.rs` and from stage 12 the Forge interface;
  - *put a Question Set of Verkstead's own* and read its Response, the way
    the server puts its own Sets today (`own_sets.rs`);
  - *put a Timeline notice*;
  - the **Nothing-else** mark reaches it as an event, as does a steer;
  - and **WASI capabilities the manifest asks for** — the Worktree's files as
    granted, outbound hosts as granted — shown and consented to at install as
    ADR-0030 *Capabilities and trust* says.
- **An Activity ends in an outcome it declared.** Outcomes are the manifest's
  own list of words; a Process (stage 11) maps each to a state, and
  `verkstead plugin check` refuses a Process with one unmapped. An outcome is
  the only way an Activity moves a Conversation — it never names a state,
  because states are the Process's.
- **Follow-up rounds cross first** because it is the smallest Activity that
  still exercises everything: one session relaunched round after round, one
  mark, a reading of the branch at the end — commits past the base against
  none, which is how Tinker lands Wrapping or Done
  ([ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Tinker*) —
  and a Set on every round. Its outcomes are `committed` and `nothing`,
  plus the one every Activity has, `steered`.
- **Investigating rounds are the same Activity configured**, not a second
  one: the following-up skill swapped for the investigating one, the
  evidence `Investigated` rather than `NothingElse` with commits, the
  branch's reading replaced by a check that nothing was committed. Stage 09
  wires that configuration in when it writes the interview; this stage builds
  the Activity so that it can be.
- **What an Activity names, it declares** (Set 1771, Q1). The skill a
  *launch a session* names is `<pack>/<skill>`, and the pack is the Activity's
  own plugin's or one of its declared **dependencies** (stage 01);
  `verkstead plugin check` and the loader refuse an `activity` block naming a
  skill from a pack it does not provide or depend on. The built-in Activities'
  plugin depends on the `verkstead` pack's, so unticking the pack says first
  that the Activities, and every Process over them, go with it.
- **The in-process trait comes first and stays.** Every built-in that has not
  crossed yet runs behind the same Rust trait the component host implements,
  so the runner is cut over to the trait in one move and each later stage
  swaps one implementation for a component. The trait is not a second
  interface: it is the WIT world's Rust rendering, generated where the
  tooling allows.

## Proposed tasks (provisional)

1. **The `activity` world** — the WIT: start facts, events in, host functions
   out, outcomes back; the `activity` provides-block in `plugin.yaml` naming
   the component, its outcomes and the skills it launches; `verkstead plugin
   check` reads them. AC: a plugin providing an Activity loads and lists; one
   whose component lacks the world's exports is listed with its trouble; one
   naming a skill from a pack it neither provides nor depends on is refused
   naming the skill.
2. **The trait behind the runner** — the runner's drivers (`runner.rs`
   `see_out`, `carry_on`, `follow_up` and the rest, `drivers.rs`'s registry,
   `stalls.rs`) call through one trait shaped like the world; the private
   `Prompt` enum's skill-and-role mapping becomes arguments to *launch a
   session*. AC: every existing driver runs through the trait; the sessions
   suite is green.
3. **The host side of the world** — wasmtime hosting of a component, each host
   function implemented over what the server already does: `skills.rs`
   builders for the prompt, `done.rs` for the evidence, `own_sets.rs` for a
   Set, the Timeline for a notice, `github.rs` for the Forge. AC: a fixture
   component that launches one session on a test skill and ends `done` drives
   a Conversation end to end in the suite.
4. **Follow-up rounds as a component** — `following_up`, `over`,
   `back_to_the_wrap_up` and `finished` in `runner.rs` and `follow_ups.rs`
   rewritten as a component in the built-in plugin, with the branch's reading
   at the end and the Nothing-else event. AC: a steered Follow-up and a Tinker
   run their rounds under the component; the follow-up tests pass unchanged;
   the Plugins page lists *Follow-up rounds* under Built-in.
5. **Restart without state** — a server restarted mid-rounds starts the
   component again with the Timeline and it carries on with the right round.
   AC: the suite's restart tests over a Follow-up pass with no table added.

## Re-verify at start

- Stage 07 landed: skills are named `<pack>/<skill>` and the built-in pack is
  `verkstead`, so *launch a session* names `verkstead/following-up`.
- Stage 01's loader and manifest grammar are what the `activity` block is
  added to; check the grammar's provides-block shape before adding one.
- `runner.rs` still holds the private `Prompt` enum near line 4013 and
  `launch` near 4302 mapping each kind to a `skills.rs` builder; `done.rs`
  still registers expectations through `Signals::expecting`.
- `runner::over` still computes `pushed` as `commits_landed > already`, and
  Tinker's ending still reads the branch against its base rather than that
  flag (processes stage 03) — the component must keep both readings apart.
- `own_sets.rs` is still how the server puts a Set of its own.
- The wasmtime version stage 01 pinned, and whether its component-model
  tooling generates the Rust trait or it is written by hand.
