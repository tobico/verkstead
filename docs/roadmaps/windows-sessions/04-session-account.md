# 04. The session account

## Goal

A Windows session runs as a **local account of Verkstead's own**: on the human's
Windows 11 machine it reaches its Worktree, opens a terminal, runs `bash`, and
is refused their Documents — for reads as well as writes. The AppContainer goes.
The `windows-2025` job runs the boundary suite for real, the way the `macos-15`
job runs the seatbelt one.

Stage [03](03-appcontainer.md) built this boundary once already and on a
mechanism that cannot host the agent. **Most of 03 is kept**: the
[`Surface`](../../../crates/server/src/sandbox/surface.rs), the entries
[`granting`](../../../crates/server/src/sandbox/granting.rs) works out, the
record under the Data Directory, the sweep at startup, the fresh profile, the
prompt file and the pipe are the same work. What changes is the identity those
entries are written for and how a process is started under it.

## Decisions in force

From [ADR-0014](../../adr/0014-windows-sessions.md), and specifically its
*Amended: the Sandbox is an account* section and the two *What the probe
answered* sections before it. The probes themselves are the three
`*-probe.rs` under `crates/server/examples`, and their output is what this
stage's grilling reads.

- **The identity is a local account**, created by an elevated step at install
  and nothing after. The account holds a long random password that does not
  expire and that it cannot change, is denied interactive logon, and is in no
  group but `Users`.
- **Its name is fingerprinted, and short.** Two Verksteads on one machine keep
  their accounts apart the way they keep their pipes apart — see
  [`pipe`](../../../crates/server/src/pipe.rs), whose `verkstead-{:016x}` is the
  scheme this follows. It cannot follow it exactly: a local account name is
  capped at 20 characters and that one is 26, so the prefix and the number of
  hex digits both come down. The name is written down in the record beside the
  entries, so a server that did not make the account can still say which one its
  entries are for.
- **One account for the installation**, not one per Conversation — a reversal of
  ADR-0014 Q14, and unavoidable: creating an account needs elevation, so one per
  Conversation would need elevation per Conversation. A session can therefore
  reach every *live* Conversation's Worktree. The human's machine, profile,
  repositories outside the Watched Paths and the account's own skills are all
  still refused, which is the boundary the Sandbox exists for.
- **The password lives beside the other secrets** under the Data Directory — see
  `secrets.yaml`. `CreateProcessWithLogonW` needs one and there is no
  passwordless route to another account's token.
- **The elevated step is a verb of Verkstead's own**, run once from an elevated
  terminal, rather than a custom action in the msi: the account and the password
  are made together and the password has to land in the Data Directory, which
  the installer does not know and may not have made yet. The verb creates the
  account, generates the password, writes it to the secrets file and says what
  it did; a second verb takes both away. Unelevated it refuses with a line
  naming what it needs, the way every other refusal here names one.
- **And taking it away takes the profile directory with it.** Starting a process
  as the account loads its profile, which makes `C:\Users\<account>` at the
  first session and leaves it there: the account being deleted does not take
  it, so the verb that removes one removes both. A machine that has run
  sessions and then had Verkstead taken off it should look as it did.
- **Reach is access-control entries**, exactly as 03 wrote them, for the
  account's SID instead of a profile's: `Own` and `Elsewhere` at the Surface's
  reach on the real path, `Empty` and `Temporary` read-write, `Nothing` refused,
  `ProcessTable` and `Devices` nothing at all, and each `PATH` entry under the
  human's profile read-only.
- **And a step through every ancestor**, which 03 did not need and this does:
  `FILE_GENERIC_EXECUTE`, not inherited, on each directory on the way to a
  granted path. Reaching a deep path needs no ancestor entry — an ordinary
  account skips the traverse check — but *resolving* one walks the prefixes, and
  an agent resolves a path before it reads it. The entry says nothing about what
  is inside, so the profile stays unlistable.
- **The console is made on the far side.** `CreateProcessWithLogonW` refuses an
  extended startup info, so a **launcher** — a verb of Verkstead's own binary,
  which a session already has bound in read-only — is started as the account
  with the console's two pipes as its plain standard handles, calls
  `CreatePseudoConsole` over them, and starts the agent on it with an ordinary
  `CreateProcessW`. Verkstead keeps the launcher's process handle, so the Job
  Object from stage 01 still holds the whole tree.
- **Windows PowerShell rather than `pwsh`** for a Conversation Terminal on this
  platform: `pwsh` on most machines is a Store execution alias, those are
  per-user, and the session account is refused one with error 1920. The fallback
  ADR-0014 already names becomes the ordinary case.
- **sccache is back on**, and loopback works from the account. The named pipe
  stays — landed, harmless, and the one transport no firewall has to agree with.
- **A boundary that cannot be made refuses the session**, as a missing `bwrap`
  does on Linux: no account, no password, an entry that will not write. It never
  falls back to the unsandboxed session of stage 01, whose note would then be a
  lie.
- **The note is already gone, and what replaced it is now untrue.** 03 landed in
  full: there is no unsandboxed note and no `SessionsHere::NotOnWindowsYet`
  left to remove. What there is instead is the AppContainer asserted as fact in
  `CONTEXT.md`'s **Sandbox** term, `adoption.md`, `README.md`, the viewer's
  `api/types.ts`, its build-cache settings pane and its setup instructions —
  along with the reason sccache is off, which stops being the reason. So this
  stage's last piece is a correction rather than a removal, and adoption gains
  the elevated step beside it.

## Proposed tasks (provisional)

1. **The account** — creating it, deleting it, and the elevated step that does.
   Accepts: an unelevated attempt refuses with a line that names what it needs;
   the account is made with the flags above and denied interactive logon; the
   password is generated and kept with the other secrets.
2. **A process runs as it** — `CreateProcessWithLogonW`, the environment, the
   working directory, the Job. Accepts: `cmd /c echo` reaches the Screen from
   inside; the process's token is the account's; the Job still kills it.
3. **The launcher** — the verb, the console it makes, the resize it takes over
   the pipe, the exit it reports. Accepts: a session's Screen paints; a resize
   from the viewer reaches the agent; an ended session leaves no launcher.
4. **The rendering** — `Surface` to entries for the account's SID, the ancestor
   steps, the fresh profile, `Nothing` over the account's own skills. Accepts:
   the boundary suite's write/read/refused classification matches the Surface
   for every access kind; the human's Documents are refused to reads and writes.
5. **The container goes** — `sandbox::container` becomes the account, the
   AppContainer profile and its capability are removed, and the record and sweep
   are rewritten against entries alone. Accepts: no `CreateAppContainerProfile`
   remains; a Done Conversation's entries are gone after a simulated crash and
   the next startup.
6. **sccache and the pipe** — the switch back on, the pipe's descriptor granting
   the account. Accepts: a Rust repo's session builds through the Compile
   Server; `verkstead ask` from inside lands a Set.
7. **The note goes, and the docs say what is true** — the view's value, the
   viewer, README, adoption, CONTEXT.md. Accepts: vitest no longer draws the
   note on a sandboxed Windows view; adoption's Windows section reads beside the
   Mac's and names the elevated step; CONTEXT.md's Sandbox term names the
   account.

## Re-verify at start

- **The three probes' output**, which is what every decision above rests on. Run
  the account probe again on the machine the stage is built on: `--set-up`
  elevated, the middle part as the human, `--tear-down` elevated.
- Whether `CreateProcessWithLogonW` still refuses an extended startup info on
  the machine at hand — the launcher exists only because it does.
- What `LOGON_WITH_PROFILE` leaves behind: the account gets a profile directory
  of its own under `C:\Users` at first logon, which is the machine's rather than
  the Data Directory's and is not swept by anything this stage writes.
- Whether a file the session account writes into a granted directory is
  afterwards readable by the human, which is what `sandbox::closing`'s
  write-back of a replaced hard link depends on.
- **`Settings::save_secrets` writes an empty file when there is no GitHub
  token**, which would eat a password kept beside it the first time somebody
  cleared their token. The rule wants to become *nothing set at all* rather than
  *no token*, and the settings tests are where that is held.
- Stage 03's `granting`, `container`, `open` and `closing` as they landed, not
  as their brief imagined them.
