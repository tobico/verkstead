# 01. One Data Directory

## What to build

Replace the two entangled location options — `--database` / `VERKSTEAD_DATABASE`
(a file, whose directory everything else is derived from) and `--state-dir` /
`VERKSTEAD_STATE_DIR` — with a single `--data-dir` / `VERKSTEAD_DATA_DIR`. The
directory holds everything Verkstead makes: the database at the fixed name
`verkstead.db`, and the worktrees, installed skills and handoff directories that
the state dir holds today. The default is the working directory, so a dev run
of `verkstead serve` behaves exactly as it does now.

The old options are removed outright, not deprecated: Verkstead is pre-release,
and the grilling settled on a clean cut. The NixOS module drops its `database`
option and passes `--data-dir` pointed at its state directory — default installs
are unaffected, because the database there is already `verkstead.db` inside that
directory.

The vocabulary follows the flag: the glossary term *State Directory* is renamed
*Data Directory* across `CONTEXT.md` and the rest of the docs, with the old term
moved into the entry's avoid-list.

## Acceptance criteria

- [ ] The server starts with only `--data-dir`, and with nothing at all in a
      dev run, creating the directory and `verkstead.db` inside it
- [ ] `--database`, `VERKSTEAD_DATABASE`, `--state-dir` and
      `VERKSTEAD_STATE_DIR` are gone from the CLI, the NixOS module and the
      docs; the module's `database` option is removed and its unit passes
      `--data-dir`
- [ ] The vm test passes with the module's new invocation
- [ ] The glossary names the concept *Data Directory*, and no doc still uses
      *State Directory* as the term
