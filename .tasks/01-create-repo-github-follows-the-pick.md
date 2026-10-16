# 01. Create repo's GitHub follows the picked Author

## What to build

Stage 01 made Create repo commit, `gh repo create` and push as the Author picked
on the card (`Gh::as_author` on the pick). Two things still read the wrong
Author or none:

- **The card's GitHub tick** is drawn off the settings' single first-Author
  `github_token` standing, not the picked Author's. It should follow the
  picker: with two or more Authors, the tick is drawn (and starts on) only when
  the picked Author has a token, and the *a remote is needed* sentence stands
  where the picked Author has none. With one Author nothing changes on screen.
  The settings view already carries every Author with its token's standing.
- **The server's guard.** A create sent with `github: true` for a picked Author
  with no token currently runs `gh` with no `GH_TOKEN`, which falls back to the
  host's own `gh` login — a write in an account nobody chose. Instead, the
  repository is made locally, **no `gh` is run**, and the answer is the one a
  create GitHub didn't finish already gets (`Created::MadeWithoutRemote`), with
  a reason saying that author has no GitHub token. (Decided in planning: make
  locally and say why, rather than refuse the whole create.)

Strings follow `docs/copy.md`.

## Acceptance criteria

- [ ] With two Authors, picking one without a token draws the remote sentence and no tick; picking one with a token draws the tick, on; switching the pick between them redraws accordingly.
- [ ] With one Author the card reads exactly as before.
- [ ] A create sent with `github: true` for a token-less Author makes and registers the repository, runs no `gh` (proven with a stand-in `gh` that records being called), and answers `MadeWithoutRemote` with the reason.
