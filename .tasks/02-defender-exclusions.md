# 02. Keep Defender out of the CI build tree

## What to build

The Windows runner's real-time antivirus scans every file the build and test
run touch, which is most of the per-test tax that makes I/O-heavy suites run
several times slower than their Linux counterparts. The runner is a throwaway
VM whose only job is this suite, so exercising the code under Defender buys no
fidelity worth the minutes it costs (settled during grilling).

Add a step near the top of the `Windows` job — before the build — that excludes
the checkout/workspace, the Cargo `target` directory, and the temp directories
the suites write in from Defender's real-time scanning, using
`Add-MpPreference -ExclusionPath`. Scope it to what the build and tests
actually touch rather than disabling protection wholesale.

Only the `Windows` job changes; the Linux and macOS jobs are untouched.

## Acceptance criteria

- [ ] The `Windows` job adds Defender exclusions for the workspace, `target`, and the temp directories before the build step runs
- [ ] The exclusions are scoped to the build/test paths, not a blanket disable of real-time protection
- [ ] The `Windows` job stays green
- [ ] The test step is measurably faster than before the exclusion (captured in the PR so task 04 can build on it)
