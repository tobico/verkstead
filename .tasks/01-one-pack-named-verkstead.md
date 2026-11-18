# 01. One pack named `verkstead`

## What to build

The thirteen bundled skills move from `data_dir/skills/<skill>/` to
`data_dir/skills/verkstead/<skill>/`, and every path that names a skill says
the pack: a session is told to read `/verkstead/skills/verkstead/<skill>/SKILL.md`
and that is where the file really is. The layout move and the path change are
one task because neither can land without the other — every prompt and test
breaks in between.

Where the skills still live in the repository is unchanged here (moving them
under `plugins/` is task 02's). The embedded files are installed under a
`verkstead/` directory by the startup rewrite in `skills.rs`
(`Skills::installed`), which still wipes the whole skills directory and writes
it again. `INSIDE` stays `/verkstead/skills` and the sandbox still binds the
whole skills directory read-only there — per-pack binds are task 03's.

What names a skill path today, all of which must say the pack:

- the per-skill consts and `Skills::named` in `crates/server/src/skills.rs`,
  and every prompt builder that uses them;
- the grilling skill's two absolute cross-references, to `breaking-down` and
  `staging` — the only skill that names another by path. They go through
  `said_at`, so the Mac and Windows real-path rewrite (where nothing is bound
  at `/verkstead/skills`) must still produce a path that exists;
- the Windows grants naming `C:\ProgramData\verkstead\skills` and the Mac
  seatbelt reach, wherever a test or a policy names a skill below that;
- the test fixtures: `crates/server/tests/suite/sessions.rs` and `sandbox.rs`
  (the in-session `ls /verkstead/skills` check now lists packs), the unit
  tests in `skills.rs`;
- CONTEXT.md's **Skill** entry and any doc naming a skill by path (roadmap
  briefs written as history are left alone).

The human's own notes and memory files quote the old paths; say so in the
pull request rather than changing them.

## Acceptance criteria

- [ ] A session reads `/verkstead/skills/verkstead/grilling/SKILL.md`, and
      `/verkstead/skills/grilling/SKILL.md` is not there; on a Mac and on
      Windows the rewritten path a prompt names is the file's real path
- [ ] The startup rewrite still wipes and rewrites: a stray file left under
      `data_dir/skills/` before a start is gone after it
- [ ] The grilling skill's references to `breaking-down` and `staging` name
      the pack and open
- [ ] A grep for `/verkstead/skills/<skill>` with no pack finds nothing outside
      history (old roadmap briefs, ADR text quoting the past)
