# 03. Tokens per Forge per Author

## What to build

One token per Forge per Author (ADR-0030, *Forge*; ADR-0025).

- **`secrets.yaml`** keys `author_tokens` by the Author's uuid and then the
  Forge plugin's uuid. A file written before this reads its single token per
  Author as the built-in GitHub's, and the next write is in the new shape.
- **The replicated Author record** (ADR-0028) changes additively (Set 1878,
  Q5). `holds_token` and `token_digest` stay and go on meaning the GitHub
  token, so an older member of the cluster still reads it right. A new map of
  Forge uuid to digest is added beside them. The cross-device copy fetches
  and clears per Forge by that map.
- **The Author as the server and viewer read it** holds a set of Forge uuids
  it has a token for. That set is what a Repo on a Forge reads to know whether
  its Author can write there.
- **The Git settings section** asks for a token per **enabled** Forge under
  each Author, labelled with the Forge's name. It verifies through the Forge's
  verify operation, which answers the scopes it found against the scopes the
  manifest declares it needs. A short token is refused naming the missing
  scopes. Today's `Scopes` reading of `X-OAuth-Scopes` stays inside the GitHub
  implementation.
- **An Author with no token for the Repo's Forge** (Set 1878, Q3) means the
  Forge is not asked. Wrap-up and `verkstead done` treat it as the Forge being
  out of reach. The Repo's row and the Git section say the Author needs a token
  for that Forge. Remove the fallback to the host's own `gh` login, which an
  Author with no token used to read under. Onboarding prefills the token from
  `GH_TOKEN`/`GITHUB_TOKEN` only, and stops asking `gh auth token`.

Words on screen follow `docs/copy.md`.

## Acceptance criteria

- [ ] An installation from before starts with its GitHub token where it was,
      for every Author, and an older cluster member still copies it.
- [ ] Enabling a second Forge draws a second token field under each Author,
      and a token set for it on one device reaches another.
- [ ] A token with scopes short of the declared ones is refused with the
      missing scopes named, and an Author with no token for a Repo's Forge
      gets no wrap-up there, with the row saying why.
