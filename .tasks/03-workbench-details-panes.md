# 03. The workbench's details panes

## What to build

Read `docs/copy.md` first. Then bring the workbench's details panes into line
with it: the timeline and its rows and headers, the brief, the checks, the
commits and the commit view, the comments and ignored comments, the code pane
and its tree, the output and screen panes, the pull request pane, the share
pane, the backlog and the roadmap, the handoff and the documents, attached
files, and the MCP server card the composer opens.

The same work as task 02, over these panes: casing, Title Case headings,
button verbs, empty states that name what fills them, progress lines that name
what is being read, explanations cut to one or two sentences, the ellipsis
character, no em-dashes, contractions where they read naturally. The timeline
draws lines the server renders as well as its own; the server's are task 06,
and this task touches the viewer's.

Nothing behaves differently, and the web suite's assertions move with the
strings.

## Acceptance criteria

- [ ] Every string on the timeline, brief, checks, commits, comments, code, output, screen, pull request, share, backlog, roadmap, handoff, documents and attached-files panes follows `docs/copy.md`.
- [ ] Pane and section headings on those panes are Title Case; everything else is sentence case.
- [ ] The web suite passes with its assertions updated, and no behaviour changed.
