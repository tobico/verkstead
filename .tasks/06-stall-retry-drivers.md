# 06. Retrying a stall for the runner, an inline session or a wrap-up

## What to build

What Retry means on a stall Interruption: **relaunch the driver from where the
repository stands**, chosen by the Conversation's state. Three of the four
states here; a grilling is the next task's, because it needs more than a
relaunch.

- **Implementing from a task list or a roadmap stage** — the runner resumes,
  reading the next step from `.tasks/` exactly as it always does. What is next
  is the repository's to say, and it has not changed just because nothing was
  running.
- **Implementing inline** — a fresh session on the inline prompt, with the
  human's note telling it what the dead one left behind.
- **Wrapping** — the watcher set respawned, the way a restarting server takes an
  interrupted wrap-up up again. Each of those watchers decides for itself
  whether there is anything left to do, so respawning them is safe.

Every relaunch registers a driver again, which is what makes a Conversation that
stalls a second time detectable rather than silently stuck for good. The human's
note reaches the session that runs, in the same place a retried step's note goes.

*Take over manually* and *Abort* are untouched and mean what they always mean.

## Acceptance criteria

- [ ] Retrying a stall relaunches the right thing for each of the three states,
      and a driver is registered again once it has
- [ ] The human's note reaches the session the retry starts
- [ ] A Conversation that stalls again after a Retry raises a second
      Interruption, the first having been settled
