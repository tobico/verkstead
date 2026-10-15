# 02. Clone from the dropdown

## What to build

Pressing a clonable row (task 01) clones that repository on the device the
dropdown is for, registers it, and lands the draft on it the way a pick on a
registered row does.

**The clone is the named device's own operation**, on a new viewer route on
that device. A browser reaches a member's over the relayed path, as it reaches
Create repo. The press sends the origin URL from the clonable reading and a
parent directory. The device runs `git clone <origin> <parent>/<name>`, where
`<name>` is the origin's last path segment, the same name task 01 drew. git runs
with that device's own credentials, set up the way a session's sandbox sets
them up:

- `GH_TOKEN` from the device's `github_token` in its own `secrets.yaml`, read at
  the moment of the call as Create repo reads it;
- `credential.https://github.com.helper` set to `!gh auth git-credential`;
- the two SSH spellings of github.com rewritten to https with `insteadOf`;
- `GIT_TERMINAL_PROMPT=0` and a null stdin, so a missing credential fails
  instead of waiting.

The clone is then registered through the same call Open repo uses, and the
answer carries the registered Repo.

- **A failed clone registers nothing, leaves no directory behind, and says
  git's own words.** That means git's stderr, not a sentence of Verkstead's. A
  directory left half-cloned is taken back, the way a failed create takes back
  its own.
- **Nothing registers twice.** A clone whose path is already registered answers
  that Repo, the way Open repo answers a registered path.
- **A destination that already exists but is not registered** is refused in
  git's own words, and nothing is registered. The human chose this over
  adopting the existing directory.
- A parent that is missing or not absolute is refused in words, the way Create
  repo's is.
- Check whether a long clone over the relayed path is cut short by any deadline
  on the way, and make sure a big repository's clone is not refused for taking
  time.

**The press.** Where Create repo has a parent remembered for that device in the
browser's remembered settings, the clone goes there straight away. Where none
is remembered, a small card asks for one with the same browse field Create
repo's parent uses, browsing that device's filesystem. Once the clone lands
there, the parent is remembered for that device exactly as Create repo writes
it, so the next Create repo or clone opens in it. While the clone runs, the
press says so and cannot be pressed again. A refusal is drawn in its own words
where the human is looking, and the draft is left as it was.

The landing is the caller's, as it already is for Create repo and Open repo:

- the compose page's own state, where the page is composing;
- a move on a saved draft;
- the Repo the draft-move card sends with the move.

Follow docs/copy.md for every word on screen, including the card and the
in-progress state.

## Acceptance criteria

- [ ] In a two-server suite, cloning a Repo the first server holds, by its
      origin, onto the second registers it there under the origin's last path
      segment and answers that Repo. Asking again for the same path answers the
      same Repo, with nothing registered twice.
- [ ] A clone git refuses (a bad origin, or an existing unregistered directory)
      answers git's own words, registers nothing and leaves no directory it made.
      The device's GitHub token reaches git the way it reaches a session.
- [ ] In the viewer suite, picking a clonable row for another device lands the
      draft on the Repo the clone made there, on the compose page and on a saved
      draft. The path card appears only where no parent is remembered, and
      after the clone that parent is remembered for that device as Create repo
      remembers it.
