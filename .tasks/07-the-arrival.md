# 07. The arrival

## What to build

What happens at each end of the move once everything has crossed, and the
demonstration that the whole of it works across two operating systems.

**Before anything goes over**, the Profile's account is put back where it
belongs. A session away from home keeps a mirror of its account's login and
configuration and of this Repo's memory entries, and writes both back to the
home device as the session ends — stage 08's machinery, unchanged. The move
runs at the turn's end, which is exactly when that write-back happens, so the
ordering is the thing to get right: the account is home before the slice and
the bundle leave, or a transfer would race the write-back and the far end would
launch under a login the source had not finished returning.

**On arrival**, the far end presses Resume for itself. Resume is the one
standing way in — it asks what *ought* to be running now, from the lifecycle
the Conversation is in and what the branch has written, and starts that — which
is exactly the question a transferred Conversation poses. A fresh session
re-primed from the record is what that gives, which is Verkstead's own Resume
rather than the harness's; continuing the agent's own context where the harness
has one is stage 10 and is not this stage's.

And a Notice on the far end's Timeline says where the work came from, so the
record reads as one story across the two machines rather than as a Conversation
that appeared from nowhere.

**The demonstration is this task's**, and it is the stage's own: a Conversation
with a session running, moved between two devices on two operating systems, and
read back on both ends. Line endings, path separators and the Worktree's new
path are all in the way of that working, and they are why the demonstration
crosses a real OS boundary rather than running twice on one.

## Acceptance criteria

- [ ] A running grilling on the source continues as a re-primed grilling on the
      far end, started by that device's own Resume.
- [ ] The Profile's memory and login are written back to their home device
      before the slice and the bundle leave, and a session launched on the far
      end runs under the account the source had.
- [ ] The far end's Timeline carries a Notice saying which device the work came
      from, and the source's says where it went.
- [ ] Demonstrated end to end between two devices on different operating
      systems, with the Timeline, the Worktree and the working changes read back
      on the far end.
