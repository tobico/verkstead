# 03. The instructions text

## Goal

One text on the settings page reaches every session, whatever harness runs it:
written into the built root as the file that harness reads as its global
instructions, and appended to the prompt as a section where a harness has no
such file. The human's own global `CLAUDE.md` does not travel; this is what
takes its place.

## Decisions in force

- **One text for every Profile**, on the settings page, kept in `config.yaml`
  beside the other things Verkstead is told rather than finds. Rejected: a text
  per Profile — the human wanted one place.
- **A file in the root where the harness has one.** `.claude/CLAUDE.md` for
  Claude, `.codex/AGENTS.md` for Codex, the config directory's `AGENTS.md` for
  OpenCode. Chosen over a prompt section for all four because the roots exist
  and a file is where each harness's users expect global instructions to be.
- **A prompt section where it has none.** Grok Build's documentation names no
  global file as of this roadmap; its sessions get the text as a section of the
  prompt, under a heading that says where it came from. The same fallback
  serves any harness whose file is unknown or moves.
- **An empty text is no file and no section.** Nothing is written for a setting
  nobody typed.
- **Read at session start**, like the rest of `config.yaml`: a change on the
  settings page applies to the next session and a running one keeps what it
  started with.
- **The Repo's own instructions are untouched.** A `CLAUDE.md` or `AGENTS.md`
  in the Worktree is the Repo's and is read as it always was; the settings text
  sits above it the way the human's global file used to.

## Proposed tasks (provisional)

1. **The setting.** `instructions` in `config.yaml`, a card on the settings
   page with a text box, the settings API round trip. AC: saved text reads back;
   an absent key is an empty text; the page says what the text reaches.
2. **The file in the root.** Written for Claude, Codex and OpenCode as each
   root is built. AC: the file is inside with the text verbatim; an empty
   setting writes no file; the boundary suites assert it.
3. **The prompt section.** For Grok, and for any harness with no file. AC: the
   section is in the prompt only where no file was written; its heading names
   the settings page.
4. **The docs.** CONTEXT.md gains the term; the adoption doc says where a global
   `CLAUDE.md` went and what replaces it.

## Re-verify at start

- Assumes stages 01 and 02 landed and every harness has a built root to write
  into.
- Check Grok Build's documentation for a global instructions file before
  building the fallback; if one has appeared, it is a file like the others.
- Assumes the settings page still saves `config.yaml` sections through one API
  and that a section with no key still reads as nothing configured.
