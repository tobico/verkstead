# 05. OpenCode

## Goal

OpenCode is a backend, and the blocking ask's second holder. Demonstrable: a
Conversation grilled and built under an OpenCode Profile — blocking
`verkstead ask` held open by the shell tool, screen-signature idling, its
session store read onto the Timeline, `provider/model` Pairings, and the form
offering the type.

## Decisions in force

All from [ADR-0011](../../adr/0011-agent-backends.md); what bears on this
stage, with the research facts that shaped them (v1.18.x, August 2026 — the
project moves fast, re-check everything):

- **Home.** One Profile directory, with OpenCode's XDG config and data
  directories pointed into it (`XDG_CONFIG_HOME`, `XDG_DATA_HOME` and kin
  set for the session, or bound at the XDG defaults under the sandbox HOME).
  Auth lives in the data directory's `auth.json`; providers by OAuth or API
  key both land there.
- **Launch line.** Model by `-m provider/model` — the Pairing's model string
  is simply typed in that form on the Profile — and the initial prompt by
  `--prompt`, whose auto-submit behavior is unverified: if it only prefills,
  Verkstead types the submit through the terminal, the channel it already
  has. Approvals are a permission config: Verkstead passes allow-everything
  at launch (the environment-variable form suits an orchestrator), and
  OpenCode has no sandbox of its own to switch off.
- **The blocking ask holds.** OpenCode's shell tool is synchronous and
  accepts any timeout the model passes, holding no model turn open — so the
  ask blocks as Claude's does, free. The Guide's OpenCode tailoring (stage
  02's mechanism) says to pass a large timeout; the store-and-nudge path is
  not this backend's.
- **Session identity is found, not named.** No session id at launch;
  sessions live in the data directory's store as JSON (with state migrating
  into SQLite across versions — the layout is explicitly unstable). Match on
  the session's project/worktree and start time, as Codex's discovery does;
  lines/records verbatim, parsed at render time, Capture the record when
  the store cannot be read.
- **Usage limits are provider-shaped and retried internally** before
  anything surfaces; this backend ships with no phrase at all until one is
  observed, and such a stop lands as an ordinary stall meanwhile. No phrase
  means the matcher skips the backend, not that it matches an empty string —
  see stage 04's note on `limits::says_so`, which would otherwise read the
  first line of anything as a limit.

## Proposed tasks (provisional)

1. **The `opencode` type launches.** Variant, XDG-shaped binds, argv and
   permission-config mapping, form row; the prompt-submit question answered
   on the real thing.
   - An OpenCode Profile saves, pairs and launches; the Brief reaches the
     session and it starts on it.
2. **The blocking ask under OpenCode.** The Guide's tailoring, and the proof
   that the held command survives a long answer gap.
   - A grilling under the type blocks on `verkstead ask` and resumes with
     the Response on stdout.
3. **The session-store reader.** Discovery by project and start time;
   rendering its records as turns and bookkeeping.
   - A real session's Transcript draws; a store this build cannot read
     leaves the session Capture-only rather than failing it.
4. **OpenCode's idle signature, and the end-to-end proof.**
   - The real TUI reads idle at its prompt; a full Conversation runs under
     the type.

## Re-verify at start

- The session store's current shape first — JSON files against SQLite is
  version-dependent and was mid-migration when this was planned; pin what
  the supported release is and read that.
- `--prompt`'s submit behavior on the pinned release.
- The permission config's current spelling (config key, environment
  variable) and the shell tool's timeout default and override.
- What stages 03 and 04 taught about signatures and alt-screen handling.
