# 01. Declare a server in settings

## What to build

A new **MCP servers** section on the settings page, where a server is declared
once for the whole installation. In this task a server is a **name and a URL**
and nothing else: headers are task 04, and nothing reads a declaration until
task 02.

Only remote servers spoken to over HTTP can be declared. A stdio server — a
command the agent starts as its own child — was considered and turned down in
the grilling, so there is no command, no arguments and no transport choice on
the form.

**The name is the identity.** It is what a Conversation's chip will refer to
and what the agent sees in its tool names, so it is lowercase letters, digits
and hyphens, and unique among the declared servers. A name that breaks either
rule is refused by the server, naming the field at fault, before anything is
written — the way an ignored-comments rule is refused. **A server is never
renamed**: the form offers the name on a new server alone, and changing one is
deleting it and declaring it again. The URL can be edited.

The declarations are kept in `config.yaml` beside the other things Verkstead is
told, and follow that file's contract: read at the moment they are needed, and
an absent, empty or unparseable key is no servers rather than an error. The
settings page saves the whole of `config.yaml` in one request, so every
existing section has to carry the servers along in its own save, and this
section has to carry theirs — see how the sections hold one another's values
today.

The section arrives as the others did: a word in the settings openings, a card
in the middle pane and a details pane the route reaches. A list of the declared
servers with a way to add one, edit one's URL and delete one.

This task also writes the decision down, because it reverses a recorded one:

- **A new ADR** amending ADR-0011, which records that a session is given no MCP
  servers. The account's own are still stripped; a server declared here and
  attached to the Conversation is the exception. Record what the grilling
  settled and turned down: HTTP only, no stdio; static headers only, no OAuth;
  header values secret from the page and the wire but readable by the session,
  which has to be handed them; a reference by name rather than a copy; all
  tools allowed with no per-tool filter; every session of the Conversation; all
  four harnesses. And one rule for later: **a harness that cannot take an HTTP
  MCP server launches without it and the chip says that harness does not take
  it** — nothing is built for that now, because all four can.
- **A `CONTEXT.md` entry** for the term, in the glossary's own form, with its
  `_Avoid_` line, and the **Built Root** and **Attachment** entries amended
  where they now say something untrue.

## Acceptance criteria

- [ ] A server added on the settings page is listed there after a reload, its
      URL can be changed, and it can be deleted.
- [ ] A name holding anything but lowercase letters, digits and hyphens, or one
      already taken, is refused naming the field, and nothing is written.
- [ ] An existing server's name cannot be changed from the page.
- [ ] Saving any other settings section leaves the declared servers as they
      were, and saving a server leaves every other section as it was.
- [ ] A `config.yaml` with no servers key, or one that cannot be parsed, reads
      as no servers.
- [ ] The section has its own route, and the settings routes suite reaches it.
- [ ] The ADR and the glossary entry are written, and the entries they amend
      no longer say a session can have no MCP server at all.
