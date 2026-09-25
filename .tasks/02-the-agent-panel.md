# 02. The Agent panel on a Draft's composer

## What to build

On the saved Draft's composer the role pickers leave the setup row and move
inside a panel that drops from one trigger labelled **Agent**, so the row reads
Repo, Process, Agent. The panel is the Repo option's own pattern — a `Menu` with
`panel`, one flat card — and not a modal: the Repo option beside it is exactly
this, and two shapes in one row would be two things to learn. The pickers inside
are today's, drawn by hand because every row carries a harness mark, stacked
under their role names — Grilling, Implementation, Review for Develop. The
labels inside the panel keep the role names, because the tests, the Brief's
setup facts and the Steer form all speak them; *Agent* is the trigger's label
alone.

Which pickers the panel holds is the role table's, from task 01.

**The trigger reads** the Implementation Pairing's reading dropped to its short
form, as today's role trigger does, with that Pairing's harness mark beside it —
which is why the short form drops the backend's name. Then ` +1` for each other
role the Process uses that is picked onto a different Pairing, which is the Repo
trigger's own convention for the companions it counts. A role skipped and a role
matching add nothing. **Not chosen** stands while any role the Process requires
is empty, so the trigger says what the start press will refuse on.

The per-Repo Pairing memory and the platform default are untouched. They are
keyed by role, the pickers are the same pickers standing on the same prefill,
and each still saves itself the moment it is touched, saying its own refusals
where it stands.

The workbench and surviving suites drive those pickers through the trigger: the
panel is opened the way the Repo panel already is, and each picker inside is
then found by its label and read by its reading, as `web/tests/pickers.ts`
insists.

And `CONTEXT.md` gains an **Agent** entry for the control — its Process entry
already names **Agent** as a term with nothing behind it.

## Acceptance criteria

- [ ] The Draft composer's setup row reads Repo, Process, Agent, and opening the
      Agent trigger shows a labelled picker per role the Process uses, each
      saving exactly as it did standing in the row.
- [ ] The trigger reads the Implementation Pairing once where every other role
      the Process uses is on that Pairing or skipped, reads ` +1` for a Review
      on a different one, and reads *Not chosen* while Implementation is empty.
- [ ] Every case in the workbench and surviving suites that picked a role passes
      through the trigger, and the inert-Start case asserts the waiting text.
- [ ] `CONTEXT.md` has an **Agent** entry for the control.
