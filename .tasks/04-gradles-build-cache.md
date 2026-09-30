# 04. Gradle's build cache, where a Repo turns it on

## What to build

Settled in the grilling: **Verkstead does not switch Gradle's build cache on.**
Doing so from `GRADLE_OPTS` would override a Repo that set
`org.gradle.caching=false` on purpose. The local build cache's directory is under
the Gradle home, which task 03 shares, so a Repo that switches caching on
already gets one cache for the machine. This task proves that end to end and
changes no behaviour, unless the proof finds the cache is not where it was
expected.

Check where the local build cache really goes in the Gradle release the proofs
run, and whether a Repo's `settings.gradle` pointing `buildCache.local` at its
own directory still wins. That is the Repo's choice, and it should keep it.

## Acceptance criteria

- [ ] A Repo with `org.gradle.caching=true` builds a cacheable task in one Conversation's Sandbox. The same task in a second Conversation, in a fresh Worktree, reports `FROM-CACHE`.
- [ ] A Repo that does not switch caching on is untouched: the task runs rather than coming `FROM-CACHE`.
- [ ] The descriptor sets nothing about caching, and its comment says why.
