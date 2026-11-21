# 07. GitHub across: the component

## What to build

Revising Set 1760's Q2, Set 1768's Q36 moved GitHub across as a component
rather than keeping it in-process Rust over `gh`.

- **The WIT `forge` world**: exports one function per trait operation from
  task 01, failing with the variants `no-pull-request`, `not-logged-in`,
  `no-remote` and `refused(string)`. `Ours` stays the host's.
- **The GitHub component** in a crate under `plugins/`, built like the
  Activities' components and bundled as the built-in plugin's code. It speaks
  REST and GraphQL to `https://api.github.com` over WASI HTTP:
  - the stack walk reads the pull requests endpoint, `isCrossRepository`
    included, so a fork is still refused at take-up;
  - gists are created and deleted;
  - repositories are created;
  - `verify` reads `X-OAuth-Scopes` off the REST call.
- **The token as a secret**: the server reads the Author's token for this Forge
  out of `secrets.yaml` at call time. It sets it in the component's environment
  for that call only, under the variable the `secrets` capability names. The
  component never learns where the file is, and nothing is returned to the
  viewer.
- **Hosts added to a Forge** (task 02's setting) widen its outbound grant, so a
  GitHub Enterprise host is reachable. The component derives the API base from
  the host: `https://<host>/api/v3` and `/api/graphql`.
- **A host-side outbound redirect** for tests: a seam mapping a granted host to
  a local address. A test's HTTP stand-in for GitHub is reached through it with
  the grant unchanged.
- **The component behind the trait for live use.** The `gh` implementation
  stays only as the suite's stand-in until task 08.

## Acceptance criteria

- [ ] The component's operations pass their own tests against an HTTP
      stand-in. The tests cover each error kind, a stack walked both ways, and
      a fork refused.
- [ ] A component asking a host outside its grant is refused, and the token is
      in its environment only for the call it was set for.
- [ ] On a real Repo, by hand: Share publishes a gist, **Create repo** makes a
      repository, the wrap-up watches checks and comments, and the Target
      field reads a URL out of a Brief.
