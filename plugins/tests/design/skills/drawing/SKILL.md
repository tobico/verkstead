---
name: drawing
description: Draw what the human asked for, round by round, through the MCP server they attached, and commit each round's pictures. Use when a session has been dispatched into a Design conversation's drawing rounds.
---

Draw what the brief at the end of this prompt asks for, and keep talking to the
human about it. This is a conversation rather than a single step: each round
goes back to them as a Question Set, and they say when there is nothing else.

You start in a worktree of the repository, on the branch the work is on. There
is nothing to create and nothing to switch to.

## 1. Find out what you are drawing with

The pictures are made through an **MCP server** the human attached to this
Conversation — a modelling tool, an image generator, whatever they picked — and
this prompt lists the ones they attached under *MCP servers* with the tools
each one gives you. Read that list before anything else: it is what you have,
and there is nothing else on this machine to draw with.

Where the list is empty, there is nothing to draw with at all. Say so in one
sentence and ask the human which server to attach, rather than drawing
something by hand or writing the pictures as code.

## 2. Draw a round

One round is one thing to look at. Make it, write the files into the worktree,
and commit them — a round the human can see is a round that is on the branch,
and nothing of this session's survives it otherwise.

Keep the source of a picture beside the picture, where the tool makes one: the
scene, the prompt, the parameters. The next round starts from what this one
left, and so does the export.

## 3. Ask the human what they think

Put the round to them as a Question Set: what you drew, what you chose and why,
and the question you actually want answered. One Set a round, with the
pictures attached, rather than a question at a time.

Then do what they say, and ask again. The rounds go on for as long as they
want them to: what ends them is the human ticking **Nothing else** beside the
comment box, which closes the drawing and leaves the branch where the last
round put it.
