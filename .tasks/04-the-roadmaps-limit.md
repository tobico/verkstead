# 04. The roadmap's limit

## What to build

How many stages of one roadmap run at once becomes a **server setting**, three
where nobody has said otherwise, and task 03's constant reads it.

On the **Settings page as a card of its own**, with room beside it for the
server-wide limit stage 05 adds — one section about how much Verkstead runs at
once, rather than a field tucked inside a section about something else. It takes
a whole number of at least one; what a limit that would start nothing means is
not a question worth having, so the page refuses it.

Written into the settings file the way everything else there is, and read
**afresh at every start** the way the git author and the build cache are: an
absent key, an absent file and one nothing can parse all mean three, and a change
reaches the next start without a restart.

## Acceptance criteria

- [ ] Set to one, a declared roadmap runs its stages one at a time in the order
      its declarations allow.
- [ ] A stage blocked on the human holds its place, so a ready stage does not
      take it.
- [ ] A change takes effect at the next start and stops nothing already running;
      a settings file with no key, or one nothing can parse, means three.
