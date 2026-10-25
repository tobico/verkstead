# A signed Mac release

A release's Mac app is signed with a Developer ID and notarized, and a `zip` of
the universal app, `Verkstead-universal.zip`, is published beside the dmg —
because Squirrel.Mac, which stage 04's electron-updater drives, refuses an
ad-hoc signed update, and `latest-mac.yml` is written off the zip. A fresh Mac
then opens the app from the dmg with no *Open Anyway*, and the README's Mac
section loses that paragraph (ADR-0027, *The release*).

The secrets are the human's — an individual Apple Developer Program enrolment,
a Developer ID Application certificate and an App Store Connect API key, kept
as repository secrets. A pack without them (a developer's checkout, the
`macos-pack-probe` branch) is still ad-hoc and still verifies; a **release** run
without them fails its Mac leg and publishes nothing. Which of the two a pack
did is said in its log. Windows signing is not part of this stage.

Roadmap stage: [02: A signed Mac release](docs/roadmaps/cluster-updates/02-a-signed-mac-release.md)

## Tasks

- [ ] 01: Signing behind the secrets — [details](01-signing-behind-the-secrets.md)
- [ ] 02: Notarization and the zip — [details](02-notarization-and-the-zip.md)
- [ ] 03: The words — [details](03-the-words.md)
