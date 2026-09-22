# 05. gh on an Intel Mac comes from GitHub's release

## What to build

An Intel Mac has no Homebrew to `brew install gh` with, and GitHub ships no
install script — but every release carries `gh_<version>_macOS_amd64.zip`,
with the binary at `bin/gh` inside it. The Intel planner's gh row becomes a
unit run as the user, in the vendor units' shape: one shell line that resolves
the current version from where `https://github.com/cli/cli/releases/latest`
redirects to (the tag is the last path segment, `v` and all), downloads that
version's `macOS_amd64` zip, unpacks it somewhere temporary, and puts `bin/gh`
in the home's `.local/bin`, executable. The unit's *lands* is `~/.local/bin`,
so the directory is written to `session_path` on success as every vendor unit's
is. The version is resolved at install time rather than written into the
binary — a pinned one goes stale with every Verkstead release, and the human
pressing Next wants today's gh.

A failure reads the way every as-the-user unit's does: the first line of
stderr under the row, and the hint screen beneath it with the link task 04
gave the row — so a Mac that cannot reach GitHub is told what to download by
hand.

Test it the way the vendor installers are tested: a stub `curl` on the stated
machine's `PATH` that answers the redirect with a location and the download
with a zip holding a stub `gh` (the suite's real `zip` or `unzip` named by
absolute path, as the suites already name `cp`), and a stub whose download
fails with a line of its own. Assert the row goes present at the next probe
with `~/.local/bin/gh` resolved, and that `session_path` gained the directory.

## Acceptance criteria

- [ ] Ticking gh on an Intel Mac raises no dialog, lands `gh` in
      `~/.local/bin`, writes the directory to `session_path`, and the row is
      present at the next probe with that path resolved — in the installing
      suite with `curl` stubbed.
- [ ] A download that fails puts curl's first line under the gh row and the
      hint screen's link beneath it; nothing is left in `~/.local/bin`.
- [ ] The version is read from the releases redirect at install time; no gh
      version number appears in the source.
