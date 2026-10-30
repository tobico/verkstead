# 03. The fallback Notice

## What to build

The platform is a preference. Where a declared task cannot be placed (task
02), the driver writes a **Notice** naming the task and the platform, and then
launches the task where it is. Stage 04 turns the askable cases into a Question
Set of Verkstead's own, so give each case its own sentence and its own place in
the code for stage 04 to replace:

- **No device of that platform is linked.** This includes a Verkstead with no
  cluster at all. Stage 04 keeps this one as a Notice.
- **Devices of it are linked, none permitted.** Name them.
- **Permitted ones of it, none of them able to take it.** Name each with what
  its preflight found: not reachable, or what it lacks, in the words the
  Transfer dialog's findings use.

**A roadmap stage's tasks are held** (Set 1543, Q4). Placing a declared task
moves its Conversation, and a stage's Conversation moved off its roadmap's home
device goes dark to that roadmap's scheduler until stage 05 lands. So for a
Conversation that is a stage of a roadmap, the driver does not place: it writes
a Notice of its own saying the task wants that platform and why it runs here,
and launches it here. Stage 06's brief says to lift the hold. Keep it to one
clearly named check in the placement so the lifting is a deletion.

The words follow `docs/copy.md`: a full sentence naming the task, the platform
as the human writes it (*Windows*, *macOS*, *Linux*), and the device by its
name.

## Acceptance criteria

- [ ] Each of the three cases writes its own Notice naming the task and the platform, and the task's session then launches here. Each is covered by a test.
- [ ] A declared task in a Conversation with no cluster gets the *no device linked* Notice and runs.
- [ ] A declared task in a roadmap stage's Conversation is not moved even with a ticked, awake device of its platform; its Notice says so, and it runs here.
- [ ] A Resume that reaches the same task again writes its Notice again rather than nothing.
