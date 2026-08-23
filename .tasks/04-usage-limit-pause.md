# 04. Usage-limit pause

## What to build

A claude account that exhausts its window mid-run pauses the Conversation and
tells the human's devices. It resumes on their say-so, or when the window
resets.

**claude waits too, and that is settled.** The version in use holds the session
and continues by itself when the limit lifts, under a setting of its own.
Verkstead does not turn that off and does not depend on it: both wait, the reset
wakes both, and Verkstead's pause is what makes the wait visible and answerable
from a phone instead of being a session that has gone quiet for no stated
reason. Nothing here reaches into the agent's own configuration.

The pause is what an unattended run owes the human: a run that stopped, said on
the Timeline, with the account it stopped on and the time the window resets
where they can be read. While it is paused nothing advances — no next Step, no
fresh session — and the Conversation carries *blocked on you*, which is the
badge on an active state rather than a state of its own.

Resuming happens two ways and they meet in the same place: the human says so
from the workbench or their phone, or the reset time passes. Neither reverts
anything; the repository is left exactly as the session left it.

Exhaustion is read from what the session leaves behind — the Capture of what it
printed and the Transcript its backend wrote — rather than from anything it is
asked. Both are already kept for every session. Recognition should survive the
wording moving, because the wording is the backend's and will move: what is
matched belongs in one place, said once, and a session that never comes back
from a limit is worth an Interruption rather than an indefinite silence.

**No auto-switching between Profiles.** An exhausted account is a wait, never a
reason to spend a different one.

## Acceptance criteria

- [ ] A session whose account exhausts its window puts a pause on the Timeline
      naming the Profile and, where it can be read, when the window resets — and
      one push goes to the subscribed devices.
- [ ] While paused nothing advances the run, and the Conversation carries
      *blocked on you*.
- [ ] Resuming by the human's press, and resuming on the reset time passing,
      both start the work again from where it stopped, with the Worktree
      untouched.
- [ ] Nothing changes the agent's own limit configuration, and no Conversation
      moves to another Agent Profile because one was exhausted.
