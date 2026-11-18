# 03. The Agent interface

## Goal

A harness is a **Plugin** of the Agent **Category**: a `provides` block in a
`plugin.yaml` that says everything about the harness that is data, and a wasm
component that exports the three things about it that are code. The server
reaches every harness through one Rust trait, and at the end of this stage that
trait has six implementations — the five built-ins as they stand today, moved
behind it unchanged, and a wasm-backed one that runs whatever a manifest and a
component declare. A third-party harness installed from a repository launches a
real session: its Profile is made on the Profile form, which draws one field per
account path the manifest declares; its Pairing is picked on the composer with
the mark the manifest ships; `verkstead guide` inside its sandbox prints the
Running-the-ask section its manifest wrote; and its transcript is drawn on the
Timeline out of what its `parse-transcript` export returned. `AgentType` is no
longer a closed word list anywhere — store, render or viewer — and the ten
hand-kept viewer tables keyed by it are gone. Nothing crosses yet: the five
built-ins stay in-process Rust, which is stages 04 to 06.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Agent***: the split between data
  and code was decided piece by piece in the grilling (Set 1764). **Data**, in
  the manifest: the binary's name, the display name and the mark; the launch
  line — model flag, prompt flag or positional, whether the session id is named
  on the line, the resume shape (a flag or a subcommand), the tail flags, a
  session-only settings JSON — written with placeholders the host fills; the
  account as a **list of labelled paths**, with the allowlist, the credential
  files and the memory directories under each; idle detection as a screen
  signature or byte silence; the usage-limit and signed-out phrases; the ask
  channel, blocking or store-and-nudge, and the *Running the ask* section of the
  Guide as markdown; the model list with one default; the MCP tool-name syntax
  the prompt uses; a link to install instructions and, per platform, install
  commands; and **per-platform overrides** of env, paths and launch flags keyed
  `linux`, `macos`, `windows` over the common declaration. **Code**, as exports
  of the component: `parse-transcript`, `write-configuration` and `login`. Each
  was offered as data-with-an-override and the human chose the export outright,
  so there is no declarative JSONL mapping and no JSON-merge-at-a-path: a
  harness whose record is plain JSONL still ships a parser. Idle detection was
  the one piece kept as data, byte silence and a signature covering every
  harness met so far.
- **`parse-transcript` owns discovery as well as parsing.** Where the record is
  by session id is a fact about the harness that may itself need code — opencode
  keeps a SQLite database, agy a summaries database beside JSONL steps — so the
  export is handed the account's root and the session id and answers with the
  kinds the Transcript pane draws: said, tool call, result, system notice, and
  unknown, which folds away under its own name as ADR-0006 has always had it.
  The render crate's per-format kind lists (`transcript.rs` `ROLLOUT`, `GROK`,
  `AGY`, `OPENCODE`) become each component's business as it crosses.
- **`write-configuration` is the whole written account.** It takes the
  account's configuration as found and the declared MCP servers attached to the
  Conversation (ADR-0021) and answers with what the Built Root is to hold —
  settings.json, config.toml, opencode.json, whichever is the harness's own —
  plus the session-only settings the launch line names. The strip-and-never-
  write-back rule of ADR-0011 is the host's, not the export's: the host hands
  the export a copy with the account's own servers already taken out.
- **`login` drives the dialogue; the host owns the console.** ADR-0022's
  `Speaking` table — what the harness prints before the address, what it says to
  a cut code, when it gives up — becomes the export's behaviour: the host runs
  the harness's login line on pipes or on a pseudoconsole as the platform
  requires, feeds the export what the harness writes, and the export answers
  with the address to show, the code to type, or the words to say. A harness
  whose manifest declares no login is one Verkstead does not log in, exactly as
  Codex, Grok and OpenCode are today.
- **Profiles go generic** (Set 1764, Q17): the form draws one field per declared
  path, labelled as the manifest labels it; Claude's dir-and-file pair is two
  declared paths and nothing special; the `claude_dir` and `config_file` columns
  on `profiles` move into the per-path table that `profile_homes` generalises
  to. Keeping Claude's special shape beside the generic one was offered and
  declined.
- **The id is a string, the mark is a file.** `AgentType` stops being an enum:
  a Profile records the plugin's name, the render crate carries it as a string,
  and the viewer draws the mark the manifest ships (an SVG, like the lobehub
  ones in `web/src/marks/` today) rather than looking one up. The viewer's
  `KNOWN_MODELS` goes; the Profile form offers the plugin's declared list, and
  `pairing_defaults` reads the declared default (Q22).
- **The Guide is fetched** (Q18): `verkstead guide` asks the server for the
  section belonging to the harness `VERKSTEAD_AGENT` names, and prints the core
  with it spliced in; where the server cannot be reached it prints the core
  alone, which is what a Guide printed outside a sandbox has always been.
  Writing the tailored Guide into the Built Root was the alternative and was
  declined.
- **Installing the binary** (Q19): detection is a PATH walk by the declared
  name, as onboarding does for the five; a plugin may also declare install
  commands per platform, and they run only on the human's press, the wizard's
  own install run started from the Plugins page or the onboarding row.
- **Disabling** (Q20): allowed over standing Profiles. The row's Profiles stay,
  are not offered on any picker, a remembered Pairing on one is not applied —
  the broken-Profile rule CONTEXT.md gives under **Pairing** — and sessions
  already running finish. The row says how many Profiles it has.
- **One trait, six implementations.** The five built-ins move behind the trait
  with their behaviour byte-for-byte what it was, so the sessions suite proves
  the move; the wasm-backed implementation is a sixth, and a third-party plugin
  is only ever that one. Stages 04 to 06 replace the five with components and
  leave the trait with one implementation.

## Proposed tasks (provisional)

1. **The trait** — one `Harness` trait in the server naming every operation the
   survey found branching on `AgentType` (binary, line, signature, account
   layout, root building, transcript search and parse, login, phrases, channel,
   tool-name syntax, memory roots), the five built-ins as its implementations
   and every `match` site replaced by a call. AC: no `match` on `AgentType` is
   left in non-test server code; the sessions, logins and transcript suites
   pass unchanged.
2. **The `agent` provides-block** — the manifest grammar for every data piece
   above, with per-platform overrides, validated by `verkstead plugin check`.
   AC: a manifest missing a required piece is refused naming it; the WIT world
   with the three exports is in the repository with generated bindings.
3. **The wasm-backed implementation** — the sixth `Harness`, built from a
   manifest and a component: placeholders filled into the launch line, the
   component's three exports called with what the host holds. AC: a stub
   harness plugin (a script that writes a JSONL record) launches, idles, is
   nudged or blocks as declared, and its Transcript is drawn.
4. **Profiles generic** — the per-path table, the migration of Claude's two
   columns, the form drawing declared paths, the id as a string across store,
   render and viewer. AC: an existing Claude Profile reads back with both paths;
   a Profile of a plugin harness is saved and bound; the ten viewer tables are
   gone and the mark comes from the manifest.
5. **Models and the Guide** — declared models on the form and in
   `pairing_defaults`, `KNOWN_MODELS` removed; `verkstead guide` fetching its
   section. AC: the Guide inside a plugin harness's session carries its
   manifest's section; outside a sandbox it prints the core.
6. **Detection, install and disable** — onboarding rows from enabled Agent
   plugins, install commands on a press, the disable semantics above. AC: a
   disabled Agent's Profiles are absent from every picker and a Conversation
   remembering one starts with that role empty.

## Re-verify at start

- Stage 01 landed: the loader, the manifest root, the Replicated record and the
  Plugins page exist, and `verkstead plugin check` validates a manifest with an
  empty `provides`.
- `AgentType` is still the closed enum at `crates/store/src/profiles.rs` with
  five variants and `Account` still doubles as its discriminator; a sixth
  harness may have landed since, which changes the count and nothing else.
- The render crate still mirrors the enum in `profiles.rs` and ts-rs still
  writes `web/src/api/types.ts` on `cargo test`.
- `crates/cli/src/guide.rs` still splices `RUNNING_*` constants by
  `VERKSTEAD_AGENT`; the CLI still has no call to the server that is not an
  ask, a memo or a preview.
- Declared MCP servers are still written per harness in `sandbox/root.rs`, so
  `write-configuration`'s input is what that code builds today.
- The sessions suite's `Agents::running` stub override is still the only way to
  launch an arbitrary program as a harness; the stub harness plugin of task 3
  may replace it.
