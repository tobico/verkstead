# 05. The pick at Open repo and Create repo

## What to build

With two Authors configured, a Repo registered from here on is recorded as the
one the human picks. This is what makes the stage's Goal real: a Repo opened
after a second Author is added is recorded as its, and its sessions run as it.
Brought forward from stage 02's first task in planning. Stage 02 keeps the
Create repo card's GitHub tick following the picked Author's token, Create
repo's `gh repo create` under the picked Author, the Repos pane, and every
refusal.

- **Open repo and Create repo each carry an Author picker**, drawn only where
  more than one Author exists, labelled by the display name with the git name as
  fallback. With one Author neither card draws one, and the registration
  records it silently (task 02's rule).
- **The registration records the pick.** With two Authors, the server refuses a
  registration or a create that names no Author, or one that names a uuid that
  is not configured, as a named outcome the card draws. The card does not let it
  get that far.
- **Create repo's first commit is by the picked Author.** A picked Author
  lacking a name or an email refuses the create the way a missing git author
  does today.

## Acceptance criteria

- [ ] With two Authors, Open repo cannot land without a pick, and the Repo
      opened under the second Author runs a session carrying the second
      Author's `user.email` and `GH_TOKEN`.
- [ ] Create repo under the second Author makes its first commit as the second
      Author, and records the Repo as its.
- [ ] With one Author, neither card draws a picker, and both record that Author.
- [ ] The server refuses a two-Author registration that names no Author or an
      unknown uuid, by name.
