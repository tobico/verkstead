# 02. The Share pane

## What to build

Sharing moves from the menus to a pane of its own. A share icon button —
fontawesome's `share`, drawn exactly the way the settings gear beside the
Verkstead wordmark is — stands to the right of the timeline pane header and
opens a Share opening in the details pane; the button reads as open while the
pane shows, and on a narrow window the press walks into the details pane the
way selecting a Timeline event does.

The pane shows the Published Share where one exists: the viewer link with a
copy-to-clipboard button beside it, when the share was taken, and the
underlying gist link — the gist is where a share can be deleted, and nothing
in-app deletes one. Where none was published yet, it says so plainly. Under the
details, buttons carry the actions the menus lose: Download, Publish / Publish
again, and Share to pull request — the last only where the record holds a pull
request — keeping the pending words and outcome toasts they have today.

The four share rows (Share, Publish, Published share, Share to pull request)
leave the conversation actions, which takes them out of the status button menu
and the sidebar right-click together. The record has to start handing the
workbench the gist URL beside the composed viewer link, which today is the only
one it sends.

## Acceptance criteria

- [ ] Neither the status button menu nor the sidebar right-click menu carries
      a share row.
- [ ] The share icon button opens the Share pane, reads as open while it
      shows, and works on a narrow window.
- [ ] The pane shows the viewer link with copy, the publish moment and the
      gist link for a published share, and a plain word where none exists.
- [ ] Download, publish and share-to-pull-request all work from the pane, and
      share-to-pull-request is absent where no pull request is on record.
