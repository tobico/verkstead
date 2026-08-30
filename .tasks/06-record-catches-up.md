# 06. The record catches up

## What to build

The project's record states, in several places, that sandbox binds and
watched paths are configured "in the environment at installation" and never
anywhere the workbench can reach — a rule settled 2026-08-20 and now
deliberately revised (settled 2026-08-30, grilling configurable-paths).
Rewrite the record so it states what is true after tasks 01–05, keeping each
document's own voice and the settled-date idiom the design doc uses.

What the record now says, in short: settings-held binds and watched paths
exist so a standalone install is configurable entirely from the workbench;
the two sources union; installer entries are read-only on the page; settings
entries are re-read per use, never fatal, and reported on resolution; on a
nix install a settings entry the hardened unit cannot see is saved, reported,
and functionless until the installer widens the unit's namespace — which is
why the module keeps its build-time assertion that `watchedPaths` is
non-empty and keeps building `BindPaths` from its own options.

The places that state the old rule, found while grounding:

- `docs/design/verkstead.md` — the Sandbox configuration bullet and the
  build-cache bullet that calls itself "the one deliberate exception"
- `CONTEXT.md` — the **Sandbox Configuration** and **Watched Path** entries
- `docs/adoption.md` — the `watchedPaths` / `sandboxBinds` explanations
- `nix/module.nix` — the option docs and comments
- The module docs of the server's sandbox and watched-path sources
- The settings UI copy where it restates the boundary (the build cache's)
- `docs/development.md` — `--watched-path` described as the one required flag

Sweep for stragglers rather than trusting this list; the phrases worth
grepping for are "installer", "installation" and "boundary" near "bind" and
"watched".

## Acceptance criteria

- [ ] No document, option doc, module doc or UI copy still claims binds and
      watched paths are the installer's alone to configure
- [ ] The design doc records the revision with its settled date, and
      CONTEXT.md's vocabulary entries describe the settings surface
- [ ] The nix module still refuses an empty `watchedPaths` at build time, and
      `development.md` no longer calls `--watched-path` required
