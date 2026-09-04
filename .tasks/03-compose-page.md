# 03. The compose page

## What to build

The same paperclip and pills on the compose page, where nothing exists on the
server until Start or Save as draft is pressed.

Files attached there are **held in the page** — the File objects themselves,
beside what the device is holding of the composition, and not written to
storage with it: a reload keeps the text and loses the files, and says nothing
about it. They are drawn as pills exactly as a draft's are, with a remove press
that drops the held file. The paperclip stands at the near edge of the row the
two presses are at the far edge of, left of Save as draft. Not offered while a
roadmap is loaded: the box is locked to a card then and there is nothing to
attach a file to.

**The replay.** When the press has made the Conversation, the held files are
uploaded one by one through the same route a draft uses, as one more field put
through the endpoints after creation. An upload the server refuses is one more
refusal carried to the draft the page lands on and said on its composer, with
the rest of the replay intact — the shape a refused field already takes. The
page lands in the Conversation it made and this device stops holding anything,
files included. Where Start work was pressed, the uploads finish before the
grilling is started, because the Brief freezes when it starts and a file
arriving after would be refused.

The holding — a list of files not yet on the server, with add and remove, and
a way to flush it through uploads once there is a Conversation to send them to
— is a piece of its own rather than a fold in the compose page, because an
Answer sheet holding files until it is sent is the same shape.

## Acceptance criteria

- [ ] Attaching a file on the compose page and pressing Save as draft lands on a
      draft whose composer shows the pill and whose directory holds the file.
- [ ] Attaching a file and pressing Start work lands on a grilling Conversation
      whose session can read the file; no upload is refused for arriving after
      the freeze.
- [ ] A reload before the press keeps the Brief text and shows no pill; an
      upload the server refuses is said on the new draft's composer with every
      other field of the replay in place.
