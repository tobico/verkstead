# 06. Sessions turn on, with the note

## What to build

Windows runs sessions. `run_on(Platform::Windows)` answers `Run`, and everything
that refused for being Windows stops refusing.

**`SessionsHere::NotOnWindowsYet` goes, and the wording with it.** Six refusal
call sites in the server carry it today — starting a grilling, adopting a stage,
resuming, steering, sending a conflict back to a wrap-up, and opening a
Conversation Terminal — and each loses its arm, its variant on the rendered
enum, and the test that asserted it. The viewer's `NoSessions` is in eight files:
the module that holds the sentence, the two panes that draw it instead of a
press, and the five refusal maps that name it. The vitest cases that assert the
refusal go with them.

**In its place, one server-decided value on the Conversation view**, carried the
way `compiles_uncached` is: a fact the server works out and the page reads,
rather than three fields for the browser to combine. It says whether this
session will be sandboxed, and on a Windows build until stage 03 it says it will
not.

**Drawn in three places, and this task builds two of them**: above **Start work**
on the composer, and beside the terminal on the session pane. The third is the
Conversation Terminal pane, which is task 07's — a human who opens a tab from
the Timeline's header reaches neither of these two, and what that pane says is
about the shell in front of them rather than about the agent.

The wording, approved:

> This session is not sandboxed: on Windows the agent runs with your own
> account's reach until the sandbox stage lands.

One value, read in three places, and no setting behind it: a setting would be a
second place to say what the note already says.

## Acceptance criteria

- [ ] Pressing **Start work** on a Windows server starts a session, and what the
      agent prints reaches the Capture; the Rust test that said Windows runs no
      sessions says the opposite.
- [ ] Nothing in the server or the viewer refuses anything for being Windows:
      the six refusal arms and the viewer's `NoSessions` are gone, with their
      tests.
- [ ] vitest draws the note above **Start work** on the composer and beside the
      terminal on the session pane of an unsandboxed view, and draws neither on
      a sandboxed one.
