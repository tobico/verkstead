# 06. Install from git, and Update

## What to build

**Add a plugin…** also takes a git URL. The server clones it to a staging place
(through the same credential path a Repo's clone takes in the server), reads
the manifest, and shows task 05's consent card. **Install** moves the checkout
to `plugins/<uuid>/` and writes the record with the **revision pinned at
install**. Cancelling leaves nothing behind. The plugin tracks its default
branch.

**Every device clones the source for itself.** When the record arrives on
another device, that device clones the repository and checks out the pinned
revision, and the plugin is in trouble there until it can. Nothing fetches on a
timer.

**Update** on a git row is a press that fetches. Where the new manifest asks for
more capabilities than were granted, the consent card is shown again before the
new revision lands (ADR-0030). Otherwise it checks out the new revision and
re-pins it on the record, so the other devices follow. A failed fetch, or a
manifest that no longer reads, leaves the old checkout in place and puts trouble
on the row.

## Acceptance criteria

- [ ] A URL that is not a repository is refused before anything is listed or
      written.
- [ ] A manifest asking for `gitlab.com` outbound shows it on the consent card
      before Install.
- [ ] A plugin installed from git on one device is listed and cloned at the
      pinned revision on a second (two-device suite test).
- [ ] A pull that fails leaves the old checkout loaded and a trouble on the row.
      A pull whose manifest asks for more capabilities shows the consent card
      before the new revision lands.
