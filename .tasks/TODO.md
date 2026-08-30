# Configurable paths

Make sandbox binds and watched paths configurable in the workbench settings,
in addition to the CLI flags and environment variables the nix module keeps
using. The goal is standalone use outside NixOS: a bare binary configurable
entirely from the /settings page, no flags or env vars required. The two
sources union; settings entries are re-read per use and never fatal (saves
always land, the page reports per-entry whether the server can see it, an
unresolvable bind is skipped with a logged warning); CLI/env entries keep
their fail-fast startup checks and show on the page read-only.

## Tasks

- [ ] 01: Settings-held sandbox binds reach sessions — [details](01-settings-sandbox-binds.md)
- [ ] 02: Settings-held watched paths widen the boundary — [details](02-settings-watched-paths.md)
- [ ] 03: The settings API tells and takes Paths — [details](03-paths-settings-api.md)
- [ ] 04: The Paths card and pane — [details](04-paths-card-and-pane.md)
- [ ] 05: Per-repo binds on the Repo's pane — [details](05-per-repo-binds-pane.md)
- [ ] 06: The record catches up — [details](06-record-catches-up.md)
