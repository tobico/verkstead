# Windows CI under ten minutes

The two Windows legs of the CI workflow take 26 to 35 minutes each, and the
whole workflow is to come under ten. This backlog brings the Windows legs
inside that budget and keeps them there; the Linux `Rust` job, at 28 minutes,
follows in a GitHub issue the last task opens. Measured on the last green run
on `main` before this work (2026-10-28): a leg builds for 8 to 10 minutes and
tests for 17 to 20. The build is the 129 test binaries, each compiled and
linked by MSVC with full debug info — on a four-core machine the workspace's
libraries and binaries take 4m39s and the test binaries 11m38s on top. The
tests are about 4,200 test-seconds a leg on four cores, spread thinly over the
suites: `conversations` is 16% and nothing else is above 8%.

What was settled, and holds for every task: free four-core runners only, with
a larger runner never before the free levers are measured; one pull request,
one commit per lever, each push's leg times read off CI and written into a
ledger kept in the Windows job's comment in `.github/workflows/ci.yml`; a hard
`timeout-minutes: 10` on each Windows leg once the figures are in, with the
typical run near seven minutes because two legs of one run already differed by
two minutes of build; line tables only for every build; Defender exclusions on
the runner; one test binary per crate, bar the seven test files whose safety
argument is being alone in their process; as many legs as the budget needs, up
to ten; fixture work test-side only, with the database and production code
left as they are. Rejected and not to be reopened: sccache for the CI build
(linking is the cost, and the sessions suite counts sccache servers), a
build-once nextest archive shared across legs (free parallel runners make it
cost wall clock rather than save it), and the Linux job inside this work.

## Tasks

- [x] 01: Open the measuring pull request and record the baseline — [details](01-measuring-pull-request-and-baseline.md)
- [x] 02: One test binary each for store, schema and cli — [details](02-merge-store-schema-cli-tests.md)
- [x] 03: One test binary for the server, and the seven that stay alone — [details](03-merge-server-tests.md)
- [x] 04: Defender exclusions on the runner — [details](04-defender-exclusions.md)
- [x] 05: Deal the suite over the legs the budget needs — [details](05-legs.md)
- [x] 06: Try rust-lld — [details](06-rust-lld.md)
- [x] 07: Template repositories for the suites that make one — [details](07-template-repositories.md)
- [x] 08: The hard timeout, the final ledger and the Linux issue — [details](08-timeout-ledger-linux-issue.md)
- [x] 09: The slow session-account tests — [details](09-slow-session-account-tests.md)
- [x] 10: The timeout down to ten — [details](10-timeout-down-to-ten.md)
