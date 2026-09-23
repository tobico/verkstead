# 06. Dragging a file in from the tree

## What to build

**A row of the file tree is picked up the same way a tab is** — the same
pointer gesture, the same grace before a press becomes a drag, the same hold to
lift one under a finger — and dropped on the same targets the last two tasks
built: a group's tab bar at an insertion point, or one of the five zones over a
group's content.

What a drop does is open the file. On a bar or a centre it opens in that group;
on an edge it splits there and opens in the new group. A file already open in
the group it is dropped on is turned to rather than opened twice, and one open
in a *different* group opens as a second view of the one buffer, which task 01
made possible and task 02 made ordinary.

**A press is still a press.** The tree's rows open a file on a click and expand
a folder on one, and a drag must not take either: a press released about where
it landed opens or expands as it always did, and one that moved a file does not
also open it in the group it came from.

**A drop anywhere else does nothing**, and leaves the tree exactly as it was —
the row still where it is, the folder still open or closed. Dragging is not a
move on disk: the tree's own rename and delete are stage 03, and nothing here
writes anything.

Only files drag. A folder row is not a drag source — there is nothing to open —
so a press on one is the expand it has always been.

## Acceptance criteria

- [ ] A file dropped on a group's content centre opens in that group and makes
      it active.
- [ ] A file dropped on a group's edge splits there and opens in the new group.
- [ ] A file dropped anywhere that is not a group opens nothing and leaves the
      tree as it was.
- [ ] A press that did not move still opens a file and still expands a folder,
      and a folder row does not drag.
