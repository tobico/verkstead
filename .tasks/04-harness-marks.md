# 04. The harness marks

## What to build

Each harness gets its brand mark, drawn beside the reading everywhere JSX can
draw one: the Agent run timeline card head, the details pane header, the status
button's running line, and the Brief pane's three pairing facts. (The pickers
get theirs in task 05 — a native `<option>` cannot hold an SVG.)

The marks are the lobehub icon set's, copied into the repository rather than
depended on (the human's call — the React package cannot run in this SolidJS
app, and its source carries the SVG inside each component; the static-svg
sibling package publishes the same art as plain files). Four marks, MIT
licensed, attributed to lobehub/lobe-icons in a source comment:

- Claude Code → `claude` in its **color** variant (color where lobehub has it
  was the pick).
- Codex → `codex` (mono — lobehub has a Codex mark of its own; no OpenAI
  stand-in).
- Grok Build → `grok` (mono).
- OpenCode → `opencode` (mono).

Mono marks render like the existing icon component — inline SVG, 1em-ish,
`currentColor` — so they sit in whatever ink surrounds them; the color mark
keeps its own fills. One small component maps an agent type to its mark, beside
the shared harness vocabulary from task 02.

An event with no recorded harness draws no mark, exactly as it composes no
harness word.

## Acceptance criteria

- [ ] All four harnesses draw their own mark beside the reading on the timeline
      card, the details pane header, the status line and the Brief facts;
      Claude Code's is the color variant, the rest follow the surrounding ink.
- [ ] Events with no recorded harness draw text only, with no gap where a mark
      would have been.
- [ ] The SVGs live in the repository with lobehub attribution and license
      noted; no new npm dependency.
