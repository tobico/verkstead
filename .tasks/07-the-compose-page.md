# 07. The compose page

## What to build

The compose page — where a new Conversation's first Brief is written — gets
the Attach menu the draft composer got in task 02: *Attach file*, then each
declared MCP server, or the link to settings where none is declared. The same
menu and the same chip, not a second drawing of them.

The difference is the compose page's own. There is no Conversation yet, so
files picked there are **held in the page until Start or Save**, and servers
are held with them: a pick draws its chip at once, its × takes it off, and
nothing is recorded anywhere until the Conversation exists. On Start or Save
the Conversation is made with those servers attached, in the same step that
gives it its files.

This holds for every Process the compose page can start, since a server is the
Conversation's whatever its sessions are sent to do.

After this task the menu is in both places a Brief is written, and Answers to a
Question Set still offer files alone.

## Acceptance criteria

- [ ] The compose page's Attach opens the same menu the draft composer's does,
      link to settings included.
- [ ] A server picked there draws a chip and leaves the menu, and its × undoes
      both, with nothing recorded before Start or Save.
- [ ] A Conversation started or saved from the page has the picked servers
      attached, and its first session runs with them.
- [ ] Leaving the page without starting or saving attaches nothing to anything.
