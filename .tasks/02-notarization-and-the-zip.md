# 02. Notarization and the zip

## What to build

The universal app is notarized and stapled inside the Mac pack, and a `zip` of
it is published beside the dmg.

- **Notarization through electron-builder**, with the App Store Connect API
  key the human chose: `APPLE_API_KEY` (a path to the `.p8`, so the leg writes
  the secret's contents to a file first), `APPLE_API_KEY_ID` and
  `APPLE_API_ISSUER`, from repository secrets. The ticket is stapled to the
  app before the dmg and the zip are made from it. A pack without the key
  (task 01's ad-hoc path, a developer's checkout, the probe without secrets)
  skips notarization and says so; a **release** without it fails before
  packing, beside task 01's check for the certificate.
- **The `zip` target**, universal, named `Verkstead-universal.zip` the way the
  dmg's name is written down rather than defaulted. The dmg stays what a human
  downloads; the zip is what stage 04's `latest-mac.yml` will point at. That
  metadata, and any blockmaps, are stage 04's — `publish: null` stays.
- **The Mac leg asserts the files it uploads**: in the mounted dmg, and in the
  app unzipped out of the zip with `ditto -x -k` (the zip is what Squirrel.Mac
  will install, so a zip that loses a symlink or a seal must fail here),
  `codesign --verify --strict`, `xcrun stapler validate`, and `spctl --assess
  --type execute` accepting the app as Gatekeeper would. Both files are
  uploaded under the `desktop-macos` artifact.
- **`publish` names the zip** in its list of desktop assets, so the Release
  holds `Verkstead-universal.zip` beside the dmg and the name check still reads
  both ways round.

**This task needs the human's real secrets in the repository to be proven**:
the enrolment had not been started when the stage was planned, so ask them
(a Question Set) whether all five secrets from task 01's section of
`docs/releasing.md` are in before running the proof, and wait for them if not.
The proof is the `macos-pack-probe` branch with the secrets reaching it, which
repository secrets do.

## Acceptance criteria

- [ ] On the probe runner with the secrets, `spctl --assess --type execute` and `stapler validate` accept the app in the mounted dmg and the app unzipped from `Verkstead-universal.zip`.
- [ ] A pack without the notarization key skips notarization and says so; a release without it fails before packing, naming the missing secret.
- [ ] `publish` expects `Verkstead-universal.zip` beside the dmg, and a release's assets include it.
