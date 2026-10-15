# 06. A session in the Annex

## What to build

This is the end-to-end proof that the Annex is somewhere work happens. It
also covers the two pieces of the viewer that the proof needs.

**The compose page's device select** (decided). Today the select draws only
a device's name and an OS icon (the visible label is `device.name`, and the
OS word only picks the icon), and it draws no Annex mark. The Annex carries
its Keeper's hostname, so its row would read the same as the Keeper's. The
Annex's row reads **its name, the Annex mark, and *Linux (WSL)* as a quiet
second word**. Use the same mark the Devices list draws off
`DeviceIdentity.kept_by`, and share the component rather than copying it.

**The remembered parent preset to `/srv/repos`.** The parent that Create repo
and stage 02's clone remember is the browser's `localStorage`
(`verkstead.repo-parent.<deviceId>`, in `web/src/remembered.ts`), not a
server setting. So the preset is a fallback in the viewer: an Annex (a device
whose identity has `kept_by`) with nothing remembered reads `/srv/repos`.
Something the human remembered there wins.

**The proof.** On this Windows machine, with the Annex made (task 04) and
linked (task 05):

1. Clone a repository onto the Annex through stage 02's clonable row. It
   lands in `/srv/repos` with nothing asked.
2. Draft a Conversation onto *Linux (WSL)* with a Profile mirrored from the
   Keeper.
3. The session runs under bwrap in the Annex and commits.

The Keeper's git author and GitHub token reach the Annex through task 03's
written `config.yaml`/`secrets.yaml`, and task 05's link rewrites them at
every boot. If they are written only at make, make task 05's link rewrite
them too, so a change on the Keeper reaches the Annex at its next start.
Stage 04 copies the rest of the settings.

## Acceptance criteria

- [ ] The compose page's device select lists the Annex with its name, the
      Annex mark and *Linux (WSL)*. A viewer test covers it.
- [ ] Create repo and the clone on the Annex default to `/srv/repos` when
      nothing is remembered, and a remembered parent wins. A viewer test
      covers it.
- [ ] A Conversation drafted onto the Annex with a mirrored Profile runs its
      session under bwrap and commits. The commit's author is the Keeper's
      git author, and `gh auth status` in the session is the Keeper's
      account.
