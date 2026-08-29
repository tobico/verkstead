# 01. Known models constant and the profile model picker

## What to build

A single web module holding the known Claude models — each as an id and a
pretty name, `claude-opus-5` reading "Opus 5", `claude-fable-5` reading
"Fable 5" — with a `prettify(id)` helper that returns the pretty name for a
known id and the id itself, unchanged, for anything the list does not know.
The fallback matters: the list *will* go stale the week a new model ships, and
an unknown id must degrade to legible rather than to broken.

The profile settings form then offers those known models as picks for a
profile's enabled models, replacing the type-one-id-per-line textarea as the
ordinary path. A hand-entry escape hatch stays beside the picks, so an id the
constant has not learned yet can still be added by hand — this was settled
deliberately against the form's previous free-text-only rationale. A profile
saved earlier with hand-typed ids must round-trip: its known ids show as picked,
its unknown ids show via the escape hatch, and saving changes nothing the human
did not touch.

Nothing on the server changes: the wire carries model ids exactly as before,
and pretty names are the viewer's alone.

## Acceptance criteria

- [ ] One module exports the known Claude models with pretty names, and `prettify` falls back to the raw id for unknown ones
- [ ] Profile settings configure enabled models by picking known models, with a hand-entry way to add an id the list does not know
- [ ] A profile saved under the old free-text field loads and saves without losing or altering its models
- [ ] Web tests cover the prettifier fallback and the picker round-trip
