# Session account

A Windows session stops running inside an AppContainer and starts running as a
**local account of Verkstead's own**. The container was built, and the first real
session inside one refused every file it was given: nothing can resolve a path
inside a container however well the path is granted, and msys2 will not start in
one at all. Three mechanisms were then probed on a real machine — a restricted
SID list, a deny-only account SID, a lowered integrity level — and each gave a
real boundary while breaking node, both PowerShells or `bash`. An ordinary local
account breaks none of them, because from the machine's point of view there is
nothing unusual about it.

What a session may reach is unchanged: the same Surface, the same
access-control entries on the same real paths, the same per-Conversation record
and startup sweep. What changes is whose SID those entries name, how a process
is started under it, and that the human's own files are now refused for reads as
well as writes. The costs are an elevated step once at install, one account for
the whole installation rather than one per Conversation, and a launcher process
that makes the pseudoconsole on the far side of the boundary — none of which a
per-user install can avoid.

Roadmap stage: [04: The session account](docs/roadmaps/windows-sessions/04-session-account.md)

## Tasks

- [x] 01: The session account and the verb that makes it — [details](01-account-and-verb.md)
- [x] 02: Starting a process as the account, on a console — [details](02-starting-as-the-account.md)
- [x] 03: Entries for an account, with a step through every ancestor — [details](03-entries-and-ancestors.md)
- [ ] 04: Windows sessions run as the account — [details](04-sessions-run-as-it.md)
- [ ] 05: sccache back on for Windows — [details](05-sccache-back-on.md)
- [ ] 06: The docs and the viewer say what is true — [details](06-docs-say-what-is-true.md)
