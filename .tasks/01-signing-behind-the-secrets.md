# 01. Signing behind the secrets

## What to build

The Mac pack signs with a Developer ID when it is given one and ad-hoc when it
is not, and the release's Mac leg hands it the identity from the repository's
secrets.

- **The identity comes from electron-builder's own variables**, `CSC_LINK` (the
  base64 `.p12` of the Developer ID Application certificate) and
  `CSC_KEY_PASSWORD`. Present, the pack signs with that identity; absent, it
  signs ad-hoc exactly as `identity: "-"` does today (an Apple silicon Mac will
  not run a Mach-O with no signature at all, so "absent" must never mean
  unsigned). How the switch is made — the configuration, `scripts/pack.mjs`, or
  a flag the pack passes — is this task's to choose; a developer packing from a
  checkout with nothing set must get today's bundle.
- **Hardened runtime on**, with the entitlements an Electron app needs under it
  (JIT, unsigned executable memory, and whatever else the pinned
  electron-builder 26 / Electron 43 wants — check electron-builder's own default
  entitlements rather than remembering them). Under an ad-hoc signature
  library validation would reject the pre-signed Electron framework for its
  different Team ID — that is why `hardenedRuntime: false` is there today — so
  either the ad-hoc path keeps the runtime off, or the entitlements disable
  library validation; whichever, the ad-hoc bundle must still launch.
- **The sidecar CLI** (`Contents/Resources/cli/verkstead`, lipo'd universal by
  `scripts/pack.mjs`) is signed as part of the bundle with the same identity
  and the hardened runtime, as it is verified to be today.
- **The pack says which it did** in its log: Developer ID (with the identity's
  name) or ad-hoc.
- **The release's `desktop-macos` job** passes `CSC_LINK` and
  `CSC_KEY_PASSWORD` from repository secrets of those names (repository
  secrets, not an environment: the probe branch must be able to reach them),
  and **fails before packing when either is missing** — a release never
  publishes an ad-hoc Mac app. Its existing *The signature survived the
  packing* step additionally checks, on a release, that the signature is a
  Developer ID one with the hardened runtime flag, over the bundle and the
  sidecar.
- **`docs/releasing.md` gains the Mac's secrets**: how the human enrols as an
  individual, makes the Developer ID Application certificate and exports it as
  a `.p12`, makes an App Store Connect API key (the `.p8`, its key id and the
  issuer id — task 02 uses them, as `APPLE_API_KEY`, `APPLE_API_KEY_ID`,
  `APPLE_API_ISSUER`), and the exact repository secret names for all five. The
  human has not enrolled yet, so this is written first for them to follow
  while the rest is built.

**Proven on the `macos-pack-probe` branch** (macos-15, step shell `bash -e`; see
the memory on Mac proofs), twice: once with nothing set, and once with a
**throwaway self-signed code-signing certificate** generated on the runner and
passed as `CSC_LINK` / `CSC_KEY_PASSWORD`, which proves the identity plumbing
before the human's real certificate exists. Gatekeeper will not trust that
certificate and nothing is notarized here; that is task 02's.

## Acceptance criteria

- [ ] A pack with no secrets is ad-hoc, says so in its log, launches, and `codesign --verify --strict` passes over the bundle and the sidecar as today.
- [ ] A pack given a certificate signs bundle and sidecar with it, says so in its log, verifies strictly, and `codesign -dv` shows the hardened runtime flag on both.
- [ ] That signed app comes up on the runner, and a session it starts through the hardened sidecar still runs under `sandbox-exec`.
- [ ] The release's Mac leg fails before packing, with a line naming the missing secret, when `CSC_LINK` or `CSC_KEY_PASSWORD` is absent.
- [ ] `docs/releasing.md` says how to make the certificate and the API key and names all five repository secrets.
