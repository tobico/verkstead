# 02. The shared reading

## What to build

One formatter for how a pairing reads, used by every site that says who runs a
session. This task builds the formatter and applies it where the data is
already in hand: the pairing pickers on the setup card and the steer modal, and
the Brief details pane's Grilling / Implementation / Review facts. (The Agent
run timeline card, its details pane header and the status button follow in
task 03; harness marks in task 04.)

The reading composes rather than being hand-kept per pairing:

- `<harness display name> <model display name>` — "Claude Code Fable 5",
  "OpenCode Minimax M2.1".
- **Collapse rule**: the harness name is dropped when the model's display name
  already contains the harness's brand word — the first word of its display
  name, matched as a whole word, case-insensitively. So a Grok Build profile on
  `grok-4.6` reads "Grok 4.6", and a Codex profile on `gpt-5-codex` reads
  "GPT-5 Codex".
- **Profile suffix**: ` — <profile name>` is appended only when the saved
  profiles hold more than one of that harness (agent type), counted over all
  saved profiles regardless of model.
- A pairing with no model reads `<harness name> — <profile name>`
  ("Claude Code — Work"), suffix unconditional there.
- An unknown model id degrades to `<harness name> <raw id>`, never to nothing.

Two shared vocabularies get a home the formatter can reach:

- The harness display names ("Claude Code", "Codex", "Grok Build", "OpenCode")
  currently live module-private in the profile list page; lift them (and the
  viewer-side agent-type union) into a shared module and re-use them there.
- The known-models table gains entries and a harness tag apiece:

  ```
  grok-4.6                → Grok 4.6      (Grok)
  grok-4.5                → Grok 4.5      (Grok)
  minimax/minimax-m2.1    → Minimax M2.1  (OpenCode)
  opencode/gpt-5.1-codex  → GPT-5.1 Codex (OpenCode)
  ```

  The existing Claude entries are tagged Claude. The profile form's offered
  model checkboxes filter by the form's picked agent type, so Grok ids are not
  offered on a Claude Code profile; the free-text "another model id" escape
  hatch stays for everything the list does not know. The models test asserting
  every known id starts with `claude-` moves with this.

Tests pinning the old strings (`profile — raw-id` in the pickers, the Brief
facts, the profile form's checkbox labels) move to the new reading.

## Acceptance criteria

- [ ] Picker rows and Brief facts read "Claude Code Fable 5 — Work" style, the
      suffix drawn only where that harness has more than one saved profile.
- [ ] "Grok 4.6" and "GPT-5 Codex" — the collapse rule works on both spellings;
      a no-model pairing reads "Claude Code — Work"; an unknown id reads as
      harness + raw id.
- [ ] The profile form offers each known model only under its own harness, and
      hand-typed ids still work.
- [ ] The `<select>` option values and everything sent over the wire are
      unchanged — only what the human reads moved.
