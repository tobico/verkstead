# 03. The words

## What to build

Everything that tells a human about the Windows download, rewritten for the
exe, and the one-line note the first NSIS release carries. Read
`docs/copy.md` before writing any of it.

**The release note.** `publish` writes every word of a Release's notes with
`gh release create --generate-notes`. The human chose (Set 1393 Q3) **an
optional input on the release workflow**: a line put above the generated
notes, empty by default. `docs/releasing.md` says when to use it. The first
NSIS release's line says, in one line, that a machine with the msi installs
this exe over it by hand once, and the installer removes the msi.

**The words.**

- **README** — the Windows paragraph names `Verkstead-x86_64.exe`. SmartScreen's
  *More info* / *Run anyway* and *Unblock* stay. *Taking it off Windows again*
  says uninstall rather than "the msi". There is a line for a machine that has
  the msi.
- **`docs/releasing.md`** — the Windows leg's description (NSIS, the include
  script, the silent install, the counts over itself and over the msi; the
  WiX and same-version-upgrade paragraph goes) and checklist item 5 (the exe
  downloaded in a browser).
- **`docs/development.md`** — the Windows pack section: what comes out, where
  it installs, and the include script in place of `msi.mjs`.
- **The site** — `site/index.html`'s download link and `site/site.js`'s
  platform pick.
- **Comments that name the msi**, which all now describe something that is
  gone: across `desktop/` (`electron-builder.yml`, `pnpm-workspace.yaml`,
  `main.ts`, `startup.ts`, `runkey.test.ts`), the workflow (its header, the
  `prepare` version bump's reason), `docs/agents/git-workflow.md`,
  `tools/generate-packaging.sh`, and the two crates'
  (`unseen.rs`, `session_account.rs`). The ADRs and the earlier roadmaps are
  history and stay as written.

## Acceptance criteria

- [ ] `grep -ri msi` outside `docs/adr/`, `docs/roadmaps/` and the
      include script's msi removal finds nothing
- [ ] The site's Windows download resolves to
      `releases/latest/download/Verkstead-x86_64.exe`
- [ ] The release workflow takes an optional note line, and the publish step
      puts it above the generated notes; left empty, the notes are as
      today
- [ ] The README tells a Windows downloader what to do with the exe, and what
      to do on a machine that has the msi
