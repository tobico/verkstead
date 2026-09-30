# 04. Codex, and the id nobody named

## What to build

The first harness Verkstead cannot name, which is the harder half of resuming
one.

**Codex takes no session id at launch**, so nothing Verkstead knows before the
session starts names its log. What identifies a rollout afterwards is what the
session wrote in it about itself, and the Transcript search already finds it on
that basis: the log under the account's store that names this Worktree and
appeared after this session was launched. The rollout also names **itself** —
its own session id is in it — and that is the id `codex resume` takes. So the
work is to write it down: when the search finds the log, the id it names goes
onto the record beside the session, exactly where the id Verkstead picked would
have gone for the two backends that take one. From there task 01's lookup is
unchanged, and a device that has never seen this session can resume it.

Reading that id is reading somebody else's file format, so it belongs where
every other reading of one does — in the crate that parses, beside the reading
that already says whose session a rollout is. Nothing else here parses a line.

**Its resume is a subcommand rather than a flag**, which is the first line in
the table whose shape changes rather than growing an argument: the subcommand
comes straight after the binary, the session id and the prompt are its
positionals, and the model flag and every configuration override the ordinary
line carries — the credential store file-backed, the Worktree pre-seeded as
trusted — have to come across onto it or a resumed session comes up logged out
and sitting on a trust prompt.

**Its store travels but its cwd does not.** The memory sync carries the
rollouts this Conversation's sessions wrote, picked out of one flat directory
by the Worktree's name — but the path inside the file is the sending device's.
Codex filters a resume by the directory it is started in, so either the
rollout's own record of where it ran is brought up to date as it lands, or the
resume is asked in a way that does not filter on it. Whichever is chosen is a
decision about writing into somebody else's file format, and the one that does
not write into it is worth the try first.

## Acceptance criteria

- [ ] The session id a rollout names for itself is written onto the record as
      the Transcript search finds the log, and is there for a device that never
      ran the session.
- [ ] A codex session transferred mid-turn resumes into the same rollout on the
      receiving device, under the account's login and with the Worktree
      trusted.
- [ ] The Transcript of the resumed session opens where the carried rollout
      ended.
- [ ] A rollout that was never found, or whose id could not be read, falls
      through to Verkstead's Resume.
