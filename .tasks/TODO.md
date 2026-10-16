# The author list

A second Author — a git name, an email and a token of its own — is added on the
settings page's Git section, a Repo opened afterwards is recorded as its, and
every session on that Repo commits as that name, pushes with that token, and has
the first Author's token nowhere in its sandbox. Everything Verkstead itself does
on that Repo — its `gh` calls, the commits of its cuts, the first commit of a
created repository, sharing a Conversation — is done as that Author too.

An installation that never adds a second Author sees nothing change: the Git
section and the wizard read as they did, the single `git_author` and
`github_token` have become one Author in the files, and every registered Repo is
recorded as that one. The decisions are ADR-0025's and the **Author** term's in
`CONTEXT.md`.

Roadmap stage: [01: The author list](docs/roadmaps/authors/01-author-list.md)

## Tasks

- [x] 01: One Author, end to end — [details](01-one-author-end-to-end.md)
- [x] 02: Every Repo records its Author — [details](02-every-repo-records-its-author.md)
- [x] 03: The session gets its Repo's Author — [details](03-the-session-gets-its-repos-author.md)
- [ ] 04: The Git section's switcher and Add author — [details](04-the-git-sections-switcher.md)
- [ ] 05: The pick at Open repo and Create repo — [details](05-the-pick-at-open-and-create.md)
