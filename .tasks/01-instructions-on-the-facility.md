# 01. Instructions on the facility

## What to build

The **Instructions** text becomes a Replicated record (ADR-0028) on stage 01's
facility: one text for the cluster, newest write wins, out of `config.yaml`.
Read the facility first: the store's `replicated` module (the `Kind` trait, the
merge and stamping rules, `take_up`) and the server's `replicating` module with
its Authors kind. They are the pattern to follow, not to copy.

**A single record under a fixed uuid**, the same constant on every device. Its
fields are the text. **Cleared is an empty text**, written as an edit and never
a deletion stamp, so a later text still wins over it.

**Taken up from `config.yaml` once, at serve time**, as the Authors were, after
which the key is dropped from the file and never read again. A hand-edited
`instructions:` key after the move is ignored, and the log says so. Two rules
for the move, both decided with the human:

- **An empty or absent text is never taken up.** Otherwise a device that never
  had a text, upgraded last, would wipe everyone's.
- **A non-empty text taken up is stamped older than any edit**, as the Author
  mirrors were (the epoch). Any edit made after the upgrade wins. Between two
  devices' taken-up texts, the device id decides: arbitrary but stable, and the
  human sees the one that won and edits it.

**An Annex takes nothing up.** A Verkstead whose `config.yaml` carries
`kept_by` has no settings of its own: its `instructions:` key is only the
Keeper's old copy, so it is dropped rather than taken up, and the Annex gets
the text from its Keeper as a member, like any other. **The Keeper stops
handing the Instructions in** at boot (the `working_as` copy).

**Every read goes through the record.** The settings save (one request writes
every section) writes the record only where the text changed, so a save about
something else rewrites nothing. The Built Root writes the record's text into
each harness's file at launch, as it does today from `config.yaml`. The
settings page reads it back as it does now. The kind gets its own *moved*
Nudge, announced on a local write, and a merge that changes the text redraws
the page on every device.

Rewrite the **Instructions** term in `CONTEXT.md`: one replicated text in the
database, newest wins, cleared being empty. Drop its *Becoming one text for the
cluster* pointer, and fix the Annex term's list of what is copied in at boot.
Read `docs/copy.md` before touching any string.

## Acceptance criteria

- [ ] Two Verksteads: a text saved on A is in the next root built on B, after the Nudge, and after B's start pull if B was down.
- [ ] Clearing the text on B empties it on A, and a session launched on A then gets no instructions file.
- [ ] An installation with `instructions:` in `config.yaml` starts with that text from the database and the key gone from the file. A hand-edited key added afterwards is ignored and logged.
- [ ] Two devices, one with a text and one with none, both upgraded: both show the text, whichever was upgraded last.
- [ ] An Annex's own `instructions:` key is not taken up, and the Keeper no longer writes one into it.
- [ ] A settings save that changes another section leaves the record's stamp as it was.
- [ ] The **Instructions** term in `CONTEXT.md` is rewritten and its pointer gone.
