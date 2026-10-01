# 02. The grilling

## What to build

A decision, and the backlog that follows from it. Read
`docs/roadmaps/language-caches/06-spike-findings.md`, which task 01 wrote, then
grill the human on it the way the `grilling` skill says, through
`verkstead ask`. The three ways it can go:

- **Build the daemon of Verkstead's own.** It would be modelled on the Compile
  Server, and would be a built-in capability the `jvm` descriptor names, since
  a descriptor is data and cannot name a command (ADR-0021).
- **Take the per-session registry instead.**
- **Leave the daemon off**, as stage 04 left it.

Put the findings to the human with the spike's recommendation, plus whatever
the design questions the chosen way leaves open. For a daemon of Verkstead's
own, those include at least: how many daemons it keeps and for which
Gradle/JDK combinations, what a session whose build matches none of them gets,
its memory and idle budget, how it is held and stopped on each platform, and
what the Sweep and a Clear of the JVM's stores do to it.

**Then write the build into this backlog.** Once the decision is made, add a
task file per slice the decision needs, and list them in `.tasks/TODO.md`
between this task and the docs task, renumbering the docs task to come last.
Follow the backlog's rules:

- sequential, with no "blocked by";
- thin vertical slices, each end to end and demonstrable;
- each small enough for one fresh session, carrying everything it needs;
- proven by real builds in two Sandboxes on Linux and by environment
  assertions on all three platforms, as the rest of the roadmap is.

Put the breakdown to the human for approval in the same grilling before writing
it. If the decision is to leave the daemon off, add no tasks.

## Acceptance criteria

- [ ] The human has decided between building the daemon of Verkstead's own, the per-session registry, and leaving the daemon off, and the decision is recorded with its reasons in the findings file
- [ ] Where building was decided, the build tasks the human approved are in `.tasks/` and listed in `TODO.md` ahead of the docs task, which is renumbered last
- [ ] Where the daemon stays off, the backlog is unchanged apart from this task's box
