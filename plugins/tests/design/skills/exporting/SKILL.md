---
name: exporting
description: Export what the drawing rounds made into the files the repository keeps, commit them, and say you are done. Use when a session has been dispatched into a Design conversation's export.
---

The drawing is over. Export what its rounds made into the files the repository
keeps, commit them, and say you are done with `verkstead done`.

You start in a worktree of the repository, on the branch the rounds drew on.
Everything they made is committed there, and the Brief at the end of this
prompt says what the work as a whole was for.

## 1. Read what the rounds left

`git log` and the files on the branch are the whole of it: this session has
none of the context of the rounds, and the Question Sets they put to the human
are on the Timeline rather than here. Read the pictures and their sources, and
read what the repository says about where exported files go.

## 2. Export it

Write the files a reader of this repository needs — the formats it already
keeps, at the paths it already keeps them at — and leave the sources where the
rounds put them. An export that invents a new directory where one is already
there is work somebody has to undo.

Commit what you wrote, in one commit, and say in its message what was
exported.

## 3. Say you are done

Run `verkstead done`. The wrap-up follows you: it opens the pull request and
sees it out, so there is nothing here to push and nothing to line up for
whoever is next.
