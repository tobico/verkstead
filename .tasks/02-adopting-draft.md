# 02. An adopting Conversation and its page

## What to build

Clicking a roadmap in the notice creates a **Conversation** in Draft against
that Repo, marked as the one adopting that roadmap, and opens it on a page
shaped for adopting rather than for grilling.

The mark is the only new thing stored, and it is deliberately thin: which
roadmap this drafting Conversation is adopting, and nothing else. The roadmap's
own content — its stages, its briefs, its boxes — is never stored; it is read
back from the repository wherever it is needed, as everything else about a
roadmap is.

The page names the roadmap and the stage that would be adopted, and carries:

- the **grilling Profile** and **implementation Profile** pickers, both fixed
  before adopting exactly as both are fixed before grilling — the implementation
  one is what the work runs under, and the grilling one is carried because later
  stages inherit both from their predecessor and a reopened Conversation grills;
- the **base commit override**, defaulting to the Repo's default branch tip;
- one **Adopt** press, which task 03 makes do something.

And it carries neither a **Brief** editor nor *Start grilling*. There is nothing
to type: the Brief is the stage brief, and it arrives when the stage is adopted.

The stage the page names is **re-read at whatever the base resolves to**, not
carried over from what the notice showed. A base override pointing somewhere the
roadmap reads differently — an unmerged predecessor's tip, say — makes the page
name the stage that is next *there*.

The branch name does not apply on this path and the page does not offer it. The
Conversation keeps the server-invented name it was started with, which is what
the sidebar row shows until the stage is adopted and the branch becomes the
stage's own slug.

Adopting is not immediate on click, because both Profiles have to be fixed first
and nothing at the Repo level supplies them.

## Acceptance criteria

- [ ] Clicking a roadmap in the notice creates a Draft Conversation against that
      Repo marked as adopting that roadmap, and opens it.
- [ ] Its page names the roadmap and its next stage, offers both Profile
      pickers, the base commit override and an Adopt press — and offers neither
      a Brief editor nor *Start grilling*.
- [ ] Overriding the base commit re-reads the stage at that commit, so a base
      where the roadmap reads differently changes the stage the page names.
