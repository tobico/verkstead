# 01. Identity on disk

## What to build

A Verkstead invents itself an identity at first start and reads the same one back
at every start after: a **device id** and a self-signed **certificate**, both
kept in the Data Directory beside the Workbench Key's own file
(ADR-0020, *A device is an id and a certificate*).

The id is random and short enough to sit in a URL segment, because every record
and URL in cluster mode names a device by it. The tailnet node name and the
hostname were both rejected as ids: one is gone the moment the machine leaves the
tailnet, and the other collides between a Windows machine and its WSL.

The certificate is the other half of what a device *is*: a link, in the stages
after this one, is the two fingerprints each side holds, and nothing is stored
beside the certificates. Here it is only made, read back and named — nothing
presents it yet, and nothing renews it yet.

**Follow the Workbench Key's own module for the shape of all of this**, because
it is the same problem solved once already: a file of its own rather than a field
in the settings files, written atomically with the platform's own mode, whatever
is already there read back and a fresh one written only where there is nothing,
and an empty file treated as a file that is not there. It also has a `stated`
constructor beside its `issued` one, so a suite can pin a golden fixture instead
of filtering random bytes out of a payload — the identity wants the same, since
every later task and every later stage asserts against an id and a fingerprint.

What makes this demonstrable is the **startup line**: the one line an operator
reads as Verkstead comes up already carries the listen address, the Data
Directory and the rest, and it gains the device id and the certificate's
fingerprint. The fingerprint is spelled the way a human compares one by eye,
because that is what it is for — stage 02 draws it on a pending row and on a
confirmation modal so two machines can be checked against each other.

Certificate generation is new to this tree: nothing here makes one, and `rustls`
is present only as `reqwest`'s feature. Whatever crate is added for it, **write
the validity down explicitly rather than taking the crate's default** — the
defaults in this space are not short, and a certificate that never expires is the
thing ADR-0020 rejected. This stage's settled validity is **90 days**, which
task 07 acts on; nothing here re-issues.

The glossary gains its first cluster-mode terms, near the Remote Access entries
it belongs beside.

## Acceptance criteria

- [ ] A second start reads the first start's id and certificate; deleting the
      files makes fresh ones, and an empty file counts as one that is not there.
- [ ] The startup line names the device id and the certificate's fingerprint,
      and the fingerprint reads the same across a restart.
- [ ] Both files are written with the same mode the Workbench Key's file gets,
      and a stated id and certificate exist for the suites the way the key's
      stated secret does.
- [ ] The certificate's validity is 90 days, said once where it can be read
      rather than inherited from a crate's default.
- [ ] `CONTEXT.md` gains **Device** and **Device Id**, beside the Remote Access
      terms.
