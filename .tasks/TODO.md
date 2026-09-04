# The ConPTY terminal and unsandboxed sessions

A Windows 11 machine runs a grilling from **Start work** to its first Question
Set: the Screen shows the agent working and takes a keystroke, a Conversation
Terminal opens `pwsh`, and every one of those says plainly that it is not
sandboxed. Two things stood between a Windows Verkstead and a session — the
pseudo-terminal and the Sandbox — and this stage is the first of them. Between
this stage and the third a Windows session runs **unsandboxed**, with the
human's own account's reach, and the workbench says so in three places.

What lands here: a ConPTY opened by hand with a child inside a Job Object, a
third rendering of the Sandbox that sets the environment and runs `argv`
directly, a fresh profile per Conversation with the Profile's account joined
into it, the prompt written to a file, `SessionsHere::NotOnWindowsYet` removed
with every refusal it carried, Conversation Terminals on `pwsh`, and a Windows
end-to-end suite the `windows-2025` job runs.

Roadmap stage: [01: The ConPTY terminal and unsandboxed sessions](docs/roadmaps/windows-sessions/01-conpty-terminal.md)

## Tasks

- [x] 01: The ConPTY, and a child inside a Job — [details](01-conpty-and-job.md)
- [x] 02: The open rendering — [details](02-open-rendering.md)
- [x] 03: The fresh profile — [details](03-fresh-profile.md)
- [x] 04: A replaced link is written back — [details](04-written-back.md)
- [x] 05: The prompt goes to a file — [details](05-prompt-file.md)
- [x] 06: Sessions turn on, with the note — [details](06-sessions-turn-on.md)
- [x] 07: Terminals on pwsh, and the note on their pane — [details](07-terminals-on-pwsh.md)
- [x] 08: The Windows end-to-end suite — [details](08-windows-suite.md)
- [x] 09: Docs — [details](09-docs.md)
