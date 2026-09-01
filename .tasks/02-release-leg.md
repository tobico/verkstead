# 02. A tag ships the AppImage

## What to build

The release workflow gains its Linux desktop leg, and the `publish` job is
widened once so that this artifact and the two stages 04 and 05 will add reach
the Release.

**The leg** builds on the shared viewer artifact the CLI legs already wait for —
the viewer is embedded at compile time, so the desktop binary needs it on disk
just as they do — installs the same toolkit development packages `ci.yml`
installs for the workspace build, since the runner image carries none of them by
declaration, builds the desktop binary and calls task 01's bundling script. Then
it asserts what the CLI legs assert of their own binaries: the artifact runs,
and the viewer is inside it rather than the 503 an empty embed serves. It
uploads the AppImage as a workflow artifact and attaches nothing itself, which
is the pattern the CLI legs already follow.

**`publish`** currently fetches `verkstead-*`, insists on exactly four, uploads
`binaries/verkstead-*` and waits on the build matrix alone. `Verkstead-x86_64.AppImage`
matches none of that, and a Windows `.exe` named in the CLI scheme would match
the pattern and break the count. Widen it for all three desktop artifacts at
once: the four CLI binaries stay counted as a set of their own, so a missing one
still fails the release, and the desktop artifacts are fetched and counted
beside them by name — `publish` names the desktop assets it expects, so a
naming mistake in stage 04 or 05 fails the release rather than quietly shipping
one asset short. `publish` waits on the desktop legs as well as the build
matrix. `manifest` is not this task's: it hashes the four CLI assets for the
flake, and four is the right number there and stays so.

**And a way to run this without tagging.** The workflow fires on `v*` tags and
nothing else, so today the only way to exercise a leg is to publish. Add
`workflow_dispatch`, with the jobs that write — the Release and the manifest
commit — gated on the ref actually being a tag. That is what makes the leg
provable before the first real release, and it costs one condition on two jobs.

If a leg dies within seconds having run no steps at all, that is this account's
Actions billing rather than anything in the workflow.

## Acceptance criteria

- [ ] A `workflow_dispatch` run builds the AppImage, runs the leg's own
      assertions and uploads the artifact, while creating no Release and
      committing no manifest.
- [ ] A tag attaches `Verkstead-x86_64.AppImage` to the Release alongside the
      four CLI binaries.
- [ ] The CLI binaries are still counted as a set of their own, so a missing one
      fails the release; and a desktop artifact that was built and not attached
      fails it too.
- [ ] `manifest` still hashes exactly the four CLI assets and still runs after
      the Release.
