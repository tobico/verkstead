# 05. A late sign-out resumes by itself

## What to build

A session can sign out after the human has already logged in again — it was
built before the login. When the watcher fires and the account's login file
has been replaced since that session's root was built, the run resumes straight
away instead of stopping for a login; the Timeline says why.

## Acceptance criteria

- [ ] A sign-out whose account login is newer than the session's root resumes
      without asking
- [ ] A sign-out whose account login is unchanged stops as Signed out as before
