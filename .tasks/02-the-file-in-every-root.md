# 02. The file in every root

## What to build

As each Built Root is built, the settings text is written into it as the file
that harness reads as its global instructions — so a session starts with the
text already in front of it, having asked nothing and read no prompt section.

A file rather than a prompt section for all four, because the roots exist and a
file is where each harness's own users expect global instructions to be. Each
harness names its own, and all four are inside a directory a root already
builds:

| Harness | The file, said from HOME |
| --- | --- |
| Claude Code | `.claude/CLAUDE.md` |
| Codex | `.codex/AGENTS.md` |
| Grok Build | `.grok/AGENTS.md` |
| OpenCode | the config directory's `AGENTS.md` |

Grok Build's is the one that moved since the roadmap was staged: its
documentation now says it reads global rules in its own directory, taking an
`AGENTS.md` there before it reads anything in the repository — so it is a file
like the other three, and the prompt-section fallback the brief planned for it
is not built at all.

**The text verbatim, and nothing else in the file.** No heading, no line saying
where it came from: what the human typed is what the harness reads.

**Written rather than joined**, exactly as each root's configuration file is:
it is Verkstead's own file, it is never the account's, and nothing of it is
written back. Read off the settings at the moment the root is built, so a text
saved on the settings page reaches the next session and a running one keeps
what it started with.

**An empty setting writes no file.** Nothing is left in a root for a setting
nobody typed, and a root then holds exactly what it held before this stage.

A Conversation Terminal's root gets the file on the same terms, the roots being
built by the one path.

The Repo's own `CLAUDE.md` or `AGENTS.md` in the Worktree is the Repo's and is
read as it always was. This sits above it, the way the human's global file used
to.

## Acceptance criteria

- [ ] With a text configured, a session under each of the four harnesses finds
      that harness's own file inside its root, holding the text verbatim — and
      a `CLAUDE.md` or `AGENTS.md` in the Worktree is untouched and still read.
- [ ] With no text configured, no such file is in the root at all, and the
      root's contents are exactly what they were before this task.
- [ ] The boundary suites assert both states on all three platforms, including
      the ones that today assert a Claude root has no `CLAUDE.md` in it.
- [ ] The file is Verkstead's own end to end: nothing of it is joined from the
      account, nothing of it is written back as a session ends, and a session
      that rewrites it leaves the account's own files alone.
