# 06. Claude across

## Goal

Claude Code, the harness in daily use, becomes an embedded plugin — the last of
the five — and stage 03's trait is left with one implementation, the wasm-backed
one. Every Agent row on the Plugins page is a component. A Claude Profile's two
paths are two declared paths and nothing special; its `settings.json` is written
by its component with the attached MCP servers and the bypass ADR-0011 writes
into the root; its login over pipes is the component's `login` export; its Named
JSONL transcript is its `parse-transcript`; byte silence is its declared idle
detection; and `verkstead guide` in a Claude session prints the blocking
section its manifest carries. The sessions, logins, transcript and mirroring
suites pass unchanged, and a Develop Conversation run end to end on Claude from
grilling to pull request looks exactly as it did the day before.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Agent***, and Set 1769's Q41:
  Claude crosses last because it is the harness in daily use and the one with
  the most measured behaviour — a fault in the boundary found on Codex costs a
  test, found on Claude costs a day's work.
- **Everything measured carries over**: ADR-0011 as built — the account as the
  `~/.claude` directory and `~/.claude.json` file pair, the Built Root seeded
  from it by allowlist, the cover over `~/.claude/skills`, `mcpServers` stripped
  from the copy and never written back, the bypass written into the root as well
  as passed on the line — and ADR-0022's login: `auth login --claudeai`, the
  address after `visit: `, `Paste code here if prompted >`, `Invalid code` on
  standard error for a cut code and the harness waiting on, `auth status`
  answering by exit code, an API key reading as logged in.
- **Two declared paths.** The pair is the manifest's `account:` list with two
  entries, labelled as the Profile form has labelled them; the per-path table
  stage 03 made holds both, and the migration that moved `claude_dir` and
  `config_file` into it is already behind this stage.
- **`write-configuration` writes the whole of what ADR-0011 writes**: the
  `settings.json` with the bypass and the attached servers, the `.claude.json`
  copy with the account's servers taken out — the host hands the export the
  stripped copy, the strip being the host's rule — and the project entries the
  root's `Harness::Claude` carries today. The strip-and-never-write-back rule is
  not the component's to break: the host writes back only what it wrote before.
- **`login` over pipes.** Claude's code is read off standard input rather than a
  terminal, so the manifest says so and the host runs the login on pipes; the
  component sees what Claude prints and answers with the address, the code to
  type and the cut-short word. The modal and its words stay the host's.
- **Byte silence is data.** Claude is judged idle on the bytes going quiet
  rather than a screen signature, and the manifest says `idle: silence`; the
  long-stop every harness has keeps being the host's.
- **The desktop app check stays host-side.** `sessions::desktop_app` detects the
  Claude desktop application sitting where the binary should be and refuses the
  launch with a reason; that is a fact about a binary on a machine rather than
  about the harness's record or configuration, so it is a per-platform
  `not-this-binary` declaration in the manifest — a path pattern the host refuses
  — and not an export.
- **The transcript parser moves.** The Named JSONL kinds the render crate keeps
  — `mode`, `permission-mode`, `pr-link`, `queue-operation`, `system` and the
  rest that are not the conversation — become the component's; unknown kinds
  fold away under their own name as ADR-0006 has it, which stays the host's.
- **The trait keeps one implementation.** The five in-process implementations
  are deleted with this stage; the trait stays, because the wasm-backed
  implementation is what every Agent plugin is, and a trait with one
  implementation is still the seam the suite drives a stub through.
- **The blocking section** of the Guide — Claude backgrounds the ask itself, as
  `RUNNING_CLAUDE` says today — is the manifest's markdown, fetched by the CLI
  since stage 03; a Guide printed outside a sandbox prints the core alone.

## Left open by stage 03

- **A declared file path skips the strip-and-never-write-back.** Stage 03's
  wasm-backed harness joins a `kind: file` account path into the session as it
  is: the account's own MCP servers reach the session, and whatever the session
  writes goes straight back to the account. Only a directory path's allowlist
  is copied in stripped. `docs/plugins.md` says so, and it is not yet a problem
  because no plugin harness declares such a file; Claude's `.claude.json` is
  exactly one, and holds `mcpServers` at the top level and under each project.
  So before Claude's pair can be two declared paths, this stage settles how a
  file path is given to a session under ADR-0011's rule — copied in with the
  account's servers taken out and only what the host wrote merged back, as
  `Harness::copied` does for `.claude.json` today, or another shape — and the
  host, not the component, owns it.

## Proposed tasks (provisional)

1. **The manifest** — two declared paths, the allowlist and credential file,
   silence as idle, the blocking channel and its Guide section, the model list
   with aliases, the `not-this-binary` pattern for the desktop app, per-platform
   blocks where the Mac login's seatbelt or the Windows prompt-file launch need
   a variable. AC: `verkstead plugin check` accepts it; an existing Claude
   Profile reads back through it.
2. **The component** — `parse-transcript` over the Named JSONL, `write-
   configuration` producing `settings.json` and the stripped `.claude.json` with
   project entries, `login` over pipes. AC: the render suite's Claude fixtures
   parse to the same kinds; a Conversation with a declared server gets it in the
   root's `settings.json` and the account's own servers never return; the login
   suite passes against the component.
3. **The crossing** — the in-process Claude implementation deleted, and with it
   the other four's remains and every per-harness constant in `sessions.rs`,
   `logins.rs`, `limits.rs`, `signouts.rs`; the trait left with the wasm-backed
   implementation. AC: the sessions, logins, transcript, limits and sign-out
   suites pass unchanged; `grep -i claude` in non-test server code finds the
   embedded plugin's directory and the migrations.
4. **End to end** — a Develop Conversation on Claude from Start to an open pull
   request, with a declared MCP server attached and a memo left, on this
   machine. AC: the Timeline, the Transcript and the Code pane read as before;
   the memo is in the next session's prompt.
5. **The record** — CONTEXT.md's **Agent Profile** entry revised to speak of
   declared paths rather than Claude's pair, and ADR-0011's *the account's
   shape is its type's* marked as amended by ADR-0030. AC: the terms match the
   screen.

## Re-verify at start

- Stage 05 landed: four harnesses are components, the login export has run on
  Linux and Windows, and the pattern for a two-part record exists.
- Claude Code's current release still prints `visit: ` and `Paste code here`,
  and still answers `auth status` by exit code; re-measure in a throwaway HOME.
- `sandbox/root.rs` still carries Claude's project entries on its private
  `Harness::Claude` variant and still strips `mcpServers` at the top level and
  under each project.
- The desktop-app detection in `sessions.rs` still exists and still fires on a
  path pattern rather than on anything the harness prints.
- The Mac login still runs inside the seatbelt policy and writes
  `.credentials.json` into the account (ADR-0011's Antigravity amendment);
  verify on cat.local by hand once the component is in.
- No sixth harness has landed in-process since stage 03; if one has, it crosses
  here too or gets a stage of its own.
