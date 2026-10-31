# 03. The gate for stages

## What to build

Where a declared stage cannot be moved at its start, what stage 04 built for a
task happens for the stage (ADR-0029, *Verkstead asks with a Question Set of its
own*), and the roadmap is told which.

- **The askable cases put Verkstead's own Set** on the stage's Timeline: devices
  of the platform linked and none permitted (*Stage 06 wants Windows. nuc-win is
  linked but not permitted.* — one *Move to* each, against *Run it here*), and
  permitted ones asleep (*…is not awake, and the work moves there by itself once
  it is.* — *Run it here* alone), and both together as one Set. The wording is
  the table's, by `docs/copy.md`; the stage is named by its label as a task is.
- **The answer.** *Move to* ticks the device and moves the stage; its arrival
  plans it. *Run it here* plans the stage at home, through the ordinary Resume,
  which reaches `plan_stage` without asking again. A sleeping device answering
  moves the stage and locks the Set, as for a task.
- **The open Set is the wait.** It holds the stage on the drivers register, so
  the stage takes a place as waiting on the human does. A Resume or a restart
  reaching the unplanned stage while the Set is open waits on it rather than
  planning or asking twice — the stage's equivalent of the inline run's
  `inline_waiting`, asked on the stage-to-plan path.
- **No device of the platform linked, or none able to take it** (each awake one
  lacking the Repo, a companion or the harness): a Notice on the stage's
  Timeline saying why, said once because a stage is placed once, and the stage
  planned at home.
- **The roadmap's start Notice says which**: that the stage waits on the human
  to say where it runs, or that it runs here and why.

## Acceptance criteria

- [ ] A Windows stage on a roadmap with a Windows device linked but not ticked
      puts the Set with *Move to* and *Run it here*, reaches the phone, and plans
      nothing until answered; *Move to* moves the stage and it is planned on
      arrival; *Run it here* plans it at home.
- [ ] A stage whose ticked Windows device is asleep is planned there once the
      device answers, the Set locked with a line saying so.
- [ ] A restart while the Set is open neither plans the stage nor puts a second
      Set; the stage holds its place while it waits.
- [ ] A roadmap with no Windows device linked plans the stage at home with one
      Notice saying so, and the roadmap's start Notice says it runs here.
