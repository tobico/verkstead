# 02. Attach a server in the draft composer

## What to build

In the draft composer — the details pane a Brief's Timeline card opens while
its round drafts — the **Attach** button grows into a menu. Its first entry is
*Attach file*, which does what the button did. Under it is one entry per MCP
server declared in settings, by name. Picking one attaches that server to the
Conversation and draws a **chip** for it in the row the attached files are
drawn in, told apart from a file at a glance.

**With no servers declared it is still a menu**: *Attach file*, and a link to
the MCP servers section of settings where one is declared. This was chosen over
falling back to the plain button.

**A server already attached leaves the menu.** The chip's × is how it comes
off, and taking it off puts it back in the menu.

**What is attached is the name, not a copy.** The Conversation records which
server names it has, and the declaration is looked up by that name whenever it
is needed. So a URL edited in settings afterwards is what the Conversation
uses, and a server deleted from settings leaves a chip that **says the server
is gone** rather than vanishing — it can still be taken off.

**Servers belong to the Conversation, not the Brief.** They freeze as files
do: attaching and removing are refused once the Brief is no longer drafting,
off the same state a file is refused off, and the frozen Brief draws the chips
read-only. But a Conversation steered back into Grilling drafts a new Brief,
and that draft shows the servers attached before **already attached and
removable**, where its files start afresh.

The menu is the draft composer's only in this task. The compose page is task
07, and the Answers to a Question Set never offer servers: their Attach stays
the plain button.

Nothing launches with a server yet — that is task 03.

## Acceptance criteria

- [ ] The draft composer's Attach opens a menu of *Attach file* followed by
      each declared server, and *Attach file* attaches a file as before.
- [ ] With no servers declared the menu holds *Attach file* and a link that
      opens the MCP servers section of settings.
- [ ] Picking a server draws its chip and takes it out of the menu, and the
      chip's × takes it off and puts it back.
- [ ] Attaching and removing a server are refused once the Brief is frozen, and
      the frozen Brief draws the chips with no ×.
- [ ] A new draft opened by a steer into Grilling shows the Conversation's
      servers attached and removable.
- [ ] A chip whose server has been deleted from settings says so, and can still
      be removed while drafting.
- [ ] Attach on an Answer is still the plain file button.
