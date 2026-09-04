# 04. Drag and drop

## What to build

The whole composer box — the Brief text, the pills and the Setup row inside it
— is a drop target on both composers. While a drag carrying files is over it
the box is highlighted, and the highlight goes when the drag leaves or drops.
Dropping attaches every file dropped the way the paperclip does: uploaded at
once on a draft, held in the page on the compose page. Directories in a drop
are skipped without a word; a drag carrying no files changes nothing.

With this the attaching UI has two entry points and one behaviour, so what was
built in tasks 01 and 03 becomes **one shared piece**: the pill row with its
remove presses and the paperclip, taking the list to draw and what to do on
add and remove, and the drop handling over a box. The draft composer and the
compose page both draw it. An Answer sheet is the third place it will be drawn.

## Acceptance criteria

- [ ] Dropping two files on a draft's composer attaches both, and the same on
      the compose page holds both.
- [ ] The highlight is drawn while files are dragged over the box and not
      otherwise, and a drop with a directory in it attaches the files and skips
      the directory.
- [ ] Both composers draw their pills, paperclip and drop handling through the
      one component.
