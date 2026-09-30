# 07. The docs

## What to build

What an installer and a reader of this repository are told about the eleven
stores, and the corrections this stage's work makes necessary in what stage 01
wrote.

**The adoption doc** gains what is shared per tool — a row per tool under its
ecosystem's entry, saying which directory moves and whether the compiled half is
shared as well — and says plainly **what a shared writable store means**: one
session can plant a package another installs. That is accepted rather than
mitigated; it is already true of Rust's, and the machine is one person's. This
is the stage where it becomes true of more than Rust, which is why it is said
here.

**Three things stage 01 wrote are now wrong**, and this task is where they are
put right:

- Both the adoption doc and `CONTEXT.md` illustrate `{stores}` with a `node`
  entry setting `PNPM_HOME`. `PNPM_HOME` is where pnpm puts global *binaries*,
  not its content-addressable store — an installer who copies that example gets
  nothing. It is also no longer a hypothetical, `node` being a built-in now, so
  the illustration needs a different language as well as a correct variable.
- Both say that with Rust the only language built in, nothing names `{stores}`
  and no session is opened onto it. Three of these tools name it.
- `development.md`'s `config.yaml` example and its settings-API transcript both
  show a server whose only language is Rust.

The four entries are the grammar's worked examples now, in the way Rust's was:
whatever the built-in YAML says is what the docs quote, so keep them in step.

## Acceptance criteria

- [ ] The adoption doc lists what is shared per tool, and says one session can
      plant a package another installs
- [ ] The `{stores}` worked example in the adoption doc and in `CONTEXT.md` names
      a variable that really moves a store, and neither still says nothing names
      `{stores}`
- [ ] `development.md`'s `config.yaml` and settings-API examples match a server
      with these four languages in it
- [ ] Every descriptor the docs quote matches the embedded YAML as it now stands
