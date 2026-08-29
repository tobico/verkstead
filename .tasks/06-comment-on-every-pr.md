# 06. One click to every pull request

## What to build

The whole one-click flow: a single **Share to pull request** press builds the
file, publishes it (task 04), and posts **one comment on every pull request
the Conversation holds** — the work's own repo's and each companion's, as the
store records them. Offered only where at least one pull request is recorded.

The comment carries:

- The **link**: `viewer-url#<gist-id>` when the share viewer URL setting is
  configured, the plain gist (or fallback) URL when it is not.
- The **itemized summary**, settled during grilling: the Brief's first line,
  the Question Sets by title, and the commits by subject with their
  files/+/− stats.

Comments only — the PR description is never edited. A re-share is a fresh
snapshot, a fresh publish and a fresh comment on each PR; nothing earlier is
edited or deleted. A PR the comment could not land on (gone, no permission)
is reported by name against the ones that succeeded, not silently swallowed.

## Acceptance criteria

- [ ] One press results in a comment on every recorded pull request, each
      carrying the link and the itemized summary.
- [ ] With no viewer URL configured the comment links the published file
      directly; with one configured it links through the viewer.
- [ ] Sharing twice leaves two comments and edits nothing; a partial failure
      names the PR that missed out.
