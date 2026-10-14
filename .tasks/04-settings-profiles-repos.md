# 04. Settings, profiles and repos

## What to build

Read `docs/copy.md` first. Then bring the settings page into line with it:
every pane — git author and GitHub token, remote access with its tailnet,
devices, linking and the workbench key, the sandbox and its binds, languages
and the build caches, MCP servers, instructions, desktop, cleanup, at-once —
with the agent profile list and its login card, and the repo list with its
open and create cards.

The settings panes carry the longest copy on screen: paragraphs of three and
four sentences explaining a control's mechanism. The guide's rule is one
sentence, two at most, beside a control, with anything longer behind a
disclosure on the pane or in a linked document. Cut to that, and keep what
the human needs to know behind the disclosure rather than losing it. Apply
the rest as the earlier tasks did: casing, Title Case headings, button verbs,
confirmations as a question with one sentence of consequence and the verb
against Cancel, destructive presses that say what goes, empty states that
name what fills them.

The panes also draw refusals and `trouble` lines that come from the server as
they are; those strings are task 06, and this task leaves them alone while
making sure the surround reads correctly once they are sentences.

Nothing behaves differently, and the web suite's assertions move with the
strings.

## Acceptance criteria

- [ ] Every string on every settings pane, the profile list and login card, and the repo list and its cards follows `docs/copy.md`, and no explanation beside a control runs past two sentences without a disclosure.
- [ ] Pane and section headings are Title Case; everything else is sentence case; every confirmation is a question, a consequence and the verb.
- [ ] The web suite passes with its assertions updated, and no behaviour changed.
