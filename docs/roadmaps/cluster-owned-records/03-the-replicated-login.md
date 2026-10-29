# 03. The replicated login

## Goal

A Profile's login is replicated: every device the Profile is bound on reads its
account's login at each launch, at each session's ending and on the refresh
schedule and publishes it where it differs; every device keeps the newest it
has heard of; an unbound device builds its copy from that and starts a session
with every bound device switched off. *Use this machine's own account* on the
card binds this device. The brokenness readings are rewritten for a Profile
with no home: no login anywhere in the cluster, or the harness absent here. The
fetch-from-a-bound-device of 02 and the account mirror's home fetch go.
Demonstrable with two Verksteads: log in on A, start on B with A off; then
bind B with the press, and the card reads one record with two bindings.

## Decisions in force

From [ADR-0028](../../adr/0028-records-are-the-clusters.md); this stage
rewrites the **Account mirror** term in full.

- **Newest write wins on the login**, as ADR-0020 accepted for two machines
  refreshing one OAuth pair, now across more copies: a sign-out that results
  reads as the Profile being broken there and the fix is a login on a bound
  device. Exclusive lending was turned down in ADR-0020 and is not revisited.
- **Three moments publish**: launch, ending and the refresh schedule on a
  bound device; a session's ending on an unbound device publishes the copy's
  login the same way, which is the write-back a home used to take. Watching
  the account directory was turned down.
- **Only a changed file publishes.** A bound device publishes when its file
  has changed since it last wrote or read it — the fingerprint the mirror
  already keeps — never because the file differs from the newest login heard
  of: a bound device launching before it has taken in a copy's refreshed
  login would otherwise publish its stale file with a later stamp, and the
  rotated refresh token would be lost everywhere. The login is stamped with
  the file's own change time, and a newer arrival is written into a bound
  device's account file before its next launch reads it.
- **The login is a secret**, so it travels on the Peer Listener behind the
  Member Gate as the token does, never down `/api/ui/`; only the allowlist of
  files a Built Root is made from travels — the login and the composed
  configuration, with the names ADR-0025 strips.
- **Binding is a press on the card, never automatic.** The press takes the
  harness's account paths as the Profile form does, validates them on this
  device, and writes this device's binding row.
- **A quiet device blocks no start.** A device that has heard a login starts
  from it; one that has heard none, and has no binding, refuses by name saying
  no login has reached it yet.
- **Brokenness per device**: harness absent here; no login anywhere; and on a
  bound device, no login file at the path — the macOS Keychain case — which is
  broken here and runnable nowhere until a login lands.

## Proposed tasks (provisional)

1. **The login record.** The login, with its composed configuration, as a
   per-Profile record on the facility: contents, timestamp, the writing device,
   carried on a gated peer route rather than the table route.
   - A login published on A is held on B and C; the newest of two wins.

2. **Publishing.** A bound device reads and publishes at launch, ending and on
   the refresh schedule; a session's ending on a copy publishes too; a file
   unchanged since this device last wrote or read it is not sent, by the
   fingerprint the mirror already keeps, and a login is stamped with the
   file's change time. A newer arrival is written into a bound device's
   account file.
   - A token refreshed inside a session on B is what A's next session gets,
     and what A's own account file holds afterwards.
   - A launching on A before it has taken in B's refreshed login publishes
     nothing, and B's login is still the newest everywhere afterwards.

3. **The copy.** An unbound device builds its copy under the Data Directory
   from the newest login it holds, written beside and renamed over so a running
   session keeps its file; the 02 fetch goes, and with it the account route.
   - A session on B starts with every bound device off.

4. **Binding from the card.** *Use this machine's own account* with the path
   fields, validated here, writing this device's binding; the card's line
   turns from *runs from a copy* to the path.
   - Binding B makes B publish its own login from the next launch.

5. **Brokenness rewritten.** The three readings above, per device, on the card
   and on every picker row; Start refused by name.
   - A Profile nobody has logged into reads *no login yet* on every device.

## Re-verify at start

- Assumes 02 landed: the record, the binding rows, the copy named by uuid and
  the interim fetch from a bound device.
- Assumes `crates/server/src/mirroring/account.rs` is still the fetch and
  write-back, with its allowlist in `crates/server/src/sandbox/root.rs` (`mirrored`), and the
  SHA-256 fingerprint rule.
- Assumes the ADR-0025 strip of the account's names from `.claude.json` still
  runs where the copy is composed.
- Assumes the onboarding probe's words are still what *harness absent* reads
  as, and Windows' same-volume rule still holds for the copy.
