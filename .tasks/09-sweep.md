# 09. Sweep what still describes ending on quiet

## What to build

Tasks 01 to 08 each changed the code, the skills and the tests of one slice.
What is left is whatever still describes the old rule — that a session reports
through the repository alone and is ended on what landed plus quiet — in places
no one slice owned. `docs/adr/0018-a-session-says-it-is-done.md` and the
`CONTEXT.md` glossary already describe the finished work; this task makes
everything else agree with them, and makes them agree with what was actually
built.

- **The runner's and the Rescue's module documentation.** Both open with long
  accounts of *done plus quiet*, *every kind of session is ended by Verkstead on
  quiet*, *twice at most* and the would-not-ask stop. Rewrite them to the rule as
  built: ended on the done signal, checked against the repository at that moment.
  The same for the documentation on `Pace` — `grace` and `proposing` no longer
  mean what their comments say — and remove any field, function, constant or
  enum arm that nothing reads any more.
- **The design document, the development guide, the README and the roadmap
  documents still in flight** under `docs/`. Finished roadmaps are history and
  are left alone; anything that reads as a description of how Verkstead works
  today is corrected.
- **The ADR and the glossary against the code.** Read both again with the built
  thing beside them. Where the build settled a detail differently — and said why
  in its commit — correct the document rather than the code. Where it simply
  drifted, that is a bug: fix it or, if it is a real question, put it to the human
  as a Set.
- **The skills and the Guide, read as a whole.** Twelve skills were edited across
  six tasks. Read each end to end for a passage that still promises the session
  will be ended when it goes quiet, or tells the agent to *stop* as its last act
  where it should say to run `verkstead done`.
- **Test names and comments** in the session suites that describe a grace ending
  a session, where the test now shows a signal doing it.

Search for the vocabulary the old rule was written in — *quiet*, *grace*, *go
quiet*, *goes quiet*, *would not ask*, *landed plus*, *said nothing* — and read
every hit; most will be fine and some will not.

## Acceptance criteria

- [ ] No module documentation, design document or skill describes a session being ended by quiet, by a landing alone, or by an unanswered Rescue
- [ ] No dead code is left behind from the old enders: nothing unused, and no `Pace` field documented as something it no longer is
- [ ] ADR-0018 and the `CONTEXT.md` entries match what was built, with any deliberate difference recorded in them
- [ ] The whole workspace builds without warnings, and the Rust, Windows and web suites pass
