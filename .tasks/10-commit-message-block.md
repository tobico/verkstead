# 10. Commit message block

## What to build

Two halves of one change to commit messages:

- **Presentation**: the commit details pane renders the commit message body
  in the same boxed markdown treatment an answer set's Preface uses — a
  section headed "Message" whose body sits in the padded, bordered, rounded
  block — registered with the pane's jump navigation like the other
  sections. The rendering itself (server-rendered markdown, mermaid
  diagrams) is unchanged; only the framing changes.
- **Template**: the commit-message instructions the packaged skills give
  implementing agents currently put the mermaid delta diagram first, before
  the prose. Move it to the bottom: prose first, diagram after it, trailers
  still last. The same template text appears in six skill files under
  `crates/server/skills/`, and the ordering is also described in CONTEXT.md's
  Commit Summary vocabulary entry and the design doc — update all of them
  consistently. The server-side snippet logic that strips diagrams for the
  timeline card keeps working either way.

## Acceptance criteria

- [ ] An opened commit shows its message as a boxed "Message" section,
      reachable from the pane's jump navigation
- [ ] All six skill files, CONTEXT.md and the design doc describe the
      diagram-last ordering; no stray "diagram first" wording remains
- [ ] Rust and web tests pass
