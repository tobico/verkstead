# 05. The prompt goes to a file

## What to build

A session's prompt is one argument today, and an implementing session's carries
the whole handoff document inlined. Windows caps a command line at 32,767
characters, which a long handoff exceeds — so **on Windows the prompt is always
written to a file** in the Conversation's handoff directory, and the agent is
started on one line naming it.

**Always rather than only when it would not fit**, so that a Windows session has
one shape rather than two and nothing turns on a length nobody measured. **And
only on Windows**, because nothing on the other platforms is the worse for the
argument: Linux and macOS argv are unchanged, and so is every prompt builder
above this.

The one line is what the agent is actually run on, and it has to be a line the
agent will act on: it names the file by the path the *session* will open it at —
which is the handoff directory as it is reached from inside, the answer task 02
gave `handoffs::inside` on Windows — and says to read it and follow it. The file
goes in the handoff directory rather than anywhere else because that is the one
writable place a session has that git will never see, and every session of the
Conversation reaches it.

The choice belongs where the argument vector is built, off `Platform` as a value
rather than a `cfg!`, so both arms are testable on either machine.

## Acceptance criteria

- [ ] On Windows a session's argv carries one short line naming the prompt file
      instead of the prompt, and the file in the handoff directory holds the
      whole prompt the builder produced — the companion listing and the naming
      instruction included.
- [ ] A stand-in agent started that way reads the Brief out of the file.
- [ ] A prompt long enough to exceed a Windows command line starts a session all
      the same.
- [ ] Linux and macOS argv are byte-for-byte what they were, and no file is
      written on either.
