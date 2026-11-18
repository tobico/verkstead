# 07. Dependencies

## What to build

Task 01 parses the `dependencies:` list. This task resolves it at load against
what is installed and enabled (built-ins included), by uuid and **version
specifier**. A dependency that is **missing**, **disabled**, or installed
outside the specifier (**outdated**) puts the dependent in trouble: its row says
which dependency and why, by the display name the dependency entry carries, and
the dependent counts as disabled. Resolution is re-run whenever the installed
set or any enabled flag changes, so fixing the dependency clears the trouble.

Where the dependency entry names a repository, the dependent's row offers the
fix: **Install** for a missing one, **Update** for an outdated one. Both go
through the same consent card as **Add a plugin…**. A disabled dependency is
fixed by ticking it, and the row says so.

## Acceptance criteria

- [ ] A fixture plugin depending on an absent one is in trouble naming it, and
      installs it from its declared URL through the consent card, after which
      the dependent loads.
- [ ] A dependency on `^2` over an installed `1.4.0` reads outdated and offers
      the update.
- [ ] Unticking the dependency puts the dependent in trouble as disabled, and
      ticking it again clears that.
