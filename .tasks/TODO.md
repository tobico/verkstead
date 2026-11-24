# The Forge interface and GitHub across

A **Forge** becomes a plugin's to provide, and GitHub is the first one across
the boundary. A manifest's `forge` block declares the hosts the Forge answers
for, the noun it calls a pull request, how git in the sandbox authenticates
against it, the tool lent to the agent, and its capabilities — `checks`,
`review`, `publish`, `stacks`, `create` — and its component exports the
operations the server has always asked of GitHub.

When this stage is done the built-in GitHub plugin is a wasm component over
WASI HTTP, `gh` is gone from the server, and nothing the human sees on a Repo
at github.com has changed. Underneath, a session opens its pull request with
`verkstead pull-request open`, every skill has lost its `gh` lines but the
capability-guarded `gh stack` ones, a Repo whose remote matches no enabled
Forge says *No forge for <host>*, and each Author holds one token per Forge.

Roadmap stage: [12: The Forge interface and GitHub across](docs/roadmaps/plugins/12-the-forge-interface.md)

## Tasks

- [x] 01: The Forge trait and the `forge` block — [details](01-forge-trait-and-block.md)
- [x] 02: A Repo's Forge — [details](02-a-repos-forge.md)
- [x] 03: Tokens per Forge per Author — [details](03-tokens-per-forge.md)
- [x] 04: The noun — [details](04-the-noun.md)
- [x] 05: Capabilities — [details](05-capabilities.md)
- [x] 06: `verkstead pull-request open` and forge-neutral skills — [details](06-pull-request-open.md)
- [x] 07: GitHub across: the component — [details](07-github-component.md)
- [x] 08: `gh` leaves the server — [details](08-gh-leaves-the-server.md)
