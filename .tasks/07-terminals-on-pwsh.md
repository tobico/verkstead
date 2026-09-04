# 07. Terminals on pwsh, and the note on their pane

## What to build

A Conversation Terminal opens on Windows: the same server-held pseudo-terminal,
the same virtual terminal, the same xterm in the browser, running a shell.

**Which shell.** There is no passwd entry to read a login shell from, so
`terminals::shell` gains a Windows answer beside the Unix one — the term is
about what a Terminal *is* rather than about what one platform does, so it is
one function with two arms rather than a second notion. `pwsh` where `where.exe`
finds it on the server's own `PATH`, and Windows PowerShell where it does not.
The Unix rules do not carry over: the check that a shell is under one of the
directories every Sandbox binds is a fact about a mount namespace, and on
Windows there are no such roots to be under — a Windows answer that went through
that check would fall back for every shell there is.

**Ending it.** The Job's kill after `LINGERING` is the whole of the ending
there; `hang_up` stays a no-op on Windows, there being no process group to send
a hangup to. Everything else about a terminal's life is unchanged: it lives
until its shell exits, it is closed, the Conversation closes or the server
stops.

**And the note on the pane.** A Conversation Terminal is a shell in the
Conversation's Sandbox and on Windows there is none, so the pane at `/terminal`
says so too — the third of the three places the one unsandboxed value from task
06 is read. Worded about the shell in front of them rather than about the agent:

> This shell is not sandboxed: on Windows it runs with your own account's reach
> until the sandbox stage lands.

## Acceptance criteria

- [ ] A terminal tab on Windows opens `pwsh` where it is installed and
      `powershell` where it is not, and `$PSVersionTable` typed into it prints
      on the Screen.
- [ ] Closing the tab ends the shell and everything the shell started, and the
      Conversation closing ends every terminal before its Worktree goes.
- [ ] vitest draws the note on the terminal pane of an unsandboxed view and
      draws none on a sandboxed one.
