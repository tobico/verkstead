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

- [x] One press results in a comment on every recorded pull request, each
      carrying the link and the itemized summary.

      Built as `POST /api/ui/conversations/{id}/share/comment`: the pull
      requests are read first — a Conversation on none answers
      `NoPullRequest` and publishes nothing — then the share is published
      through the same function the **Publish** row uses, and
      `crate::commenting::on_each` says the comment once per pull request,
      inside the repository that number belongs to and as the token the gist
      was made with.

      **Proved in its parts rather than through the endpoint.** The comment
      body is `verkstead_render::itemized`, tested over a Timeline of a Brief,
      a Set and commits; the fan-out is tested against a `gh` script that
      records what it was asked and where; the endpoint's own guard — nothing
      published where there is nowhere to comment — is tested in
      `crates/server/tests/sharing.rs`. What no test here reaches is the
      endpoint end to end, for the reason that file already records: the press
      composes the share *file*, which is `pnpm build`'s second output, and
      `cargo test` does not wait on it and CI does not run it.

      **What could not be done here**: the same missing `gist` scope tasks 04
      and 05 recorded. No share could be published, so no comment has been
      left on a real pull request. The next session with a gist-scoped token
      should press this once end to end and read what lands.
- [x] With no viewer URL configured the comment links the published file
      directly; with one configured it links through the viewer.

      `crate::sharing::link`, tested both ways and for the viewer URL somebody
      pasted with a fragment already on it. The id is the gist URL's last
      segment, which is what the viewer page reads back out of the fragment.
- [x] Sharing twice leaves two comments and edits nothing; a partial failure
      names the PR that missed out.

      Two goes are two `POST .../issues/{n}/comments` and nothing else — no
      request this module can make edits anything that is already there. A
      pull request the comment could not land on comes back with `gh`'s own
      words against it, the ones after it are still tried, and the row that
      was pressed names it beside the ones that worked.
