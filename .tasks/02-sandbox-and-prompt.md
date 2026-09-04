# 02. Into the sandbox and the prompt

## What to build

What the files are for: every session of a Conversation that has attachments
can read them, and its prompt says they are there.

**The bind.** The Conversation's attachments directory is bound into its
Sandbox read-only, the way the handoff directory is bound read-write: a field on
the sandbox description beside the handoff directory, resolved where the
sandbox is built for a Conversation, and one more line in the ordered surface
after the tmpfs and HOME. Inside it is `/verkstead/attachments`, beside the
skills, on the platform whose sandbox can mount one there. On macOS `/verkstead`
is the Data Directory itself, so the policy reaches the Conversation's own
subdirectory of the attachments root and no other, and the path a session is
told is that directory's real one — the skills' own arrangement, one level
deeper. A Conversation with no attachments gets no bind and nothing at that
path. The Compile Server's sandbox does not get it.

Read-only is the decision: the copy is the record, and an agent that wants to
work on a file copies it into the Worktree.

**The prompt.** A `# Attached files` section appended in the one place every
session is launched from, beside the companion-repositories and branch-naming
sections, so a prompt builder added later cannot forget it. One line per file
naming its in-sandbox path and its size, grouped under its origin — *the Brief*
now, a Question Set's Answer later — in the neutral tone of the companions
listing: what is there and where, and no instruction about what to do with it,
because the Brief says what a file is for. No attachments, no section. It
appears in every session kind, grilling through the wrap-up's own.

The VM test's own sandbox probe is a hand-written bwrap invocation kept in step
with the server's, and its listing of `/verkstead` is asserted; both move with
this.

## Acceptance criteria

- [ ] A sandboxed probe in a Conversation with an attached file reads it at
      the path the prompt names and cannot write it or create a file beside it.
- [ ] A Conversation with no attachments gets no `# Attached files` section and
      nothing at `/verkstead/attachments`.
- [ ] The section is on the prompt of a grilling session and of a wrap-up
      session alike, and the VM test still passes.
