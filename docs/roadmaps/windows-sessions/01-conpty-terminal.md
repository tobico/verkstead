# 01. The ConPTY terminal and unsandboxed sessions

## Goal

A Windows 11 machine runs a grilling from **Start work** to its first Question
Set, the Screen shows the agent working and takes a keystroke, a Conversation
Terminal opens `pwsh`, and every one of those says plainly that it is not
sandboxed. The `windows-2025` CI job proves the same end to end with a
stand-in agent. `SessionsHere::NotOnWindowsYet` and its wording are gone.

## Decisions in force

All from [ADR-0014](../../adr/0014-windows-sessions.md); the why is there and
is not repeated here beyond what a task needs.

- **ConPTY, by hand on `windows-sys`** (grilling Q9). `CreatePseudoConsole`,
  `ResizePseudoConsole`, `ClosePseudoConsole`; the two pipes are the held end.
  The Windows arm of `terminal` replaces `absent.rs`, and the `cfg` select at
  the top of the module stays a `cfg` — the one place the codebase allows one,
  because there is no value to be had.
- **`CreateProcessW` by hand, and a `Child` of Verkstead's own on Windows.**
  Rust's `Command` cannot attach a pseudoconsole. The Windows child is a
  process handle inside a **Job Object** with kill-on-close, which is what
  `--die-with-parent` and the macOS keeper are; `outliving::keep` stays a no-op
  on Windows. Whatever the sessions module and `terminals` ask of a child —
  `id`, `wait`, `start_kill` — is the same on both arms.
- **The agent is found with `PATHEXT`** and the command line quoted by
  `CommandLineToArgvW`'s rules; `claude.cmd` and `claude.exe` both start.
- **A third rendering of the Sandbox, chosen by Platform as a value**: the
  "open" one, which sets the environment and the working directory and runs
  `argv` directly. The two-way `cfg` between `bwrap` and `seatbelt` at the top
  of `sandbox.rs` becomes a Platform match, and a Windows build stops compiling
  the bubblewrap renderer as its own.
- **Environment cleared, then set explicitly**, as every rendering does, plus
  the Windows names nothing runs without: `SystemRoot`, `SystemDrive`,
  `ComSpec`, `PATHEXT`, `TEMP`/`TMP`, `USERPROFILE`, `APPDATA`,
  `LOCALAPPDATA`. `PATH` is Verkstead's own bin followed by the server's own
  `PATH` — no `WINDOWS_PATH` constant beside `LINUX_PATH` and `APPLE_PATH`.
  `PATH_LIST_SEPARATOR` already says `;`.
- **The fresh profile** (Q5, Q5a): a directory per Conversation under the Data
  Directory, the way `Homes` already does on a Mac; `USERPROFILE` and `HOME`
  point at it, the Profile's account joined in — **every directory in it by a
  directory junction and every file by a hard link**, over whichever of the
  four account shapes the Profile names rather than over Claude's pair alone;
  `APPDATA`, `LOCALAPPDATA` and `TEMP` point inside it. Data Directory and
  profile on different volumes refuse the session with a line saying why.
- **The prompt goes to a file** (Q6): always on Windows, never elsewhere.
  Written to the Conversation's handoff directory; the agent is started on one
  line naming it. The stand-in agent in the Windows suite reads it from there.
- **`nix develop` is skipped by Platform** on Windows; `under_dev_shell` takes
  the Platform rather than shelling out to a `nix` that is not there.
- **Build Cache and sccache are in** (Q12): `CARGO_HOME` shared as everywhere,
  the sccache found on the server's `PATH` where there is one, and the Compile
  Server run through the open rendering — a plain process on Windows, since
  there is no Sandbox of its own to run it in yet.
- **The unsandboxed note** (Q1a, Q8): one server-decided value on the
  Conversation view, the way `compiles_uncached` is, drawn above **Start work**
  on the composer and beside the terminal on the session pane. `SessionsHere`
  loses `NotOnWindowsYet`; `run_on(Platform::Windows)` says `Run`. The five
  refusal call sites and the viewer's `NoSessions` go with it. Draft wording,
  the human's to change: *"This session is not sandboxed: on Windows the agent
  runs with your own account's reach until the sandbox stage lands."*
- **Conversation Terminals** (Q4, Q11): `pwsh` where `where.exe` finds it,
  else Windows PowerShell, on the same ConPTY; `terminals::shell` gains a
  Windows answer beside the passwd one. Ending is the Job's kill after
  `LINGERING`; `hang_up` stays a no-op.
- **Skills and handoffs get Windows homes** under the Data Directory, the way
  the Mac spells them: `own_directory`, `own_bin`, `Skills::inside`,
  `handoffs::inside` and `handoffs::said` all gain a `Platform::Windows` arm
  that is no longer the Linux spelling. `Executable::inside` is the real path
  of the running image on Windows; nothing is bound anywhere.
- **A Windows end-to-end suite of its own** (Q7): a dozen or so tests in a
  `cfg(windows)` file with a PowerShell stand-in agent, run by the
  `windows-2025` job — start, output reaching the Capture, resize reaching the
  process, typing, ending, the prompt file, the fresh profile's variables, the
  note on the view, a terminal opening `pwsh`. The Unix sessions suite stays
  `cfg(unix)`.
- **Docs**: the README's "Windows has everything but those", `adoption.md`'s
  Windows section, `design/verkstead.md`'s note and `development.md` say what
  is now true, including that a session runs unsandboxed until stage 03. And
  **CONTEXT.md's Sandbox term**, which says nothing about Windows today and
  gains the unsandboxed state here — the term is written as what the product
  is, so it moves when the stage moves it rather than ahead of it; stage 03
  takes the same sentence back out. ADR-0014 already says the why.

## Proposed tasks (provisional)

1. **The ConPTY opens and a process runs on it** — the Windows `Terminal`
   with `open`, `spawn`, `resize`, `write`, `read`, and the Windows `Child`
   inside a Job. Accepts: a `cmd /c echo` spawned on it reaches `read`;
   `read` returns `Ok(0)` after the process exits and the console is closed;
   dropping the child kills a process tree it started; the terminal suite's
   two cases have a Windows twin asked with `mode con` rather than `stty`.
2. **The open rendering** — the third renderer chosen by Platform, the cleared
   and explicit environment, `PATH` from the server's, `PATHEXT` lookup, the
   quoted command line, `nix develop` skipped, the Windows homes for skills
   and handoffs. Accepts: `Sandbox::command` on Windows is `argv` with no
   wrapper; a session's environment holds exactly the listed names; a
   `claude.cmd` on `PATH` starts.
3. **The fresh profile** — `Homes` on Windows makes the directory, joins the
   account in by junction and hard link, points the five variables into it,
   refuses across volumes. Accepts: inside the session the Profile's account
   is the real one, asked of a Claude Profile *and* of a type whose account is
   one directory, so the rule is proved over the account rather than over
   Claude's pair; a file written to `%TEMP%` lands under the fresh profile; a
   Data Directory on another volume is refused with the line.
4. **The prompt file** — on Windows the prompt is written to the handoff
   directory and the agent started on one line naming it. Accepts: the
   stand-in agent reads the Brief from the file; Linux argv unchanged.
5. **Sessions turn on, with the note** — `run_on` says `Run` for Windows, the
   view carries the unsandboxed value, the composer and session pane draw it,
   `NotOnWindowsYet` and `NoSessions` are removed with their tests. Accepts:
   vitest covers the note in both places; the refusal call sites are gone;
   the Rust test that said Windows runs no sessions says the opposite.
6. **Terminals on pwsh** — the Windows shell answer, on the same ConPTY.
   Accepts: a terminal tab on Windows runs `pwsh` where present and
   `powershell` where not; `$PSVersionTable` prints on the screen.
7. **The Windows end-to-end suite and CI** — the `cfg(windows)` suite with
   the PowerShell stand-in, on the `windows-2025` job. Accepts: the job runs
   it green; a session's Capture holds what the stand-in printed; the sccache
   Compile Server starts as a plain process when sccache is on the runner.
8. **Docs** — README, adoption, design, development, CONTEXT.md. Accepts:
   nothing left in the docs says Windows runs no sessions; the unsandboxed
   state is written where a Windows reader will find it; CONTEXT.md's Sandbox
   term says what a Windows session gets today.

## Re-verify at start

- `absent.rs`'s exact surface and every caller of `Terminal` and `Child` —
  `sessions.rs`, `terminals.rs`, `screen.rs` — as they stand then; ADR-0013
  landed after the Windows port and may have moved them.
- Whether `std::os::windows::process::CommandExt::raw_attribute` has
  stabilised; if it has, the hand-rolled `CreateProcessW` is smaller than
  this brief assumes.
- How `Homes::for_conversation` makes and clears a Mac home, and whether the
  Windows one should share that code or its own.
- Where `outliving::keep` and `compile_server` are called on Windows today,
  and what `sandbox::rendered` does for the Compile Server under the open
  rendering.
- The `NotOnWindowsYet` call sites: five in the server plus `terminals.rs`,
  and the viewer's `NoSessions` in seven files, as counted at planning.
- Whether the `windows-2025` runner has `pwsh` and Git for Windows on `PATH`
  (it does today) and whether sccache is worth installing there for the
  Compile Server case.
