# 07. Its login

## What to build

Let a plugin harness that declares a login line be logged in through its
component's `login` export (stage brief, *`login` drives the dialogue; the host
owns the console*). ADR-0022's `Speaking` table — what the harness prints
before the address, what it says to a cut code, when it gives up — becomes the
export's behaviour.

The host runs the declared login line into the Profile's account, the way it
runs Claude's and agy's today:

- on pipes, or on a pseudoconsole where the platform requires one
- feeding the export what the harness writes

The export answers with the address to show, the code to type, the words to
say, or that the login is over. The Log in press, the address card and the
code field in the viewer are the ones the built-ins already use. They are drawn
for a harness whose list entry says it logs in.

A harness whose manifest declares no login is one Verkstead does not log in,
exactly as Codex, Grok and OpenCode are today: no Log in press, and no login
probe.

Windows runs agy's login on a pseudoconsole the launcher makes as the session
account. If the stub's Windows path needs proving there, `verkstead transfer
winvm` moves the work to a permitted Windows device.

## Acceptance criteria

- [ ] A stub plugin whose login prints an address, then asks for a code, is
      driven to signed in from the Profile form. The address is shown, and the
      typed code reaches it.
- [ ] A cut code and a give-up are said as the export words them.
- [ ] A manifest with no login has no Log in press, and no login probe runs
      for its Profiles.
