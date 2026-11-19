# 05. The stage runner

## What to build

The stage runner as a component in the built-in Activities plugin, declaring
the `conversations: start` capability from task 04. **It keeps today's shape**
(Set 1839, Q1): it runs whenever a stage could start, starts every ready stage,
says what waits, and ends. It is not a long-lived Activity waiting on its
stages. The host starts it where `continuing::reading` is called today:

- a stage's (or the roadmap's own) wrap-up settling;
- the places look (`places.rs`) when a place frees, at the foot of a chain;
- a press: *Continue a roadmap*, Start, Resume (lifting the limits, task 04);
- a home device hearing a stage's report from another (`continuing::reported`).

Only home drives a roadmap (ADR-0029), so the host starts it only there.

What moves into the component is the reading, done through tasks 01 and 04:

- the index and the stage briefs at the top of the chain (`continuing::declaring`
  and `stages::next_stage`);
- the declarations: `after NN`, `no dependencies`, `on <platform>`;
- the refusal of a roadmap declaring on some lines and not others
  (`stages::misdeclared`);
- readiness from the boxes and the record together (`stages::ready`);
- where each stage is cut from (`continuing::cut_from`);
- whether `git-workflow.md` records stacking, which only changes the Timeline
  sentence;
- a start per ready stage, and the notice naming the stages that wait for a
  place.

Each start is its own act, so one stage's refusal does not stop the others.

**Outcomes**: `finished` once every stage is ticked and settled, at which the
host tells the devices the roadmap is done as it does now; `continued`
otherwise; and `steered`.

## Acceptance criteria

- [ ] A roadmap with declarations runs stages side by side up to
      `roadmap_stages`, and a held stage starts at the next look after a place
      frees
- [ ] A stage joins the chain before its pull request opens (with task 03)
- [ ] A roadmap declaring on some lines and not others is refused as today
- [ ] The last stage settling finishes the roadmap and the devices are told;
      the Plugins page lists *Stage runner* under Built-in
- [ ] The sessions suite's roadmap tests pass without being rewritten
