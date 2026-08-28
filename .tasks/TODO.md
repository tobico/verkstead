# Pipeline

Wrap-up covers every touched companion. The finish session opens a pull request
per companion holding commits, following that repository's own review process;
the server discovers and records each one; checks and comments are watched per
pull request, with fix and batch sessions told which repository and worktree
they are working in; one review session reads the whole of it and puts one
Question Set; and Done waits for every pull request to settle. A touched
companion the finish left without a pull request stops the run with a Notice
naming it.

Touched means commits beyond the base at finish time. A read-only companion,
and a read-write one nobody committed in, are ignored by wrap-up entirely — no
pull request expected, nothing waited on. The merge is still not waited on, for
the reason it never was: stages stack on unmerged predecessors.

Demonstrable end to end: a Conversation with a read-write companion it committed
in reaches Wrapping with two pull requests pinned, each named with its
repository; a red check on either dispatches a fix session that works in the
right worktree; a comment on either is answered there; one review Set covers
both; and the Conversation reaches Done only once both have settled.

Roadmap stage: [03: Pipeline](docs/roadmaps/companion-repos/03-pipeline.md)

## Tasks

- [x] 01: A pull request per repository — [details](01-a-pull-request-per-repo.md)
- [x] 02: A pull request per touched companion — [details](02-a-pull-request-per-touched-companion.md)
- [x] 03: Checks per pull request — [details](03-checks-per-pull-request.md)
- [x] 04: Comments per pull request — [details](04-comments-per-pull-request.md)
- [x] 05: One review across every pull request — [details](05-one-review-across-them-all.md)
