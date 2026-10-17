# 04. Edits, verification and removal relayed home

## What to build

A mirror stops being read-only in the Git switcher: what is pressed on it is
put to its home device as that device's own press, the way a mirrored Profile's
edit and removal already are (`mirroring::edited` / `removed`).

- **A new Author is this device's own** — Add author writes into this device's
  files, as it does today.
- **An edit of a mirror is relayed home** as the ordinary settings edit of that
  Author there — name, email, display name and the token, the token's
  verification run and answered from the home device. The pane says which
  machine the press goes to ("This author is at home on B. Saving or removing
  it here changes it there", in `docs/copy.md`'s voice), and the mirror
  redraws off the refreshed row rather than off what was typed. A token set
  this way travels back to this device only as task 03's copy.
- **Remove on a mirror is relayed home** and checks that device's Repos only —
  its own refusal naming its own Repos comes back as it stands. A member's Repo
  left pointing at a mirror that has gone refuses its next Start by name there,
  under 02's vanished-Author rule; asking every member first was turned down.
- **A home that is not answering refuses the press by name**, as a relayed
  Profile edit is refused.

## Acceptance criteria

- [ ] Editing a mirror's email from the member changes it in the home device's
      `config.yaml`, and the mirror redraws from the refreshed row.
- [ ] Setting a mirror's token from the member saves it at home and shows the
      verification the home device answered.
- [ ] Removing a mirrored Author a home Repo points at is refused naming that
      Repo; one no home Repo points at is removed, and the member's Repo that
      pointed at it refuses its next Start by name.
- [ ] With the home device not answering, saving or removing a mirror is
      refused naming that device.
