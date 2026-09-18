# 01. The desktop app refused

## What to build

A `claude` a session would resolve to that is the Claude Code **desktop app**
rather than the CLI is recognised as that, said to be that, and refused as that.
The reporter's machine had one: a session started it, it printed nothing and
exited, and nothing anywhere said why. The desktop app really does include
Claude Code — it just has no CLI on the end of that name, and Anthropic's own
documentation says to install the CLI separately to use `claude` from a
terminal. That sentence is the whole of what Verkstead has to say about it.

**Recognised by shape, and no process is run to find out.** An Electron binary
asked for `--version` may open a window, and ADR-0016 kept the wizard to reads.
Three shapes, all of them `stat`s and path reading:

- an ancestor directory named `AnthropicClaude` under `%LOCALAPPDATA%`, which is
  where the app a person downloads installs;
- a sibling `Update.exe`, which is Squirrel's updater standing beside the app it
  updates;
- an app-execution alias under `%LOCALAPPDATA%\Microsoft\WindowsApps`, which is
  what the MSIX package Anthropic ships for deployment puts on the `PATH`.

The third is defensive: the MSIX route is documented but its alias has not been
seen on a machine from here, and an alias that is not called `claude` simply
never matches. The first two are the reported install.

**Where it belongs is beside the four things already said about a name.**
Resolving a harness on a session's `PATH` already answers more than *found* — it
answers where a name was seen when a session still cannot use it, so that a row
sends somebody to fix a `PATH` rather than to install what they have. This is a
fifth answer of the same kind, carried the same way: a variant on what the
resolving returns, and a variant beside it on the wire, so what crosses is
*where the file is and that it is the desktop app* and the wording is the
viewer's own, exactly as the install commands beside it already are.

**The wizard's row** draws it as the desktop app with the CLI install named,
rather than as present. A real CLI — the npm shim, the native installer's link,
a distribution's — is unchanged and still ticks.

**And a session under such a Profile is refused rather than started**, before
anything is spawned, the way a session with no sandbox to run in is: the log
says which file and why, and Verkstead's own line says the same where the human
reads it, by the route that refusal already takes.

## Acceptance criteria

- [ ] A `claude` under `AnthropicClaude`, beside an `Update.exe`, or standing as
      a `WindowsApps` alias draws the wizard's Claude row as the desktop app with
      the npm install named; a CLI resolved any of the ordinary ways still ticks
      as present.
- [ ] A session whose Profile's harness resolves to one of those three is
      refused rather than started: the log names the file and the reason, and the
      Conversation carries Verkstead's own line saying the same.
- [ ] Nothing is run to find out. The recognition is decided from a platform
      value rather than a `cfg`, so every arm of it is a unit test on this
      machine.
- [ ] CONTEXT.md's Onboarding Mode entry names the third thing a row that is not
      there can say, beside the two it already names.
