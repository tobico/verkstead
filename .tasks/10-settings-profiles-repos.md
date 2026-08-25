# 10. Settings, profiles, repos

## What to build

Migrate the remaining pages: SettingsPage and Credentials (`settings/`),
ProfileList (`profiles/`), and RepoList (`repos/`). Each gets its colocated
module; the list-page pattern the Repos and Profiles pages share — the same
row/card shapes styled once today — is carried per the grilling's rule: values
that can be CSS variables become variables on the body, and the rest is
duplicated into each page's module rather than shared.

Conventions as settled: camelCase identical in CSS and TS; `editing`/`broken`
and friends through the module object; the forms' status lines are the task-03
components with per-page refinement via their `class` prop; comments move and
may be rewritten; dead rules die.

Tests (`settings`, `profiles`, `repos` and any other naming these classes)
import the modules.

## Acceptance criteria

- [ ] All four components have their modules; the settings, credentials,
      profiles and repos blocks are gone from `main.css`.
- [ ] Both list pages, the forms, the modals they open and the broken-profile
      states are visually unchanged.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
