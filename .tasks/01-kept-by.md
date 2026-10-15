# 01. `kept_by`, and what it switches off

## What to build

An Annex is told it is kept; it does not find out (ADR-0024, *The Keeper's
Join*). This task gives `config.yaml` the entry that tells it, and switches off
the two things a kept device must never do. Nothing joins yet — that is task
02 — so this is the Annex's half standing on its own.

`kept_by` holds the Keeper's Device Id and its certificate fingerprint, in the
form the device's own fingerprint is printed in (uppercase hex pairs joined by
`:`, SHA-256 of the certificate's DER):

```yaml
kept_by:
  device: 3f2a…            # the Keeper's Device Id
  fingerprint: "AB:CD:…"   # the Keeper's device certificate
```

- **Read like any other config key**, through the server's settings. Note that
  `config.yaml` is re-read on every call rather than once at start, and that
  the settings page writes the whole file: a key the page never draws has to
  be carried through a save the way `session_path` is, or one press of Save
  deletes it. `kept_by` is never drawn on the settings page.
- **No Onboarding Mode.** A server whose config has `kept_by` at startup skips
  the verdict and reports the mode off, so a viewer opened on it lands on the
  compose page rather than `/setup`. The mode only redirects a viewer nobody
  opens, and a mode nothing will clear is worse than none.
- **Never advertised over mDNS.** A kept device does not announce itself on
  the LAN, whatever the advertising flag says: its address is its Keeper's
  (ADR-0024, *The Keeper forwards for its Annex*). Browsing for others is
  unaffected.

Stage 03 is what writes `kept_by` into a real Annex's config; here a suite
writes it into a data directory before starting the server, and task 02's
linking operation rewrites it.

## Acceptance criteria

- [ ] A server started over a data directory whose `config.yaml` has
      `kept_by` reports Onboarding Mode off on `GET /api/ui/onboarding` with
      nothing set up, and the viewer opens on the compose page, not `/setup`.
- [ ] The same server without `kept_by` still enters Onboarding Mode as today.
- [ ] Saving the settings page on a kept server leaves `kept_by` in
      `config.yaml` unchanged.
- [ ] A kept server publishes no mDNS announcement, even with advertising on;
      an unkept one still does.
