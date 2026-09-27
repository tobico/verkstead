# 02. The Agent control

## Goal

The three role pickers on the composer become one control, **Agent**, whose
shape is the Process's. Where the Process uses one role it is today's flat
dropdown, labelled *Agent*, offering the Pairings. Where it uses several it is
a trigger that drops a panel, the way the Repo option does, holding the role
pickers stacked under their labels — Grilling, Implementation, Review for
Develop — and the trigger reads the Implementation Pairing's reading, then
` +1` for each other role picked onto a different Pairing, or *Not chosen*
while a required role is empty. The setup row reads Repo, Process, Agent, on
the compose page and the saved Draft's composer both. With only Develop landed
the panel is the shape every draft gets; the dropdown arrives with the first
one-role Process.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *The
  composer: Repo, Process, Agent*** is the rulebook, including the role table
  that says which Process uses which roles and so which shape its control
  takes. The table lives in one place the web reads, so a later stage adds a
  row rather than a branch.
- **The panel is the Repo panel's pattern**, a `Menu` with `panel`, and not a
  modal: the Repo option beside it is exactly this, and two shapes in one row
  would be two things to learn. The role pickers inside it are today's, hand
  drawn because a row carries a harness mark.
- **The trigger's reading** is the composed Pairing reading CONTEXT.md sets
  under **Pairing**, dropped to its short form as today's trigger does, with
  ` +N` for roles that differ — the Repo trigger's own convention for
  companions. A skipped role and a matching one add nothing; *Not chosen*
  stands while any required role is empty, so the trigger says what the start
  press will refuse on.
- **Readiness and the waiting text are unchanged in substance**: the press
  still waits on every role the Process uses, and the title under an inert
  Start still names what is missing. What changes is the words, which name the
  roles the Process has rather than always three.
- **The per-Repo Pairing memory and the platform default are untouched.** They
  are keyed by role, and the panel's pickers are the same pickers standing on
  the same prefill.
- **Labels inside the panel keep the role names** — *Grilling*,
  *Implementation*, *Review* — because the tests, the composer's Brief facts
  and the Steer form all speak them; *Agent* is the trigger's label alone.

## Proposed tasks (provisional)

1. **The role table** — one module saying, per Process, which roles are used
   and which may be skipped, and whether the control is a dropdown or a panel.
   AC: Develop answers three roles and a panel; the table is what the composer
   and the readiness text both read.
2. **The panel** — the three pickers move into a panel dropping from an
   *Agent* trigger in the setup row, on both composers, with the trigger
   reading composed from the picks. AC: opening the trigger shows the three
   labelled pickers; picking the same Pairing everywhere reads it once; a
   different Review reads ` +1`; an empty Implementation reads *Not chosen*.
3. **The dropdown shape** — the same control drawn as the flat Pairing
   dropdown where the table says one role, labelled *Agent*, wired to the
   Implementation role. AC: under a one-role Process the row is a dropdown and
   the panel is never drawn; the readiness text names one role.
4. **Tests moved over** — the compose and workbench suites drive the pickers
   through the trigger, by label and reading as `web/tests/pickers.ts`
   insists. AC: every case that picked a role still passes through the panel;
   the *Start inert* case asserts the new waiting text.

## Re-verify at start

- `Menu` with `panel` in `web/src/Menu.tsx` is still what the Repo option
  uses, and still the only panel shape in the setup row.
- The Pairing reading and its short form are still composed in
  `web/src/agents.ts` and `web/src/pairing.ts`, and the trigger's ` +N` for
  companions is still how the Repo trigger reads.
- Stage 01 landed the Process on the record and the composer knows it, so the
  control has a Process to shape itself by.
- No one-role Process has landed yet, so the dropdown shape is tested against
  the table rather than against a live Process until 04 or 06 lands.
