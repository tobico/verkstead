# 05. Codex's install and onboarding row from its manifest

## What to build

The onboarding wizard still installs Codex as its own (`onboarding.rs`'s
`Row::CODEX`, `onboarding/install.rs`): Homebrew `codex` on a Mac, the sentence
`CODEX_ON_INTEL` on an Intel Mac where there is no installer, npm
`@openai/codex` on Linux and Windows, the per-distribution lines on Linux, and
ticked by default. Decided (Set 1836, Q6): this becomes the manifest's
per-platform install commands and instructions link (the `linux`/`macos`/
`windows` overrides), and the onboarding row comes from the plugin as a plugin
harness's does — still ticked by default in the wizard, as a built-in harness's
row is. Then sweep what Codex knowledge is left in non-test server code.

## Acceptance criteria

- [ ] The wizard on each platform offers and runs the same Codex install as before, ticked by default, and the Intel-Mac row says the same thing
- [ ] `grep -i codex` over non-test server code finds only the embedded plugin's directory and where a stored `codex` word is read as Codex's uuid
- [ ] The onboarding and installing suites pass, with no Codex install words left in the wizard's own tables
