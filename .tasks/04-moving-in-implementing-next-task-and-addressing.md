# 04. Moving, in implementing, next-task and addressing

## What to build

The three skills that build say **CI confirms; it does not discover**
(ADR-0029, *What the skills say*): where the build, the suite or the failing
check runs only on another operating system, the session moves to a device of
that operating system and runs it there — with `verkstead transfer` where the
prompt marks the device permitted, and asking the human with `verkstead ask`
where it does not (the route task 02's Guide section describes). Pushing and
reading CI is how a fix is confirmed, never how a platform's failure is found.

- **Implementing** and **next-task** (`crates/server/skills/`) say this where
  they talk about running the repository's tests.
- **Addressing**'s failed-check step — *run the thing that failed* — gains
  **where**: on the operating system the job ran on, moving there where
  permitted and asking where not, never fixing from the log alone. The move is
  named **before** the fix.
- **No device is inferred from a job name**: *Rust (windows-latest)* is a name,
  not a declaration; the agent reads the failure and decides which operating
  system it needs.

Point at the prompt's devices section and the Guide's *Moving the work to
another device* rather than restating them. Keep skill-content tests passing,
adding phrase checks where the suite already checks these skills' words.

## Acceptance criteria

- [ ] A fix session for a failing Windows check reads in the addressing skill
      that it is to reproduce the failure on Windows — moving where permitted,
      asking where not — before fixing, and never from the log alone.
- [ ] Implementing and next-task say CI confirms and does not discover.
- [ ] None of the three infers a device from a job's name.
