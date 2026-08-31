# 01. Record the harness a session ran under

## What to build

What a session ran under is written down at launch — the profile's name and the
model id, copied so the record survives a profile being renamed or deleted. The
agent type is not among them, so the timeline and the status button cannot say
which harness a finished run was launched from. Record it the same way: written
at launch beside the existing copies, never derived from today's profiles.

The store keeps it per the companion-table pattern the session pairings
themselves follow (a fact that arrives later is a table beside the old one, not
a migration): a new STRICT table keyed by the same timeline event id, holding
the agent type as the store's own word for it (`claude`, `codex`, `grok`,
`opencode`). Write it inside the same transaction that records the pairing, so
the event and the whole of what ran arrive together or not at all. `RanUnder`
gains the agent type as an optional field, read back alongside the profile and
model for the whole timeline.

On the wire, the Agent-run timeline event gains an optional agent-type field
spelled the way `ProfileAccount`'s discriminator already spells it — `"Claude"`,
`"Codex"`, `"Grok"`, `"OpenCode"` — so the viewer's generated types line up with
the union it already has. Regenerate the viewer's types the repository's way
(they are rewritten by `cargo test` in the render crate; the diff is the check).

An event from before this change has no row, and that is not an error anywhere:
readers get nothing rather than a guess, exactly as they do for a session whose
pairing was never recorded.

## Acceptance criteria

- [ ] A session started now carries its agent type on its Agent-run timeline
      event over the UI API, alongside the profile name and model id.
- [ ] Events recorded before the change carry no agent type, and every reader
      of the timeline handles that as nothing rather than an error.
- [ ] The recorded value is the launch-time fact: renaming or deleting the
      profile afterwards changes nothing about what the event reports.
- [ ] The generated TypeScript types include the new optional field and the
      committed file matches what `cargo test` writes.
