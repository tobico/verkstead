# 05. The words, and the chain joining up

## What to build

Two words join the workbench vocabulary in `CONTEXT.md`, written in that file's
own format — the definition, and the _Avoid_ line of the words it is not:

- **Adopt** — a roadmap Verkstead did not write becomes one it drives.
- **Abandoned** — a roadmap with a stage startable now and nothing driving it.

And a test proving the chain joins up. The whole claim adoption rests on is that
starting *one* stage as a Conversation is enough: that stage's own plan commit
touches the roadmap, so the existing carry-on path picks it up when the stage
settles and starts the one after it with nothing changed. Nothing needs building
for that — the point is that it already works — but nothing currently proves it
either, and it is the join between the new entry point and the unattended
pipeline everything downstream assumes.

So: carry an adopted stage through settling and show the stage after it
starting, on the existing path, with no change to the code that starts it.

`docs/adoption.md` is deliberately left alone.

## Acceptance criteria

- [ ] `CONTEXT.md` defines Adopt and Abandoned among the workbench vocabulary,
      each with the _Avoid_ line the file's format uses.
- [ ] A test takes an adopted stage through settling and shows the next stage of
      its roadmap starting, with no change to the code that carries a roadmap on.
- [ ] `cargo test` is green.
