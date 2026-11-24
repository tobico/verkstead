# 01. The Forge trait and the `forge` block

## What to build

Put one interface between the server and GitHub, and give the manifest
grammar the `forge` block that later tasks read.

**The trait.** A `Forge` trait in the server carrying every operation the
server asks of GitHub today:
- find a pull request by branch or by number;
- open one as a draft against a base;
- read its details, its checks, its comments and how it landed;
- comment and reply on it;
- list the open ones (the stack walk reads this);
- publish a Share as a gist, and delete one;
- create a repository;
- verify a token and its scopes;
- read a target out of prose;
- say whether a remote URL is one of its own.

`github.rs` over `gh` is its one implementation for now. Every caller goes
through the trait: the activity host's `forge-*` functions in `forge.rs`, the
checks, comments and merges watchers, publishing, repos, conversations' take-up
and resolve, done, stacks, targets and the UI's scope reading. The trait's
errors are the interface's error kinds — `NoPullRequest`, `NotLoggedIn`,
`NoRemote`, `Refused(said)` — plus the host's own `Ours`. `runner.rs` and
`done.rs` branch on those without reading any tool's text, and `gh`'s stderr is
matched only inside the `gh` implementation.

**The `forge` block.** Add a `forge` provision to the manifest grammar:
- `hosts` it answers for;
- `noun` (e.g. *pull request*);
- `credential`: the git username, and the hosts it covers inside the sandbox;
- `tool`: the CLI lent to the agent, and its token variable (`gh`, `GH_TOKEN`);
- `capabilities`: a subset of `checks`, `review`, `publish`, `stacks`, `create`;
- the `scopes` a token needs.

Add a **`secrets` capability** too (Set 1878, Q2: chosen over leaving it
implied by the block). It names the environment variable the host sets a
secret in for a call, granted at Install the way `outbound` is.
`verkstead plugin check` refuses an incomplete block, an unknown capability
and a malformed `secrets` grant, each naming the field.

Ship GitHub as a built-in plugin with a stable uuid declaring the block. It
declares hosts `github.com`, noun *pull request*, username `x-access-token`,
`gh`/`GH_TOKEN`, all five capabilities, and the scopes Verkstead needs today
(`repo`, `gist`). It asks for `outbound: api.github.com` and `secrets` for its
token. It is still backed in-process by the `gh` implementation; task 07
replaces that with a component. The WIT `forge` world is task 07's, where a
component first needs it.

## Acceptance criteria

- [ ] No module but the `gh` implementation of the trait names or runs `gh`
      (lending `gh` into the sandbox as the agent's tool excepted), and every
      caller listed above reaches GitHub through the trait.
- [ ] `verkstead plugin check` refuses a `forge` block naming a capability
      that is not one of the five, one missing `hosts` or `noun`, and a
      malformed `secrets` capability, each with the field named.
- [ ] The GitHub plugin is listed under Built-in on the Plugins page as a
      Forge, and the server suite passes unchanged.
