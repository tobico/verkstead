# 14. Build a plugin

## Goal

The Plugins page has a **Build a plugin** button. A press opens a small menu of
the six Categories; picking one opens the compose page with a Brief already
written — naming the Category, its template repository and
`verkstead plugin check` — the Process set to Develop and the Repo left to the
human, a plugin being its own repository and **Create repo** the likely next
press. A Brief already on the compose page is saved as a Draft first, through
the existing Save as draft, before the button's Brief takes its place. This is
the compose page's first prefill. Behind it stand six template repositories,
one per Category, each a minimal plugin that passes the check, with a README
on the manifest and the WIT it implements.

## Decisions in force

- **A menu of the six first, then the compose page** (Set 1769, Q44). One
  general Brief naming all six and letting the interview pick was turned down:
  the Category decides which template and which WIT the Brief points at, and
  that is a fact the human has before the interview starts.
- **Process Develop, Repo left open.** A plugin is its own repository, so the
  Repo dropdown's **Create repo** is the next press; prefilling a Repo would
  guess. Develop because a plugin is a piece of work with an interview worth
  having, and because the templates make Tinker's hand-steering unnecessary.
- **A standing Brief is saved as a Draft, never discarded**
  ([ADR-0030](../../adr/0030-plugins.md), *The Plugins page*). The compose
  page keeps its draft in the browser, and a press that silently replaced it
  would lose work the human typed.
- **Templates per Category and `verkstead plugin check`** (Set 1762, Q12). The
  validator alone, and a `verkstead plugin new` that writes a skeleton, were the
  alternatives; a repository the session clones is what an agent reads best,
  and the check closes the loop before an install is tried. Where the templates
  live is settled with the human at stage start — the owner is `tobico` today,
  and a `verkstead-plugins` organisation or `tobico/verkstead-plugin-<category>`
  are the candidates.
- **The Brief is screen copy**: written per `docs/copy.md`, short and plain,
  since the human reads it on the compose page before pressing Start — and it
  is also the first thing the grilling session reads, so it names the WIT world,
  the check command and the template as facts rather than instructions.
- **This stage stands on every Category having landed** — 02, 06, 11 and 12 —
  because a template is a plugin of that Category passing the check against the
  WIT as published, and a Brief pointing at a template for a Category that
  cannot load yet would be a lie on the compose page.

## Proposed tasks (provisional)

1. **The prefill** — the compose page takes a Brief from the button and keeps
   it in the stored draft; a non-empty Brief already there is saved as a Draft
   first. AC: a press on an empty composer lands with the Brief filled and
   Develop picked; a press over a written Brief leaves a Draft on the list and
   the new Brief on the composer; the Repo is empty and Start waits on it.
2. **The button and the menu** — on the Plugins page, six rows named by
   Category. AC: each row opens the compose page with its own Brief; the menu
   names a Category only where its templates exist.
3. **The six templates** — one repository per Category, each passing
   `verkstead plugin check`, a README each. AC: cloning one and running the
   check passes; the Agent, Activity and Forge ones build a component from the
   published WIT with one documented command; the Process, Language and Skills
   ones are manifest-only.
4. **The Brief texts** — six Briefs per `docs/copy.md`, each naming the
   template, the WIT world and the check. AC: a grilling session started from
   one reaches the template and the check without asking where they are.

## Re-verify at start

- Stages 02, 06, 11 and 12 landed, so all six Categories load and their WIT
  worlds are the ones published.
- The compose page still has no prefill of any kind and still keeps its draft
  in `localStorage` under `verkstead.composing` (`web/src/workbench/composing.ts`,
  `stored()` and `keep()`), and Save as draft is still the way a Brief is kept.
- The Repo dropdown still carries **Create repo** at its foot.
- The template repositories' home has been settled with the human, and the
  Brief's wording has been read against `docs/copy.md` as it then stands.
- `verkstead plugin check` still prints its refusals the way stage 01 wrote
  them, so a template that drifts fails loudly.
