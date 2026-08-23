# 03. Git wired by environment

## What to build

Git inside the sandbox configured entirely by injected environment, with the
read-only `~/.gitconfig` bind removed. A second settings file carries the
author:

```yaml
# config.yaml
git_author:
  name: Tobias Cohen
  email: tobi@tobico.net
```

Read from the Data Directory at session spawn, alongside `secrets.yaml`. The
sandbox injects git configuration through the `GIT_CONFIG_COUNT` /
`GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n` environment scheme:

- `user.name` and `user.email` from `config.yaml`, when set
- the credential helper, `gh auth git-credential`, so a plain `git push` over
  HTTPS authenticates with `GH_TOKEN`
- `url.insteadOf` rewrites turning SSH GitHub remotes (`git@github.com:` and
  `ssh://git@github.com/`) into `https://github.com/`, so a repo cloned over
  SSH pushes with the token instead of failing on absent keys

With no author configured, git's own "tell me who you are" error stands — the
settings page (task 06) is where the missing state is surfaced. The NixOS
module's `home` option documentation is rewritten: the home directory no longer
carries credentials or git identity, and the provisioning instructions for
`.gitconfig` and `.config/gh` go.

## Acceptance criteria

- [ ] Commits made inside a session carry the configured author name and email
- [ ] `git push` inside a sandbox whose repo has an SSH GitHub remote
      authenticates over HTTPS with the token
- [ ] No `.gitconfig` and no gh files from the host are visible inside the
      sandbox, and the vm test asserts their absence
- [ ] The module's `home` documentation no longer tells anyone to provision
      credentials into it
