# 07. Broken states and the refusals

## What to build

Three ways a **mirror** Profile is not usable on this device, each **named on
the row** and each refusing Start by name. Brokenness is already answered per
read for a local Profile; these are the same question asked of a Profile whose
account is somewhere else.

- **No login file at home.** A Claude login kept in the macOS Keychain leaves no
  file to mirror, so the Profile cannot be used away at all. The home device
  says so as the mirrors refresh, and the row says it wherever it is drawn. This
  is what a sign-out at home comes to as well — the file goes, the Profile reads
  as having none, and the fix is a login on its home device.
- **The harness absent on this device.** Whether a harness is on this machine is
  what the onboarding probe already answers, so a mirror of a type absent here
  reads broken **in that probe's own word**, carrying the same trouble sentence
  that step shows, rather than in a second vocabulary for one fact.
- **The home unreachable.** A mirror whose home is not answering cannot have its
  account fetched, so it reads the way that device reads in the sidebar, and
  Start names the device.

**Refused rather than hidden.** All three stay in the pickers and in the
Profiles section: a row that says why it cannot be run is something to go and
put right, and a row quietly missing is a human looking for a Profile they know
they saved. It is also what keeps a Pairing made earlier legible — a
Conversation paired with a Profile that has since broken reads as paired with a
broken one.

**Refused before anything starts**, not found out as a session comes up logged
out: a session launched under any of the three is the failure this reading
exists to get in front of.

## Acceptance criteria

- [ ] A Profile whose home account holds no login file reads as not usable away
      in the pickers and the Profiles section of every device, and Start under
      it is refused naming that.
- [ ] A mirror whose harness is absent here reads in the onboarding probe's own
      word with that step's trouble sentence, while the same Profile on its home
      device is fine.
- [ ] A mirror whose home has stopped answering reads unreachable, and Start
      names the device.
- [ ] None of the three takes the row out of a picker, and a Conversation
      already paired with one reads as paired with a broken Profile.
