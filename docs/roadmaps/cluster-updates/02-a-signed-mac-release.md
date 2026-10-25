# 02. A signed Mac release

## Goal

A release's Mac app is signed with a Developer ID and notarized, and a `zip`
of the universal app is published beside the dmg. A fresh Mac opens the app
from the dmg with no *Open Anyway*; the workflow's Mac leg verifies the
signature strictly and assesses the bundle as Gatekeeper would before it
publishes. The README's Mac section loses its *Open Anyway* paragraph. The
secrets are the human's: the enrolment, the certificate and a notarization
credential go into the repository's secrets before this stage can be proven,
and a build without them still packs an ad-hoc bundle so that a fork or a pull
request build does not fail.

## Decisions in force

- **Developer ID and notarization, because electron-updater needs them**
  ([ADR-0027](../../adr/0027-cluster-updates.md), *The release*).
  Squirrel.Mac refuses an ad-hoc signed update, and this is the one condition
  that costs money and an account; the human took it knowing both, as an
  individual enrolment. Leaving the Mac to update by hand was not taken.
- **The `zip` target beside the dmg**, since `latest-mac.yml` is written
  off the zip and cannot be without it. The dmg stays what a human
  downloads.
- **Hardened runtime on**, which notarization requires and which the
  configuration turns off today because nothing was notarized; the
  entitlements an Electron app needs under it come with it.
- **The sidecar CLI is signed as part of the bundle**, as it is already
  verified to be today; a Developer ID signature changes the identity and
  not the layout.
- **Ad-hoc when the secrets are absent.** A pull request build, a fork, and
  the probe branch all pack; only a release signs. Which of the two a run did
  is said in its log.
- **The maintainer is an individual with no company**, and Windows signing
  was declined separately; this stage is Apple's programme only and says
  nothing about Windows.

## Proposed tasks (provisional)

1. **Signing behind the secrets** — the identity, hardened runtime and
   entitlements in the electron-builder configuration, taken from the
   environment when present and ad-hoc when not.
   - A pack with no secrets is ad-hoc and verifies as today.
   - A pack with the secrets carries a Developer ID signature that verifies
     strictly, bundle and sidecar both.
2. **Notarization and the zip** — the universal app notarized and stapled in
   the Mac leg, the zip target added and published with the dmg.
   - `spctl --assess` accepts the stapled app on the runner.
   - The release holds `Verkstead-universal.zip` beside the dmg.
3. **The words** — the README's Mac section, `docs/releasing.md`'s Mac leg
   and its post-release checklist.
   - No *Open Anyway* anywhere a Mac user reads.

## Re-verify at start

- Assumes the Mac configuration is the `mac:` section of
  `desktop/electron-builder.yml` with `identity: "-"`, `hardenedRuntime:
  false` and a `dmg` target only, and that the Mac leg is the
  `desktop-macos` job of `.github/workflows/release.yml`, which `lipo`s the
  two CLIs and runs `codesign --verify` on the bundle and on
  `Contents/Resources/cli/verkstead`.
- Assumes there is no Mac in the dev shell: Mac work is proven by a
  throwaway workflow on the `macos-pack-probe` branch running on `macos-15`,
  whose step shell is `bash -e`.
- Assumes the secrets' names are not yet chosen: pick electron-builder's own
  (`CSC_LINK`, `CSC_KEY_PASSWORD`, and the notarization trio) unless the
  workflow has a convention by then.
- Assumes the README's Mac section still walks the human past *Open
  Anyway*, and the checklist in `docs/releasing.md` still names the README
  *Updating* section that stage 04 writes.
