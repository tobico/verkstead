# 08. Models and the Guide

## What to build

**Models.** Each harness answers with its declared models (id and name), its
`default`, and its optional per-role defaults for `grilling`, `implementation`
and `review` (Set 1786, Q4). The five built-ins answer through the trait with
what `KNOWN_MODELS` and `pairing_defaults.rs`'s `model()` hold today. Claude
grills on Fable 5 and builds on Opus 5, and the others keep their own. A plugin
answers from its manifest.

- The Profile form offers the harness's declared list.
- `pairing_defaults` reads the declared default for the role. Plugin harnesses
  come after the five built-ins in its fixed preference order.
- The viewer's `KNOWN_MODELS` goes. Its model-alias recognition goes with it,
  or moves to the harness entry if something still needs it.

**The Guide is fetched** (Set 1764, Q18). `verkstead guide` asks the server for
the *Running the ask* section of the harness `VERKSTEAD_AGENT` names, and
prints the core with it spliced in at the marker. Where the server cannot be
reached, it prints the core alone, as a Guide printed outside a sandbox always
has. The five built-ins' sections are the `crates/cli/guide/running-*.md` files
today; they move server-side and are answered through the trait. The
`RUNNING_*` constants and the CLI's branch on the agent type go.

## Acceptance criteria

- [ ] The Profile form for each built-in offers the same models it did. A fresh
      Repo's default Pairing for each role is unchanged. A plugin harness's form
      offers its manifest's list, and its default is the manifest's.
- [ ] `verkstead guide` inside a stub plugin's session prints its manifest's
      *Running the ask* section. Inside a built-in session it prints the same
      Guide as before.
- [ ] Outside a sandbox, or with the server unreachable, `verkstead guide`
      prints the core alone and exits 0.
