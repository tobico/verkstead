# 01. The memory switch, end to end on Claude

## What to build

Every Agent Profile gains a **memory** switch, and Claude's built root obeys it.
This is brief tasks 1 and 5 merged, so the checkbox does something the moment it
lands.

**Stored on the Profile.** A column on the `profiles` table, added in the
migrations module the way other tables gain columns (`ADD COLUMN … NOT NULL
DEFAULT 1`), so every Profile that already exists reads as on. The profiles
table is *not* rebuilt for this, and no side table is added. The human chose
this. The comments in the store saying "there is no migration machinery to alter
`profiles` with" are out of date, so correct them where this touches them.

**Over the API and on the form.** The switch is part of what a Profile is read
and saved as, in the generated wire types, and drawn as **one checkbox on the
Profile form for every harness**, checked by default for a new Profile. Off means
the session's memory store is its own and empty: fresh memory, and none of the
human's transcripts reachable. The picker and the settings card do not show it.

**Claude's root obeys it.** Stage 01 joins the account's `projects/` entries for
the Repo's main checkout and for the Worktree into the root unconditionally. Put
that join behind the switch:

- **On:** exactly what stage 01 built. The two entries are made in the account
  where missing and joined read-write.
- **Off:** nothing under the account's `projects/` is joined or made. The root's
  `projects/` is the root's own directory and starts empty. Claude writes the
  session's transcript there. The credentials link, the written `settings.json`
  and the `.claude.json` copy and merge-back are the same either way.

On Windows, an off root grants no entry on the account's `projects/` entries.

**Discovery reads where the store really is on the host** (the human decided
this; it corrects the brief). On Linux a join is a bind inside the session's
namespace only, so on the host the root holds empty mount points and the log is
in the account. So the transcript reader looks in the **account's store when the
switch is on**, as today, and in **the root on the host when it is off**
(`homes/<id>/.claude/projects/`), on every platform. Claude's glob for
`<session-id>.jsonl` is otherwise unchanged. The Transcript already keeps lines
as they arrive, so the root being emptied when the next session starts loses
nothing.

The Linux root-sharing rule (a launch into a Conversation with something still
running in its root shares that root rather than emptying it) applies unchanged.

## Acceptance criteria

- [ ] A Profile that existed before the migration reads as memory on. A database made fresh has the column, and running the migration twice is harmless.
- [ ] The Profile form draws the checkbox for all four harnesses. Saving it off reads back off after a reload. The picker and the settings card look as they did.
- [ ] With memory off, a Claude session's root holds an empty `projects/`. The account's `projects/` is not visible inside, and no entry is made in it. The three boundary suites (Linux, macOS, Windows) assert it.
- [ ] With memory off, the session's `<session-id>.jsonl` is found under the root on the host, and the Transcript draws on the Timeline.
- [ ] With memory on, behaviour and the boundary suites are unchanged from stage 01, and the log is found in the account.
- [ ] The wire types are regenerated, and the store, server and web tests pass.
