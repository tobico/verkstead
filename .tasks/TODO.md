# Intentional credentials

GitHub auth for agent sessions today is whatever happens to sit in the service
home: sandboxes get the host's `~/.config/gh` and `~/.gitconfig` bound in
read-only, and `gh` inside them is not reliably authenticated. This feature
makes credentials intentional: a token in `secrets.yaml` and the git author in
`config.yaml`, both living in the Data Directory beside the database, entered
through a new `/settings` page, and handed to each sandbox as environment
(`GH_TOKEN` plus `GIT_CONFIG_*`). The home-directory binds go entirely, the
server's own `gh` uses the same token, and along the way the config surface is
simplified: one `--data-dir` replaces `--database` and `--state-dir`, with the
database at a fixed name inside.

## Tasks

- [x] 01: One Data Directory — [details](01-data-directory.md)
- [x] 02: The token reaches the sandbox — [details](02-token-into-sandbox.md)
- [x] 03: Git wired by environment — [details](03-git-by-environment.md)
- [x] 04: The server's own gh uses the token — [details](04-server-gh-token.md)
- [ ] 05: The settings API — [details](05-settings-api.md)
- [ ] 06: The /settings page — [details](06-settings-page.md)
- [ ] 07: Fold repos and profiles into /settings — [details](07-fold-in-repos-profiles.md)
