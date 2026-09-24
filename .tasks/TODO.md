# Working the tree

The Code pane's tree stops being read-only. A row drops a menu holding new
file, new folder, rename and delete, each of them an endpoint of the files API
done by the server as itself, refused in the body the way stage 01's reads and
writes are, and reflected both in the tree and in any tab the path is open in —
a renamed file's tab is retitled and saves to the new path, and a deleted
file's tab stays read-only saying the file is gone with its text still there to
be copied out. Ctrl+P opens a palette over git's own list of files across every
root and opens the pick into the active group. And the pane's own menu grows
the three settings ADR-0019 exposes — word wrap, font size and minimap — kept
per device and read by every editor mounted.

Between them they are what makes the pane somewhere work is finished rather
than only read: everything up to here could open a file and save it, and
nothing could make one, move one, take one away, or find one whose path nobody
remembers.

Roadmap stage: [03: Working the tree](docs/roadmaps/code-pane/03-working-the-tree.md)

## Tasks

- [x] 01: The row menu, and new file and new folder — [details](01-row-menu-and-making.md)
- [x] 02: Rename, and the tabs that follow it — [details](02-rename.md)
- [x] 03: Delete, and the tab that stays — [details](03-delete.md)
- [ ] 04: Quick open — [details](04-quick-open.md)
- [ ] 05: The pane's menu, and its three settings — [details](05-editor-settings.md)
