# 03. Card grows the snippet

## What to build

The commit's Timeline card keeps its subject and +/− counts and adds a clamped
snippet of the summary's prose under them, the way document cards clamp to
about five lines. The snippet is prose only: diagram fences are skipped before
the clamp, so a summary that leads with its diagram — which the skills ask
for — never fills the card with fence source. The glance the diagram gives
belongs to the opened pane; the card's snippet is the first lines of what the
summary says.

Cards of commits without a summary are unchanged, and nothing marks the
absence.

## Acceptance criteria

- [ ] A summarized commit's card shows a prose snippet clamped like a document
      card, under the subject and counts.
- [ ] Mermaid fence source never appears in the snippet, wherever the fence
      sits in the body.
- [ ] A summaryless commit's card is byte-for-byte what it is today.
