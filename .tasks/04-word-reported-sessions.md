# 04. Sessions that report through their own words

## What to build

Convert the sessions the runner ends today on *quiet with nothing pending* — the
`proposing` rule — to the done signal: the wrap-up's **review** session, each
**comment-batch** session (the responding skill), and the session **sent to open
the pull request** after a run stopped short of its push (the submitting skill).
See `docs/adr/0018-a-session-says-it-is-done.md`.

These report through nothing on the branch: a review that fixes nothing did its
job. So quiet was the only signal there was — a minute of it, with no Set of the
session's own open — and a review that started a background check run and ended
its turn was ended mid-review, leaving what it had got to standing as the whole
of it. After this task nothing ends one on quiet: `quiet_and_nothing_asked` goes
as an ender, and the driver ends the session on an accepted signal once it is
next Idle.

**The signal is checked against nothing of the kind's own** — there is no landing
to read — so what applies is task 02's rules: uncommitted changes refuse it, and
an open blocking Set of its own is locked. What the callers do after the session
is over is unchanged, including the question both already ask: whether anything
the human accepted was left unlanded.

**The rule about a session that never said a word goes.** It existed because
quiet-with-nothing-pending is satisfied by pure silence, and a silent review would
otherwise have settled a wrap-up over a branch nobody read. No session is taken
at its silence any more — a silent one is never ended, and is spoken to by the
Rescue — so there is no silence left to tell from a review that found nothing.
Remove it and the stop it wrote. `Pace::proposing` stays as the Rescue's grace,
which it also is; say so where it is documented.

Rewrite the ending passages of the `reviewing`, `responding` and `submitting`
skills to end on `verkstead done`.

## Acceptance criteria

- [ ] A review, a comment-batch and an open-the-pull-request session that go quiet with nothing open are not ended by the driver, however long the quiet
- [ ] Each is ended once it signals and goes Idle, and is `Reviewed::Done` to its caller as a session that finished
- [ ] A review blocked on its own Set is left alone while the Set is open, as before
- [ ] No code path stops a Conversation because a session *never said anything*
- [ ] The three skills tell the agent to run `verkstead done` last
- [ ] The Unix and Windows session suites pass
