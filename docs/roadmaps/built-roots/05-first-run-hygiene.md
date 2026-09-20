# 05. First-run hygiene

## Goal

A first run on a fresh Windows machine fails only where it has to, and says so
by name. The desktop app's `claude.exe` is refused as the desktop app on the
wizard's row and at session start, with the npm install named instead. The
desktop's log holds no workbench key. A tag whose manifest disagrees with it
fails to build. The token field says which scopes a token needs. And four small
things that read as broken are put right: the `containers\standing` warning at
every start, the mixed-separator prompt path, the log's mojibake, and the
Issue 5 note.

Depends on no other stage, and carries the release gate that matters before
`v0.1.2` is tagged.

## Decisions in force

- **The desktop app is recognised by shape and refused by name**, and no
  process is run to find out: the resolved `claude` sits under
  `%LOCALAPPDATA%\AnthropicClaude` or beside Squirrel's `Update.exe`. The
  wizard's row says it is the desktop app and names the CLI install; a session
  started under such a Profile is refused with the same line in the log and the
  Notice. Rejected: probing every harness with `--version` — an Electron binary
  probed may open a window, and ADR-0016 kept the wizard to reads.
- **The desktop logs its listening line without the key.** ADR-0015 says where
  the link is handed out is a fact about the install — the tray's Open for the
  desktop, the startup line for the daemon — and the desktop was doing both. The
  daemon's `serve` keeps the key in its line. Rejected: a `workbench-url` verb,
  which ADR-0015 already rejected; and redacting everywhere, which leaves a
  headless host no link at all. The Windows suite asserts a session cannot read
  the desktop's log file, beside its existing attempt on the human's own files.
- **The release gate compares the manifest to the tag** in the release
  workflow, before anything is built, with the hyphen rule `releasing.md`
  already states for a rehearsal tag. The manifest is not bumped here — the
  release procedure bumps it — and `v0.1.1` stays as it shipped: a tag cannot be
  re-released, and the next is `v0.1.2`.
- **The token note names its scopes.** A classic token wants `repo` and
  `workflow`, and `gist` for shares; a fine-grained one wants Contents, Pull
  requests, Issues and Workflows to write and Actions to read. The push itself
  runs inside the sandbox, so a refused push stays the session's to report; only
  the note is Verkstead's.
- **`containers\standing` is skipped by the sweep**, as its own doc comment
  already claims and its code does not.
- **The prompt path is composed one way.** `under` puts a forward slash on
  purpose and `join` puts a backslash after it; the file is composed the way the
  directory was, and the test helper that bakes the mixed form is corrected
  rather than matched.
- **The desktop log opens with a byte-order mark**, on a fresh file and after
  each roll, so the code-page viewers Windows opens it in read its em-dashes.
  Rejected: taking the em-dashes out of the messages.
- **Parked and out of scope**, written down so nobody reopens them: Issue 5, the
  uninstall entry in HKLM, whose cause wants a Windows machine and whose package
  is already per-user; Issue 7, `msiexec` under a restricted token, which is
  Windows Installer's; Issue 8, synthetic Enter in the Conversation Terminal,
  which plain Enter leaves to xterm.js; and the doubled separator in the npm
  path, which is npm's own shim.

## Proposed tasks (provisional)

1. **The desktop app refused.** AC: a `claude` resolved beside `Update.exe` or
   under `AnthropicClaude` draws the wizard's row as the desktop app with the
   install named; a session under it stops with a Notice saying the same.
2. **The key out of the desktop log.** AC: the desktop's log carries the
   listening line without `?key=`; `serve`'s stdout still carries it; the
   Windows suite reads back a refusal on the log file from inside a session;
   ADR-0015 amended.
3. **The release gate.** AC: a tag whose `cargo pkgid` version disagrees fails
   the workflow at its first job with a line naming both; `releasing.md` no
   longer says nothing checks this.
4. **The token note and the docs.** AC: the wizard's token note and the
   settings page name the scopes; the adoption doc's Windows section carries
   the Issue 5 note.
5. **The three small fixes.** AC: no warning for `standing` at startup, with a
   test; the prompt path has one kind of separator on Windows; a fresh log and
   a rolled one both open with the mark, with a test in the desktop suite.

## Re-verify at start

- Assumes the wizard's rows are still derived from a probed `Machine` and that
  a harness is still resolved on the PATH by `sandbox::standing`.
- Assumes the desktop still starts the server through `run_on_keyed` and that
  the listening line is still the server's.
- Assumes the release workflow still has a first job every other job needs.
- Check the Claude Code desktop app's install layout on a current build before
  fixing the shape it is recognised by.
