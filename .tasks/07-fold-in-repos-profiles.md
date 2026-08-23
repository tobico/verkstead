# 07. Fold repos and profiles into /settings

## What to build

The profiles list and the repos list move onto the `/settings` page, so one
page holds everything the human configures: the token, the git author, the
Agent Profiles and the registered Repos. The `/profiles` and `/repos` routes
are removed — not redirected — and every link and navigation entry that pointed
at them points at `/settings` instead.

All existing behaviour of both lists comes along unchanged: registering a Repo,
creating, editing and deleting a Profile, and whatever inline affordances the
lists carry today.

## Acceptance criteria

- [ ] One `/settings` page holds settings, profiles and repos, and both lists
      keep their full behaviour
- [ ] `/profiles` and `/repos` answer with the no-such-page fallback, and no
      link or navigation entry still targets them
