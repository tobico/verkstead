# 05. Verkstead's own git fetches as the Repo's Author

## What to build

Verkstead's own git on a Repo still runs bare: the fetch before every cut and
the drawing fetch (`worktrees::fetch` / `fetch_within`, with
`GIT_TERMINAL_PROMPT=0` and no credential helper) and the checks' `ls-remote
origin` that asks which commit a branch stands at. A private repository under
the second Author's account would not fetch with the host's credentials.

Move publishing's `git` helper — the one that clears `credential.helper` and
declares a shell-function helper reading `GH_TOKEN` from the environment, never
the URL — somewhere shared, and run every one of those calls through it with the
Repo's Author's token (`Config::author_of` on the Repo's recorded uuid,
`Secrets::author_token`), read at the moment of the call. **An Author with no
token leaves git to the host's own credentials, exactly as today** — no helper
override. Publishing keeps using the same helper.

Out of scope: there is no `git push` of Verkstead's own on a Repo (Create repo's
push is `gh repo create --push`, already as the picked Author since stage 01),
and the peer checkouts' fetches are out of local bundles. Read `Gh::as_author`
for how the `gh` side already does this.

## Acceptance criteria

- [ ] With a stand-in remote that records the credential it is offered (e.g. a local HTTP smart server or a credential-recording helper check), a fetch on a Repo whose Author has a token presents that token and no other Author's.
- [ ] A Repo whose Author has no token fetches with the host's credential configuration untouched.
- [ ] The checks' `ls-remote` goes through the same path; the existing fetch-deadline and kill-the-group behaviour is unchanged.
