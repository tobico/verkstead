# 04. The release legs, and the way in for a Windows reader

## What to build

A `v*` tag attaches two more files, and a Windows reader can find one of them
and get past what Windows will say about it.

**The desktop leg**, on the plumbing stage 03 laid and in the shape the AppImage
and dmg legs already have: a Windows runner, the viewer fetched from the
`viewer` job, a cache key of its own, the exe built, and then assertions that
run the artifact this leg is about to upload — it starts, it serves a document
with the bundle in it, and its own log says a tray came up. Bounded, for the
reason the other two are: a dialog nobody dismisses would hold the job until the
runner's hours ran out. Uploaded under the `desktop-` artifact prefix, which is
what keeps it out of `publish`'s count of bare binaries. The asset is
**`Verkstead-x86_64.exe`**, and `publish` already says in a comment that stage
05 adds the one line to its list of named desktop assets and nothing else in
that job.

**The bare CLI leg**, which is a fifth row of the existing matrix rather than a
job: `x86_64-pc-windows-msvc` on a Windows runner, uploaded as
`verkstead-windows-x64.exe` — the CLI's own naming scheme with the extension
Windows needs to run a download at all. `publish` counts five bare binaries
rather than four, and the comment that says why four was that set's own number
is rewritten rather than left to read as the truth it no longer is.
`nix/release.json` stays at its four nix systems: nothing fetches a Windows
binary through nix.

**And the way in.** The README describes three downloads today and says Windows
is a stage away; adoption has a section per platform and releasing says what a
tag produces. All three gain the exe: where it goes — anywhere, it is portable —
what SmartScreen says about an unsigned binary and where the "run anyway" is,
and that sessions run on Linux and on a Mac and are a later stage's here. There
is no Gatekeeper dance to copy: SmartScreen's is shorter and its own.

## Acceptance criteria

- [ ] A rehearsal run — the workflow started by hand — builds both Windows
      artifacts and runs the desktop leg's launch assertions against the exe it
      uploaded, and a tag attaches `Verkstead-x86_64.exe` and
      `verkstead-windows-x64.exe`
- [ ] `publish` still refuses a desktop artifact it was never told about, its
      count of bare binaries is five, and the manifest job still names four nix
      systems
- [ ] A Windows reader finds the download in the README and in adoption, knows
      what SmartScreen will say and how to get past it, and nothing left in the
      docs says the desktop app or the release is Linux-only
