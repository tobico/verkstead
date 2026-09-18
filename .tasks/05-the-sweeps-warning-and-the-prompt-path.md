# 05. The sweep's warning and the prompt path

## What to build

Two things that read as broken at every start and are not.

**The sweep stops warning about the record that is not a Conversation's.** The
containers directory holds one record per Conversation, named by its id, and
beside them one that stands for the installation itself under a name no
Conversation can have. The sweep reads that directory at every start and warns
about anything whose name is not an id — which is that record, every time. Its
own doc comment already claims the name is what keeps it out of the sweep and
cites a test that says so; the test that exists says only that the sweep yields
no record for it, which it did all along. What is missing is that the sweep says
nothing about it, and that is what the new test asserts. Skipped by name, the way
the half-written record beside it already is, and a file there that really is a
stranger still draws its warning.

**And a session's prompt path is composed one way.** A session whose prompt is
too long for a command line is started on one line naming the file instead, and
the path in that line is built twice over: the directory with a forward slash on
purpose — a path a session opens inside its sandbox is composed the way that
session will read it rather than the way this host would — and then the file name
joined onto it with the host's own separator. On Windows that is one path with
both characters in it. The file is composed the way the directory was.

The test that stands for what a session opens composes it the same wrong way, so
it agrees with the code on either host and settles nothing. It is corrected
rather than matched. The handoff document's own path is built beside it and wants
the same look.

Out of scope and written down so nobody reopens it: the doubled separator in the
npm path, which is npm's own shim.

## Acceptance criteria

- [ ] A server whose containers directory holds the installation's own record
      starts with no warning about it, and a file there that is genuinely a
      stranger still draws one — with a test that says the sweep was silent
      rather than only that it passed the record over.
- [ ] A Windows session started on a line naming its prompt is given a path with
      one kind of separator in it, and the test helper that stands for that path
      composes it the way the server does rather than baking the mixed form.
- [ ] The handoff document's own path is checked for the same seam and composed
      the same way.
