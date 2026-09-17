# 03. Sessions that report through a commit

## What to build

Convert the three kinds of session the runner ends today on *committed plus
quiet* to the done signal tasks 01 and 02 built: an **inline implementation**, an
**instruction** (the hand-written work a steer into Implementing sends a session
off with), and a **fix** session (the addressing skill, sent about a red check or
feedback during a wrap-up). See `docs/adr/0018-a-session-says-it-is-done.md`.

This is the second reported failure, at its source. An inline run's *landed* is
any commit past where the Conversation's commits stood when the session started,
so a session that committed a first slice and then waited on its tests was ended
five seconds later with the rest of its work uncommitted. After this task nothing
ends one of these on the branch plus quiet: `committed_and_quiet` goes, and the
driver ends the session on an accepted signal once it is next Idle.

**What the signal is checked against, per kind:**

- **Inline and instruction** — a new commit since the session started, by the
  same marker those drivers hold today. None is refused, saying nothing has been
  committed since the session began.
- **Fix** — nothing. A fix session that finds nothing to commit is accepted: what
  judges a fix is the check on GitHub, asked again once the session is over, and
  the wrap-up's two goes at a check are the bound it already has. Demanding a
  commit would leave a fix session with nothing to fix unable to end.

The rules from task 02 hold for all three unchanged. What each driver does after
the session is over is unchanged too — an inline run going on to its pull
request, an instruction handing back to the pipeline, the wrap-up asking GitHub
about the check again. The Rescue no longer leaves one of these alone for having
committed, exactly as in task 01.

Rewrite the ending passages of the `implementing`, `instruction` and `addressing`
skills: commit, push where the skill says to, finish everything that comes after,
and then run `verkstead done`.

## Acceptance criteria

- [ ] An inline session that commits and then stays quiet is not ended, and is ended once it signals and goes Idle
- [ ] An inline or instruction session's signal with no new commit is refused, saying nothing has been committed
- [ ] A fix session's signal is accepted with no new commit, and the wrap-up goes on to ask GitHub about the check as before
- [ ] What follows each session's ending — the pull request, the pipeline carrying on, the check being asked about — is unchanged
- [ ] The three skills tell the agent to run `verkstead done` last, and none says a session ends by going quiet
- [ ] The Unix and Windows session suites pass
