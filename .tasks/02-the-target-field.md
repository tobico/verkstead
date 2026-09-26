# 02. The Target field

## What to build

**A target of its own, so that something which is not a pull request can be
named.** A pull request URL or `#number` is unambiguous anywhere in prose, which
is why the Brief can be read for one; a bare branch name is not, so it goes in a
field.

**A Target field on the record**, kept the way a Process is — a row of its own
beside the Conversation rather than a column on it, absent until something is
typed. It is a Draft's to change and refused past drafting on the Branch field's
own rule: drafting, and no worktree cut yet. Its own endpoint, answering its own
refusals; nothing about it goes through the branch rename, which is a different
fact and would refuse a URL over its colon anyway.

**Drawn in the Repo panel**, under the Branch field, for Review and for no other
Process yet — which Processes draw it is a fact about the Process, written where
the role table is written, so that Fix Merge Issues adds itself to that list when
its stage lands. It is labelled *Target* and reads *Pull request or branch*.

**The Brief fills it while it is empty.** Saving a Brief whose prose holds a pull
request URL or `#number` writes the first of them into a Target that has nothing
in it, and never over what the human typed — so a branch somebody typed survives
a URL arriving in the Brief afterwards, and a human who typed a URL is not asked
to type it twice. The field then holds what Start will read, rather than the page
showing empty over a value the press would find.

**Start reads the field, once.** A Review's press takes the Target as it stands
and decides from its shape which of the three it is: a URL or a `#number` is a
pull request, resolved and taken up exactly as it is now. The reading out of the
Brief stays where it is as the thing that fills the field; the press has one
place to look.

**Readiness gains it.** A Review's Start is inert while the Brief is empty, while
the Target is empty, or while either Pairing is unanswered, and the sentence
under the pointer says what is missing. The refusal for a press that gets through
anyway stays — the page's copy of the world is only as fresh as its last read.

**And the base picker follows what the field holds.** A target that reads as a
pull request hides it, GitHub's base being the fact; anything else keeps it,
because a branch's base is what its pull request will be opened against — which
is what the task after this one does with it.

**The compose page too.** The field stands in its Repo panel beside the branch
and the base, held on the device like everything else there, filled from what is
being written in the box by the same reading, and put on the draft the press
creates in the create replay — so a Review composed there arrives on its own page
holding the target the page held, or carrying the refusal where the server would
not take it.

## Acceptance criteria

- [ ] A drafting Review takes a pull request URL in its Target — the value the
      Branch field answers `NotABranchName` to — and takes a bare branch name
      just as readily; past drafting the field is refused and not drawn.
- [ ] Writing a Brief that names a pull request fills an empty Target with it,
      and leaves a Target somebody has typed in exactly as they typed it.
- [ ] A Review's Start is inert while its Target is empty, saying so, and is
      ready once the Brief, the Target and both Pairings are answered; pressing
      it takes up what the Target names.
- [ ] The base picker is drawn for a Review whose Target is a branch and not for
      one whose Target is a pull request.
- [ ] A Review composed on the compose page arrives as a draft holding the Target
      that page held, and a Target the server would not take is carried to that
      draft's own composer as a refusal.
