# 03. The resolution-strategy setting

## What to build

Make how a conflict is resolved configurable: **merge** (the default) or
**rebase**, as one global setting with an optional per-Repo override.

The global half lives in `config.yaml` under the Data Directory, beside the
build cache and share viewer settings, and follows that file's whole ethos: an
absent key, an absent file and an unparseable one all mean merge — a human
should never have a worse experience for not having checked the settings. The
per-Repo override is a fact about a registered Repo, so it lives in the store
beside the Repo and is edited from that repo's row on the settings page; unset
means the global answer.

The settings page offers both — the global picker, and per-Repo rows offering
*use the global setting / merge / rebase* — and says plainly beside rebase what
it costs: the branch is force-pushed, which rewrites what reviewers saw and
breaks any stage stacked on it. The warning is the page's, said where the
choice is made rather than found by a broken stage weeks later.

Thread the resolved strategy into task 02's feedback: the dispatch reads the
override, falls back to the global, falls back to merge, and the session is
told which to do. A rebase instruction tells the session to rebase onto the
base branch and force-push with `--force-with-lease`.

## Acceptance criteria

- [ ] With nothing configured the behaviour is exactly task 02's: merge.
- [ ] A per-Repo override wins over the global setting; either can say merge
      or rebase; clearing the override falls back to the global.
- [ ] The settings page saves and reads both back, and the rebase choice
      carries the force-push warning beside it.
- [ ] The fix session's feedback names the strategy the resolution ran under.
