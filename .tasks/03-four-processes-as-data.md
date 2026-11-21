# 03. Tinker, Investigate, Review and Fix Merge Issues as data

## What to build

The four Processes that run one or two Activities are written as manifests and
run by the engine, each a **bundled plugin of its own** listed under Built-in
on the Plugins page **with a tick**, as Agent plugins have (Set 1877, Q4):
unticking one takes its row off the picker, Conversations on it run to the
end. Each depends on the *Activities* plugin and the *Verkstead* pack plugin,
which stay always-on.

- **Tinker** — Draft → `follow_up` (Follow-up rounds) → `wrapping` on
  `committed`, Done on `nothing`; wrap-up Outcomes to Done, or `stopped` for
  `closed` and `given-up`.
- **Investigate** — Draft → `investigating` (Follow-up rounds under
  `verkstead/investigating`, its evidence `investigated`) → Done on either
  Outcome. The landing and the tidy-up host-side stay as they are.
- **Review** — takes a Target; Draft → `wrapping` (Wrap-up), reviewed.
- **Fix Merge Issues** — takes a Target; Draft → `wrapping`, its state's
  options turning the review and comments watchers off. That replaces
  `narrows_the_wrap_up` — the wrap-up reads options, never the Process (stage
  09's arrangement) — so whether `narrows-the-wrap-up` stays a manifest fact at
  all, or is only the state's options, is settled here and `docs/plugins.md`
  updated to match.

States are named with the words already stored (`follow_up`,
`investigating`, `wrapping`), so these Conversations already in the database
read on with nothing rewritten, and the stored bare Process words resolve to
these plugins (task 02's aliases). Host-side work that today hangs off a
particular Outcome in code — the follow-up's ending deciding between Wrapping
and Done, settling's rule before Done, a session seen out as a pull request
lands — keeps working: where it is a fact of entering Done or the wrap-up
rather than of one Process, it moves to Verkstead's side of that entry; where
it is the Activity's, it stays inside the Activity.

The Target field and the Process's description are read from the manifest
server-side; the viewer's own tables for these four go in task 07.

## Acceptance criteria

- [ ] The four are bundled plugins, each listed under Built-in with a tick, and
      unticking one takes it off the picker
- [ ] Their tests in the sessions suite pass with the four read from manifests
- [ ] A Conversation of each stored before this stage (a fixture database)
      reads its state and runs on
- [ ] Fix Merge Issues narrows the wrap-up through its state's options alone
