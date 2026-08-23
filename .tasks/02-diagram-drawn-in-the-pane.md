# 02. Diagram drawn in the pane

## What to build

Mermaid fences in a commit's summary render as Diagrams in the commit pane,
the way Set pages already draw them: the server's markdown rendering leaves
`pre.mermaid` blocks, the client draws them on mount and redraws on a
colour-scheme flip, and an unparseable diagram degrades to its readable source
text. The pane should only pay the mermaid bundle when the summary actually
holds a diagram — the Set page's server-computed flag is the pattern.

The pane's contents nav grows a Summary entry above the Diff section, so the
one left-hand table of contents jumps to the summary, to the diff, and to the
diff's files as it already does.

## Acceptance criteria

- [ ] A mermaid fence in a summary is drawn as a diagram in the commit pane,
      correct in both light and dark, and redrawn when the scheme flips.
- [ ] A fence mermaid cannot parse stays as its source text, not an error
      rendering.
- [ ] The contents nav lists Summary above Diff and jumps to both; a commit
      without a summary lists the diff's files alone, as today.
- [ ] A pane whose summary holds no diagram never loads the mermaid bundle.
