# 05. The docs

## What to build

Bring the project's own documents up to what tasks 01 to 04 built, in the
project's vocabulary.

- **CONTEXT.md.** The **Built Root** entry is written for Claude alone ("The
  `~/.claude` a Claude session runs in"). It should describe the root for every
  harness: what each one's allowlist is, the switch, and what is written back.
  The **Agent Profile** entry gains the memory switch: on by default, a fact
  stored beside the account, and what off means. Its sentence about the other
  harnesses' homes being "bound at those defaults" goes.
- **ADR-0011.** The amended paragraph already names the rule for all four. Add
  an *As built* for Codex, Grok Build and OpenCode, as stage 01 did for Claude.
  It should cover the Codex databases and the `sessions/` plus `memories/`
  choice, what was found about Grok's `index.sqlite`, OpenCode's data directory
  joined whole, and the config keys each written file carries. Also record that
  **discovery reads where the store really is on the host** (the account's when
  memory is on, the root's when off), because a Linux join is a bind inside the
  namespace. This corrects the roadmap brief's "discovery reads the built root".
- **ADR-0014**, where its Windows grant paragraphs still speak of Claude's root
  alone.
- **The adoption doc**, including its platform sections and its NixOS `paths`
  example, wherever it says a Codex, Grok or OpenCode account is mounted or
  joined whole, and a word on the memory switch.
- **`docs/design`**, wherever it says the same.

## Acceptance criteria

- [ ] No document says a Codex, Grok Build or OpenCode account is bound, mounted or joined whole into a session.
- [ ] CONTEXT.md defines the memory switch, and its Built Root entry covers all four harnesses.
- [ ] ADR-0011 has an *As built* for the three harnesses, including the discovery correction.
