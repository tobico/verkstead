# A stack of pull requests

A **Fix Merge Issues** Conversation takes a whole stack. At Start the server
walks the chain on GitHub from the pull request it was pointed at, both ways —
a base that is another open pull request's head, a head that is another's base —
records every pull request it found, and says on the Timeline what the stack is.
The narrowed wrap-up then waits on Mergeable and the checks for every one of
them, and a conflict anywhere in the stack dispatches one `addressing` session
told the whole ordered list from the bottom, which syncs the stack with `gh
stack sync`, resolves what that backs out on, runs the repository's tests and
pushes every branch.

A stage of its own because the store has to change: everything Verkstead knows
about a pull request is keyed one per repository today, declared inline in each
table's own `CREATE TABLE`, and a stack is several in one repository. So the
first two tasks are that rekey — the pull request's own facts, then the
wrap-up's bookkeeping — each rebuild going through the migrations that run as
the database opens, the way `pull_requests_by_repo` and `wrap_up_settled_by_repo`
already did. A lone pull request goes on being what stage 06 made it: one
session, the configured resolution strategy, goes counted per pull request.

Roadmap stage: [07: A stack of pull requests](docs/roadmaps/processes/07-a-stack-of-pull-requests.md)

## Tasks

- [x] 01: Several pull requests in one repository — [details](01-several-pull-requests-in-one-repository.md)
- [x] 02: The settling rule over several pull requests — [details](02-the-settling-rule-over-several.md)
- [x] 03: Walking the stack — [details](03-walking-the-stack.md)
- [ ] 04: One session for the stack — [details](04-one-session-for-the-stack.md)
