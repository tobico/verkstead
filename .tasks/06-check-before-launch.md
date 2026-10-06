# 06. Check before launch

## What to build

Before a Claude session is launched under a local Profile, `claude auth status`
is asked of its account. No login means the launch is refused before a session
starts, and the Profile reads as broken with a new reason — signed out — on its
row and in the refusal. The card's Log in button fixes it, and a login made
there resumes nothing new (no run was started) but clears the broken reading.

## Acceptance criteria

- [ ] A launch under a Claude Profile with no login is refused and no session
      starts
- [ ] The Profile row and the refusal show the signed-out reason
- [ ] After a login from the card, the Profile is no longer broken and the
      launch goes ahead
