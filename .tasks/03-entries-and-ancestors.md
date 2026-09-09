# 03. Entries for an account, with a step through every ancestor

## What to build

The entries a Surface comes to stop being written for a container's identity and
start being written for whatever SID they are given — which after this task is
still the container's, because the switch is task 04. What changes here is the
list itself, and one thing that is genuinely new.

**The new thing is the ancestors.** *Reaching* a path deep under the human's
profile needs no entry on the directories above it: an ordinary account holds
the privilege that skips the traverse check, which is why a container never
needed them. But *resolving* a path walks its prefixes and asks each one for its
attributes, and an agent resolves a path before it reads it — so a session that
is granted its Worktree and nothing on the way to it refuses its own Worktree
the moment it checks. The fix is one entry on each directory along the way:
read-attributes and traverse, **not inherited**, which says nothing whatever
about what is inside. The human's profile stays unlistable in the same breath as
being walked through.

Not every ancestor can be written and that is an answer rather than a fault: the
directories above a human's profile are the machine's own and already let
`Users` step through them, so an entry there is refused and is not needed. What
matters is that the ones Verkstead *can* write are written, and that every one
it wrote is recorded so it comes off again with the rest — an ancestor entry
left behind is exactly the kind of thing the startup sweep exists to catch.

The list of paths an ancestor step is wanted for is a fact about the description
rather than about Win32, so it is worked out where the rest of the entries are
worked out and read by tests on every platform. Only the writing is the Windows
arm's.

## Acceptance criteria

- [ ] The entries a Surface comes to are unchanged except that whose SID they
  name is now given rather than assumed
- [ ] Every granted path also yields a not-inherited traverse entry for each
  directory on the way to it, in a shape a test on any platform can read
- [ ] An ancestor that cannot be written is not a failure, and the run says how
  many were written and how many were refused
- [ ] Every ancestor entry written is recorded beside the others, and is revoked
  with them
- [ ] Under the account, a path deep beneath the human's profile resolves — and
  the profile itself is still refused a listing in the same run
