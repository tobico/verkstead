# 05. A session runs inside an Apple sandbox

## What to build

The first session that runs on a Mac. Everything the Sandbox is for holds, and
the mechanism underneath it is Apple's rather than bubblewrap's.

**Amend ADR-0012 first.** It says sessions stay Linux-only, said plainly in the
UI, ported later platform by platform, and this stage reverses that for macOS.
The amendment goes into the ADR in place rather than into a new one, and it says
what was decided here: the macOS port lands now, on `sandbox-exec`; the boundary
there denies rather than hides, so a session can see the machine it cannot
touch; and `sandbox-exec` is formally deprecated by Apple with no supported
replacement available to an unsigned app, which is a risk accepted rather than
overlooked. Windows is untouched by any of it and stage 05 still builds the
sessions-need-Linux state for itself.

**Then the seam.** What a session may reach is one description, said once, and
rendered two ways: as bubblewrap's flags on Linux, and as a deny-by-default
policy on macOS. Neither platform's rendering is the description, and nothing
above the Sandbox learns which one it got.

**Then the first profile**, covering the part of the surface that makes a
session a session: the Conversation's Worktree and the Repo's common git
directory writable, the system read-only, the machine otherwise refused, the
network shared with the host whole and unfiltered as it is on Linux, and the
session started in the Worktree with a cleared environment.

The rest of the surface — HOME, the account, the skills, the `verkstead`
binary, companions, configured binds, the build cache — is the two tasks after
this one. This one is the seam and the floor under it.

**Proved by a probe, not by reading the policy back.** The Linux suite's own
documentation says why: a test that asserts the flags asserts itself, and goes
on passing while the mechanism changes what one of them means. So the macOS
surface is settled the same way — a command inside a real sandbox trying to
reach something and reporting what happened — which means the suite can only
run on a Mac, and CI gains a `macos-15` job to run it.

## Acceptance criteria

- [ ] ADR-0012 carries the amendment, saying what was reversed and what was
      accepted with it
- [ ] A probe run inside a real sandbox on a Mac reports the Worktree and the
      git directory writable and the rest of the machine refused
- [ ] `ci.yml` runs the macOS boundary suite on a `macos-15` job
- [ ] The 36 Linux boundary tests are unchanged and still pass, and no Linux
      behaviour moves
