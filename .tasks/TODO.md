# Scoped Nudges

Fix the viewer's two data-freshness defects at their shared root. Today every
Nudge triggers an unfiltered refetch of every active query — up to twice a
second while a session talks — and seven queries never set `reconcile`, so
their DOM rebuilds wholesale on each refetch: dropdowns snap back (while
submitting the old choice), spinners restart, folds close. Per ADR-0009,
Nudges become typed and scoped (notify-only), the 10-second poll retires in
favour of reconnect/visibility catch-up, the reconcile-or-static rule of
ADR-0005 becomes unskippable behind a query wrapper and a lint wall, and the
transcript gains an incremental read so a running session no longer
re-downloads its whole record on every batch.

## Tasks

- [x] 01: Query wrapper and lint wall — [details](01-query-wrapper-and-lint-wall.md)
- [x] 02: Controlled selects and DOM-identity tests — [details](02-controlled-selects-and-dom-identity-tests.md)
- [ ] 03: Typed Nudge on the wire — [details](03-typed-nudge-on-the-wire.md)
- [ ] 04: Scoped invalidation, catch-up, poll retired — [details](04-scoped-invalidation-catch-up-poll-retired.md)
- [ ] 05: Incremental transcript fetch — [details](05-incremental-transcript-fetch.md)
