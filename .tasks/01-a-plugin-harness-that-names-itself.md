# 01. A plugin harness that names itself

## What to build

Stage 03's agent world calls `parse-transcript(account, session)` with the id
Verkstead named the session with. Codex takes no id at launch: today the server
finds its rollout by the Worktree it records and a file appearing after the
launch (`Search::Rollout` in the server's transcript module), reads the id the
rollout gives itself, and writes it down through `session_names` (the store
crate) — `names_itself` — which is what `codex resume` and a transfer to
another device later use (`in_the_store`, `codex_carried`). A plugin harness
has no way to do any of that. This task gives it one, generically, and proves it
on the stub harness under `plugins/tests/stub-harness`.

Decided (Set 1836, Q2): the agent world is unreleased (not on `main`), so
`verkstead:plugin@1.0.0` changes in place, no version bump. The export becomes,
in shape:

```wit
record looking {
  /// The id the session is known by, where there is one: Verkstead's own for a
  /// harness named on its line, or the one an earlier answer gave.
  session: option<string>,
  /// The Worktree it runs in, and when it was launched (ms since the epoch),
  /// for a harness that has to be found by them.
  worktree: option<string>,
  launched: u64,
  /// What the last answer said to carry on from; none to read from the top.
  cursor: option<string>,
}
record transcript {
  /// The id the record was found under — what a resume names.
  session: option<string>,
  entries: list<entry>,      // only those past `cursor`
  cursor: string,            // opaque, handed back next time
}
export parse-transcript: func(account: string, looking: looking) -> option<transcript>;
```

The host side: `Entries` (the following of a plugin harness's record) hands
back the id and cursor it was last answered with, so the store is searched once
and each poll reads only what is new (a Codex rollout runs to megabytes and is
polled about once a second). The first answer carrying an id is written down in
`session_names` exactly as `names_itself` does for Codex, once; a resume and a
carried Transcript (`carried`, `Carried::Entries`) use that recorded id. A
manifest whose launch line names no `{session}` is the case that is looked for
by Worktree and launch.

Extend the stub harness so one mode writes its record under an id it makes up
and does not take one on its line, with the record naming its working
directory, as a rollout does.

## Acceptance criteria

- [ ] The WIT, its bindings, the stub harness and the server's plugged harness use the new shape; the existing stub-harness tests pass on it
- [ ] A stub session launched with no id is found by its Worktree among another Worktree's records written at the same moment, and the id it chose is in `session_names`
- [ ] A Conversation transferred mid-session on the stub harness resumes it by that id, and its Transcript carries on rather than starting over
- [ ] A poll after the first reads only past the cursor (proven by a stub record that would answer differently if re-read from the top)
