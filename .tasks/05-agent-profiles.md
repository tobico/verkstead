# 05. Agent profiles

## What to build

The second step as `02-accounts-found` and `02-none-found-naming-one` draw
it. The behaviour is today's: a found account is offered ticked where its
harness is present, the press saves the ticked ones as unnamed profiles with
every model of the harness, and it needs a profile, ticked or saved from the
form.

**Found accounts are cards.** The label over them reads *Found in ~* with the
count beside it. Each card has the harness mark, the name, the paths in mono
under it, and the box on the right; a ticked card takes an ember border, and
the whole card is the press. An account whose harness is missing is drawn
quiet with a **Not installed** pill after its name, the sentence the viewer
already says for a missing harness, and a **Go to step 1** link on the right
that opens step 1. Go to step 1 is drawn only while step 1 is met, which is
the rail's own rule and the only time there is a way back.

**Nothing found is a card too**: *No account in ~ yet* with a green **Watching**
dot and the frame's cadence behind it, the line *Log one in on <machine> and it
appears here.*, and one row per runnable harness with its mark, name and the
login command in a copyable mono block.

**Name an account yourself** is a dashed card with a plus, a title and *For an
account kept outside this server's home.* A press opens it in place into the
profile form, which stays open as today. **The form is restyled once** for the
wizard and the settings page alike (settled over a wizard-only variant): the
agent picker is a segmented control of the four harnesses with their marks,
the models are ember chips with a tick on the picked ones, and the free model
id field and Add stay under them. The fields keep their labels and notes; Save
profile keeps its place at the bottom. The profile pane's own tests pass
against the new markup.

**The button** reads *Keep N profile* or *Keep N profiles* for N ticked
accounts, and **Next** with none ticked once a profile stands saved. Disabled,
its reason beside it reads *Tick an account, or save one*.

## Acceptance criteria

- [ ] Found accounts are cards with the tick on the right and an ember border
      when ticked; a missing harness is drawn quiet with Not installed and a Go
      to step 1 that is drawn only while step 1 is met and opens it.
- [ ] Nothing found draws the Watching card with one copyable login command
      per runnable harness; the dashed card opens the profile form in place.
- [ ] The profile form draws a segmented agent picker and model chips in the
      wizard and on the settings page, and both suites pass; the button reads
      Keep N profile(s) with its reason beside it when held.
