# 01. `wants` on the proposal, drawn as a line

## What to build

A grilling's closing Set can say which platforms the work wants, and the
human reads it on the chooser.

The `proposal` block gains an optional `wants`: a list, each entry a platform
and the reason the work wants it (ADR-0029, *Declaring what the work wants*):

```yaml
proposal:
  direction: roadmap
  rationale: |
    …
  wants:
    - platform: windows
      for: the ConPTY terminal stage and its tests
```

- **The three words and no others**: `linux`, `macos`, `windows`, as a stage
  line's `on <os>` spells them. They are an enum of the schema crate's own,
  exported to the viewer's generated types. The schema crate is built for wasm
  and cannot reach the server's `platform::Platform`, so the server maps one
  onto the other where it needs to. A fourth word is refused when the Set is
  read, as a fourth direction is, and the refusal names `wants`.
- **`for` is a non-empty reason** (`for` on the wire, a Rust keyword, so the
  field is renamed). An empty or blank one is refused by `check_proposal`,
  naming `wants`, because the reason is what the human reads beside the tick.
- **No `wants`, or an empty list, is a proposal that wants nothing**, and is
  drawn exactly as today.
- The proposal's view carries the wants (the reason rendered the way the
  rationale is), and the chooser draws them as a line of prose under the
  three directions: what the work wants and why, platform by platform, in the
  platform's own spelling (*Windows*, *macOS*, *Linux*). That line is the
  whole of what a Verkstead in no cluster draws. The ticks are task 02's.

Read `docs/copy.md` before writing the line.

## Acceptance criteria

- [ ] A closing Set wanting `windows` "for the ConPTY stage" is accepted, and
      its chooser shows a line saying the work wants Windows and why.
- [ ] `platform: freebsd`, or an empty or blank `for`, refuses the Set with a
      violation naming `wants`.
- [ ] A proposal with no `wants` round-trips without the key and draws as it
      did before; the generated `types.ts` is regenerated and committed.
