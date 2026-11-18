# 13. GitLab

## Goal

The first Forge that is not GitHub, written the way a user would write one:
a plugin in a repository of its own under the human's account, passing
`verkstead plugin check`, installed on the Plugins page from its git URL, and
run against gitlab.com. It is a **demonstration and not a daily tool** — the
human's words: *I'm not actively using gitlab, the support will just be to
demonstrate the capability* — so it implements the operations a Forge must
have and declares the optional ones absent, which is what exercises the paths
stage 12 built for a Forge that lacks them.

When this stage is done a Repo whose remote is at gitlab.com runs a Tinker to
a **merge request**, drawn by that noun on the card and the pane, and the
wrap-up watches it to mergeability: the Share pane has no Publish and says so,
a roadmap on it would leave its chain to the rebases, and **Create repo** stops
at the local repository. The plugin's repository is the Forge template stage
14 points a Build a plugin session at.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Forge* and *Capabilities and
  trust*** — what a Forge declares and exports, and that a capability absent is
  settled rather than failed. Nothing here adds to the interface: a Forge that
  needs a new operation is a change to stage 12's world, not to this plugin.
- **A demonstration, so the minimum that proves the boundary.** The required
  operations against GitLab's REST API: find a merge request by branch and by
  `iid`, open one as a draft against a target branch, details — commits,
  notes, diff notes, pipeline state, `detailed_merge_status`, state — comment,
  list the open ones, landing state, verify a token and its scopes
  (`/personal_access_tokens/self`). Declared: hosts `gitlab.com` plus the
  extra-hosts setting; noun *merge request*; credential helper username
  `oauth2` over the same hosts; tool `glab` with `GITLAB_TOKEN`, lent into the
  sandbox where the machine has it and said absent where it has not; needed
  scopes `api`. **Not declared: `stacks`, `publish`, `create`.** GitLab has
  snippets and merge request dependencies, and a project can be created over
  the API, so each could be written; they are left out on purpose so that the
  Share pane's missing Publish, the chain left to rebases and the local-only
  Create repo are seen working on a real Forge and not only on a stand-in.
- **`checks` is decided at stage start against the API.** A pipeline's status
  is one field on the merge request, so declaring `checks` and mapping pipeline
  jobs to the rollup may be cheap; if it is not, the plugin declares it absent
  and the wrap-up settles the checks watcher at once. `review` maps to diff
  notes and is declared.
- **Target parsing** reads `gitlab.com/<group>/<project>/-/merge_requests/<n>`
  and `!<n>` out of prose, GitLab's own spellings, where GitHub reads
  `/pull/<n>` and `#<n>`; remote matching takes the https and ssh spellings of
  gitlab.com and of each extra host, nested groups included.
- **It lives outside this repository.** A plugin is a repository of its own,
  installed from its URL, and this one is written exactly as a user's would be
  so that stage 14's template is a thing that has been installed rather than a
  skeleton nobody ran. What lands in *this* repository is the proof — a server
  suite that installs the plugin from a local checkout, points a Repo at an
  HTTP stand-in answering as gitlab.com, and drives a Tinker to a merge request
  — and the docs naming where the plugin lives.
- **One token per Forge per Author** (stage 12): the Author gains a GitLab
  field in the Git section once the plugin is enabled, verified through the
  plugin's `verify` export.

## Proposed tasks (provisional)

1. **The plugin repository** — `plugin.yaml` with the `forge` block above, the
   component over WASI HTTP against GitLab's REST API, a README written for the
   next Forge author, passing `verkstead plugin check`. AC: the check passes;
   installed from its URL, the Plugins page lists it under Installed with
   *merge request*, its hosts and its three absent capabilities drawn.
2. **The proof in this repository** — a suite that installs the plugin from a
   checkout, enables it, registers a Repo whose remote is a gitlab.com URL
   answered by an HTTP stand-in, and runs a Tinker through to Wrapping.
   AC: the merge request is recorded and drawn by its noun; the Share pane has
   no Publish and says why; `verkstead done` is accepted once the merge
   request stands and refused before it.
3. **Proven on gitlab.com by hand** — one real Repo, one Tinker, one merge
   request, the wrap-up to mergeable, written up in the plugin's README with
   the API version it was measured against. AC: the README names the date and
   the version, and what `checks` was decided to be.
4. **Docs** — CONTEXT.md's **Forge** entry names GitLab as the second, the
   plugin's URL under docs/, and stage 14's template list points at it.
   AC: the Build a plugin Brief for a Forge names this repository.

## Re-verify at start

- Stage 12 landed: the `forge` world has the operations above and the four
  error kinds, the noun is drawn from the Forge, and the three optional paths
  are gated rather than assumed.
- GitLab's REST API still answers `detailed_merge_status` on a merge request
  and `/personal_access_tokens/self` for scopes; both were read off the
  documentation when this was planned, not measured.
- `glab` is still the GitLab CLI's name and still reads `GITLAB_TOKEN`.
- The Plugins page's install from a git URL still clones over the Author's
  credential where the repository is private, which this plugin's repository
  may be.
- The human has a gitlab.com account to measure against; the hand proof needs
  one and nothing here can make it.
