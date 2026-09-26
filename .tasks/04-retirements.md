# 04. Retirements

## What to build

**The old way in goes, now that the Process is the way in.** Everything here was
the *Wrap up a pull request* path, and a Review Conversation is what that path
was for.

**The menu level.** *Wrap up a pull request* comes out of Other actions, leaving
*Continue a roadmap* as its one level — still drawn as the menu it is, greyed
rather than hidden while there is nothing under it, because the reason there is
one menu with a level per action is that a second dropdown coming and going with
a list was the thing it replaced. The design document's own paragraph is that
record and is corrected with it.

**The list behind it**, and the `gh` read behind that: the endpoint that answered
every open pull request in every registered Repo, the per-Repo `gh pr list` it
ran, the rows it drew, and the read of which pull requests are already held that
was only ever for those rows. Opening the compose page makes no request for open
pull requests at all — the page opens without waiting on GitHub.

**The start that loaded one.** The endpoint that made a Draft holding a pull
request, the compose page's card over its box, the band across the composer that
named what was held, the *Wrap up* press on the draft's own pane, and the pane
behind that press. A Review draft has a Brief and a Target and one *Start work*
press, like every other Process.

**And the pull request's own words as a Brief.** Loading one used to prefill the
box with its title and description; nothing does now. The Brief is the human's,
and nothing about the pull request is touched.

**What stays.** The record of what was taken up stays and is written by the
Review start. The reading that gives a Conversation holding one the **Review**
Process stays too, so a Conversation from before draws and reads as what it
always was — and a **Draft** from before that holds one takes it as its target
where nothing else names one, so its Start is the new one and it wraps up the
pull request it was made for.

**The tests go with the code.** Every test and fixture asserting the retired
endpoint, the retired start, the level and the press either goes or is rewritten
against the Review path — a fixture nothing serves is a fixture that will be
believed later.

## Acceptance criteria

- [ ] Other actions offers *Continue a roadmap* and nothing else, greyed while
      there is nothing to continue, and the design document says so.
- [ ] Opening the compose page makes no request for open pull requests, and the
      endpoint, its `gh` read and its rows are gone.
- [ ] A draft can no longer be made holding a pull request: the start endpoint,
      the compose page's card, the composer's band and the *Wrap up* press are
      all gone.
- [ ] A Draft from before that adopted a pull request draws as a Review with that
      pull request as its target, and its Start takes it up through the new path.
- [ ] Nothing in the test suites still asserts the retired level, endpoint,
      start or press.
