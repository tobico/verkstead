# 01. The ConPTY, and a child inside a Job

## What to build

The Windows arm of `terminal`: a real pseudo-console a process runs on, and a
child of Verkstead's own to hold that process by. It replaces `terminal::absent`
— the uninhabited `Terminal` whose `open` only ever refuses — and it is the
one place in the codebase where the arm is chosen by `cfg` rather than by a
`Platform` value, because there is no value to be had: on one platform the type
is a descriptor the runtime is watching and on the other it is a pseudoconsole
handle and two pipes.

**The console.** Opened with `CreatePseudoConsole`, sized with
`ResizePseudoConsole`, closed with `ClosePseudoConsole`, written by hand against
`windows-sys` — which the desktop crate already depends on — the way the Unix
arm is written against `rustix` rather than against a PTY crate. The two pipes
are the end Verkstead holds: one is what the relay reads and one is what a
keystroke is written into. Reading must not park a thread: whatever the runtime
offers for a Windows pipe is what carries `read` and `write`, and the shape
those two present is the Unix arm's — `read` answers `Ok(0)` when there is
nothing left to come.

**The process.** Rust's `Command` cannot attach a pseudoconsole — the extension
that would is still unstable as of Rust 1.97 — so the Windows arm spawns with
`CreateProcessW` and an attribute list carrying the console, and returns a
`Child` of Verkstead's own rather than tokio's. That child is a process handle
inside a **Job Object** configured to kill everything in it when the last handle
closes, which is what `--die-with-parent` is on Linux and the keeper process is
on a Mac: a server that dies takes its sessions with it, and an ended session
takes every process it started. `outliving::keep` stays a no-op on Windows.

**One surface on both arms.** What the sessions module, the terminals module and
the Screen ask of a terminal is `open`, `spawn`, `resize`, `write` and `read`,
and what they ask of a child is `id`, `wait` and `start_kill`, plus the
kill-on-drop both call sites set. Those signatures are the same on both
platforms after this task. `Terminal::spawn` today takes a `tokio::process::
Command` and both call sites build one out of what the Sandbox rendered; a
tokio `Command` is not something a pseudoconsole can be attached to, so the
seam changes shape here — the Windows arm needs what the Sandbox described
(the program, the arguments, the environment and the working directory), not a
type that already decided how to spawn. Change it once, for both arms, ahead of
the rendering in task 02 that fills it.

Nothing above the terminal changes: the relay, the Capture, the Screen and the
Live grid are ordinary portable Rust and stay as they are.

## Acceptance criteria

- [ ] A `cmd /c echo` spawned on a Windows `Terminal` reaches `read`, and `read`
      answers `Ok(0)` once the process has gone and the console is closed.
- [ ] Dropping the child kills a process tree it started — a child that spawned
      a grandchild leaves neither running.
- [ ] The terminal suite's two cases have a Windows twin, asked with `mode con`
      rather than `stty`: a process started on it is on a console of its own,
      and a resize is something the process on it is told about.
- [ ] The Unix arm is untouched in behaviour, and its suite stays green.
