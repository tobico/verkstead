# 05. Grok, OpenCode and Antigravity across

## Goal

The three remaining non-Claude harnesses become embedded plugins, each a
manifest and a component, and their in-process implementations go. Grok's
updates record, OpenCode's SQLite records and Antigravity's summaries database
beside its JSONL steps are each read by their component's `parse-transcript`;
`config.toml`-shaped, `opencode.json`-shaped and `settings.json`-plus-
`mcp_config.json`-shaped configurations are each written by their component's
`write-configuration`; and Antigravity's is the **first real `login` export**,
driving the sign-in dialogue that ADR-0022's amendment measured — on a terminal,
and on Windows on the pseudoconsole the launcher makes as the session account.
Every suite that proved the three passes unchanged against the components, on
Linux in CI and on Windows where this stage is placed.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Agent***, and Set 1769's Q41: the
  built-ins cross one at a time, Claude last. These three go together because
  each is a one-home account and each teaches the boundary one new thing — a
  record that is not a file (OpenCode), a record in two parts (Antigravity), a
  login that is typed on a terminal (Antigravity) — and none is in daily use
  here.
- **Everything measured carries over**, not re-decided: ADR-0011's *As built for
  Codex, Grok Build and OpenCode* and its Antigravity amendment of 2026-11-01,
  and [the agent-backends roadmap's stages 04 and 05](../agent-backends/ROADMAP.md).
  In short — **Grok**: `~/.grok`, an updates JSONL, store-and-nudge, the record
  relocated on a resume (`transcript::carried`). **OpenCode**: no dot-directory,
  the home being the directory its XDG config and data sit inside; the record is
  rows in a SQLite database (`records.rs`), the session row rewritten on a
  carry; `opencode.json`; blocking, holding the ask in the turn under a timeout.
  **Antigravity**: `~/.gemini` found where it holds `antigravity-cli/`; the
  record a database per conversation under `conversations/` with no public
  schema, so the **transcript JSONL agy also writes** is what is followed, and
  the summaries database (`summaries.rs`) is what finds it; `settings.json` and
  `mcp_config.json` under `config/`; a session-only `--settings` JSON on the
  line (`agy_settings`); blocking, `run_command` backgrounding the ask itself;
  the `esc to cancel` signature.
- **`parse-transcript` reads databases through WASI.** OpenCode's records and
  agy's summaries are SQLite files: the component asks in its manifest for the
  account's data directory as a filesystem capability (ADR-0030, *Capabilities
  and trust*), and reads the database itself with a SQLite compiled into the
  component. The host hands over the root's path and the session id and takes
  back the Transcript's kinds; what `records.rs` and `summaries.rs` know moves
  into the two components. A built-in's capabilities are granted by shipping,
  since nobody installs it; the Plugins page shows them the same as any plugin's.
- **The `login` export, and Antigravity's.** The host runs the declared login
  line — `-p /help` — on a pseudoconsole wherever the manifest says the code is
  typed on a terminal, and feeds the component every byte the harness writes.
  The component answers with events: *here is the address to show*, *type this
  code*, *the code was refused*, *the harness gave up*. ADR-0022's modal and its
  refusals are the host's and do not move; what moves is `Speaking::AGY` — the
  `Authentication required` line, the indented address, `authentication timed
  out`, `Please sign in` as the signed-out phrase on `models`.
- **The Windows quirk is a per-platform override.** `SSH_CONNECTION` in the
  environment of every agy process on Windows — session, login and signed-out
  check — so agy keeps its token in a file rather than the session account's
  Credential Manager (ADR-0011's amendment); the manifest's `windows:` block
  carries that one variable, and nothing in the component knows which platform
  it is on.
- **The stage is placed on Windows** because the login path there is
  `cfg(windows)` code — the launcher making the pseudoconsole as the session
  account, the code typed into it, the address read off what it draws — and
  can be verified nowhere else. Linux is CI's. The Mac interplay — the seatbelt
  policy refusing the Keychain's mach services so agy falls to its file store,
  the summaries database being the root's own because agy refuses a symlinked
  one — is data and policy rather than cfg code, and is re-verified by hand on
  cat.local rather than declared.
- **Memory roots from the manifest**: the peer and mirroring code's per-harness
  memory directories (`peer/memory.rs`, `mirroring/memory.rs`) read the declared
  memory directories, Antigravity's `conversations/`, `brain/` and the summaries
  database among them.

## Proposed tasks (provisional)

1. **Grok across** — manifest and component; the in-process implementation
   deleted. AC: the Grok sessions and transcript suites pass unchanged; a resume
   relocates the record as before.
2. **OpenCode across** — the component reading the records database over a
   filesystem capability; the row rewrite on a carry; `opencode.json`. AC: the
   OpenCode suites pass unchanged; the Plugins page shows the capability the
   built-in holds.
3. **Antigravity across, Linux first** — manifest with per-platform blocks,
   component with all three exports; the summaries database read in the
   component. AC: the agy suites pass on Linux; a declared MCP server lands in
   `mcp_config.json`.
4. **The login dialogue on Windows** — the host's pseudoconsole login path
   driving the component's `login`; `SSH_CONNECTION` from the `windows:` block.
   AC: `logins_windows` passes on winvm against the component; a signed-out
   check reads the right phrase.
5. **Memory roots** — declared memory directories replace the per-harness match
   in the peer and mirroring code. AC: an agy Profile's memory syncs across
   devices as before.

## Re-verify at start

- Stage 04 landed: Codex is a component and the pattern for an embedded plugin
  — the crate, the CI build, the Built-in row — exists to copy.
- Grok, OpenCode and agy releases since may have moved their record shapes or
  signatures (`GROK_AT_WORK`, `OPENCODE_AT_WORK`, `AGY_AT_WORK`); re-measure
  each in a throwaway HOME before writing the manifest.
- `records.rs` and `summaries.rs` are still the only readers of the two
  databases, so moving them moves everything.
- The Windows login still runs on a pseudoconsole the launcher makes as the
  session account, and agy still skips its keyring under `SSH_CONNECTION`.
- The Windows sessions suite still runs under Wine here as well as on winvm;
  the login is the part Wine cannot prove.
- On cat.local: agy signed in by the card still writes its token to the
  account's file under the seatbelt, and still refuses a symlinked summaries
  database.
