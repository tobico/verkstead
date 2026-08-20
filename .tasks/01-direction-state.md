# 01. Wrap-up proposal and the Direction state

## What to build

Grilling ends by the agent's own proposal rather than a button. The bundled
grilling skill gains a closing move: once it and the human have reached a shared
understanding, it asks one final, marked Question Set that proposes wrapping up
and recommends a direction — inline, task list or roadmap — with the reasoning
behind the recommendation. Answering that Set moves the Conversation out of
Grilling and into Direction.

The workbench then shows the direction chooser: the agent's recommendation and
its rationale, with the three choices. **Roadmap is offered but disabled**,
marked as arriving in a later stage — the choice exists so the shape of the
decision is visible, but only inline and task list are selectable, and neither
does anything yet (tasks 02 and 04 wire them up).

What the Set is marked *with* is the decision to make here. The Set already
carries agent-supplied labels and prose; the marking has to be something
Verkstead can recognise on the way past without the human seeing machinery, and
without an ordinary grilling Set ever being mistaken for it.

`Lifecycle::Direction` already exists in the store — the states were laid down
in stage 01. What is missing is the transition into it, and the Moved event that
records it.

## Acceptance criteria

- [ ] The grilling skill instructs the closing proposal Set, including the
      recommended direction and its rationale
- [ ] Answering the marked Set moves the Conversation from Grilling to Direction
      and lands a Moved event on the Timeline
- [ ] An ordinary grilling Set, answered, leaves the Conversation in Grilling
- [ ] The workbench draws the direction chooser for a Conversation in Direction,
      showing the recommendation marked and its rationale
- [ ] Roadmap appears in the chooser, disabled and labelled as a later stage
