# 13. New-conversation menu

## What to build

The always-open new-conversation form at the top of the sidebar — a repo
picker and a Start button — becomes a **dropdown menu button**. Press the
button, a menu drops with the registered repos; click a repo and the
conversation is created there and navigated to, exactly what the form's
Start does today.

The abandoned-roadmap reminders move into this menu too. Below the repos, a
separated group — "Adopt a roadmap" — lists each roadmap nothing is driving,
flat: one line per roadmap naming it and its next stage, and clicking it
starts the adoption exactly as the sidebar notice does today. (Nesting the
roadmaps under their repo's own entry was offered and declined.) The notices
leave the sidebar entirely; that they are no longer always in view is the
point, and the existing no-dismissal principle survives as the group being
in the menu every time it opens.

With no repos registered the menu still opens and points at Settings, as the
empty form does today. The menu should close on choice, on escape and on a
press outside, and be operable from the keyboard.

## Acceptance criteria

- [ ] The form is gone; a button drops a menu of repos, and clicking one
      creates the conversation and navigates to it
- [ ] An "Adopt a roadmap" group beneath the repos names each abandoned
      roadmap with its next stage and starts the adoption on click, and no
      notices remain in the sidebar
- [ ] With no repos the menu points at Settings, and the menu closes on
      choice, escape and outside press
