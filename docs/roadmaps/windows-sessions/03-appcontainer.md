# 03. The AppContainer

## Goal

A Windows session runs inside an AppContainer: on the human's Windows 11
machine it reaches its Worktree and asks through the pipe, and is refused
their Documents; the unsandboxed note is gone; the `windows-2025` job runs an
AppContainer suite for real, the way the `macos-15` job runs the seatbelt
one. The stage opens with a probe, and its grilling reads the probe's output
before any of the rendering is written.

## Decisions in force

All from [ADR-0014](../../adr/0014-windows-sessions.md).

- **The probe first** (grilling Q19). A small program — a CLI verb or a test
  binary, the stage's choice — that the human runs on their Windows 11 machine
  and pastes the output of. It answers, from inside a throwaway AppContainer:
  whether a connection to `127.0.0.1` is refused and whether one to the
  machine's own address is; whether `node`, `pwsh` and `git` start under it;
  whether a ConPTY opened outside and handed in works; whether sccache's
  client reaches a server outside. Nothing else in this stage is written
  until that output is in the stage's grilling, and the tasks below are
  provisional to exactly that degree.
- **The rendering** (Q1): a third `Surface` renderer beside `bwrap` and
  `seatbelt`, chosen by Platform. `CreateAppContainerProfile` for the
  identity, `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` on the same
  attribute list stage 01's `CreateProcessW` already carries the
  pseudoconsole on. Like the Mac's: the machine is there and refused. Every
  path the Surface names is a real path — stage 01's fresh profile with the
  account junctioned in is what the container sees as `USERPROFILE`.
- **Reach is access-control entries** on the real directories: `Own` and
  `Elsewhere` become a grant to the profile's SID at the Surface's reach,
  `Nothing` an explicit deny entry, `Temporary` a directory of the session's
  own inside the fresh profile removed with the session (Q15), `ProcessTable`
  and `Devices` nothing at all. Program Files and Windows need no entry.
  **Each `PATH` entry under the human's profile is granted read-only** (Q1b),
  because per-user tool installs are not readable by a container the way
  Program Files is, and nothing else of the profile is.
- **One profile per Conversation** (Q14), named from the Data Directory and the
  Conversation id: granted at the first session, deleted with the Worktree
  and its entries stripped. **The server sweeps at startup**: profiles whose
  Conversation is Done or Closed are deleted and their entries removed from
  the directories the Surface would have named — the repo's git directory, the
  account, the `PATH` entries, Verkstead's own bin, the skills, the binds.
- **Network**: the internet-client capability only (Q16). Sessions ask through
  stage 02's pipe, whose security descriptor now grants the profile's SID.
- **sccache** (Q17): stays on where the probe says a container's client
  reaches the Compile Server, off where it says loopback blocks that too; the
  shared `CARGO_HOME` stays either way. Where it stays on, the Compile Server
  gets a container of its own the way it gets a Sandbox of its own on Linux.
- **A container that cannot be made refuses the session** (Q18) — a profile
  that will not create, a grant that fails, a volume the account cannot be
  linked across — as a missing `bwrap` does on Linux. No fall-back to the
  unsandboxed session.
- **The note goes.** The server-decided value from stage 01 says sandboxed;
  the composer and session pane draw nothing; `adoption.md`'s Windows section
  says what a session can and cannot get to, the way the Mac's does, with the
  entries left on real directories written down where a reader will look.
- **The AppContainer suite** runs for real on the `windows-2025` job: the same
  questions `tests/sandbox_macos.rs` asks of `sandbox-exec`, asked of the
  container by attempting — a file written where the Surface says read-write,
  refused where it says read-only or nothing, the pipe reached, loopback
  refused.

## Proposed tasks (provisional)

1. **The probe** — the program, its output, and the human's paste read into
   this stage's grilling. Accepts: the five questions above each print one
   line the human can paste; the grilling that follows records what each said.
2. **A container runs a process** — profile creation, the capability
   attribute beside the pseudoconsole, a `cmd /c echo` reaching the Screen
   from inside. Accepts: the process's token carries the profile's SID; the
   Job still kills it; the profile is deleted afterwards.
3. **The rendering** — `Surface` to grants and denies, the fresh profile as
   `USERPROFILE`, the session's TEMP, the `PATH` entries under the profile,
   `Nothing` over the account's skills. Accepts: the AppContainer suite's
   write/read/refused classification matches the Surface for every access
   kind; Documents are refused.
4. **Per-Conversation profiles and the sweep** — grant at first session,
   remove with the Worktree, sweep at startup. Accepts: a second Conversation's
   Worktree is refused to the first's session; after a simulated crash the
   next startup leaves no entry for a Done Conversation on the account.
5. **The pipe for the container** — stage 02's descriptor filled with the
   profile's SID; `verkstead ask` from inside lands a Set. Accepts: the
   Windows end-to-end suite's ask runs sandboxed; loopback from inside is
   refused, asserted rather than assumed.
6. **sccache as the probe said** — on with a Compile Server in a container of
   its own, or off with `CARGO_HOME` alone. Accepts: a Rust repo's session
   builds; `RUSTC_WRAPPER` is set or absent per the decision.
7. **The note goes, and the docs say what is true** — the view's value, the
   viewer, README, adoption. Accepts: vitest no longer draws the note on a
   sandboxed Windows view; adoption's Windows section reads beside the Mac's.

## Re-verify at start

- **The probe's output** — every claim in ADR-0014's loopback paragraph is
  made from documentation, and this stage's grilling is where the machine
  gets its say. If loopback is *not* refused for a desktop AppContainer, stage
  02's pipe stays (it is landed and harmless) and the sccache decision
  simplifies.
- Whether `CreateAppContainerProfile` needs anything the per-user msi install
  cannot give; whether the `windows-2025` runner's user can create one.
- Junction and hard-link semantics under a container: the grant is checked on
  the target, so the real account directory is what is granted.
- Whether Claude Code on Windows still keeps the pair at
  `%USERPROFILE%\.claude` and `.claude.json`, and whether `CLAUDE_CONFIG_DIR`
  has become a simpler road than the junction.
- Stage 01's `Homes` on Windows and stage 02's `Reachable` as they landed, not
  as their briefs imagined them.
