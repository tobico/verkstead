# 03. The Agent panel on the compose page

## What to build

The same control on the compose page, where none of the setup exists anywhere
until a press creates something. The trigger and the panel are the one
presentational piece the Draft's composer already draws — two pages that asked
this question apart would come to word it differently, which is the seam every
other control in the row already stands on — and what a pick *does* stays the
page's: a field of the draft held on the device rather than a request.

So the compose page's setup row reads Repo, Process, Agent too. The pickers
inside stand on what the repo was last grilled with, or the server's prefill
where nothing has grilled it, until the human touches them — the prefill is
shown rather than held, which is what keeps an untouched picker untouched at
create time. The trigger's reading is composed from what the pickers show rather
than from anything the page has stored, so a page whose repo memory has not
landed reads *Not chosen* until it does, and switching repos simply reads
another memory.

Which Process the row is shaped by is the one the page is standing on: Develop
for a fresh draft, and Review over a loaded pull request, which is settled
rather than picked.

The create replay is unchanged. Each role the human touched is still sent on its
own request after the Conversation is made, and a role left on its prefill still
sends nothing, so the server's own prefill stands.

Start work still waits on every role the Process uses and says so in its title,
which is task 01's text read through the new control.

The compose suite drives the pickers through the trigger, the panel opened the
way that suite already opens the Repo panel, and each picker found by its label
and read by its reading.

## Acceptance criteria

- [ ] The compose page's setup row reads Repo, Process, Agent, and the panel
      holds a picker per role the Process uses, each showing the repo's own
      remembered Pairing until it is touched.
- [ ] The trigger reads the way it does on a Draft's composer — the short
      reading, ` +N`, *Not chosen* — off what the pickers show.
- [ ] The compose suite drives the pickers through the trigger, and a create
      still sends exactly the roles that were touched and no others.
