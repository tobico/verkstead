# 03. A component hosted end to end

## What to build

Host an Activity's wasm component for real. Wasmtime links the `activity`
world's imports to the in-process host implementation from task 02, so a
component and an in-process driver reach the server through the same calls. The
component is started with the start facts, and its returned Outcome is taken.

- **Starting one.**
  - Until stage 11 there is no Process to say which Activity a state runs, so
    this task adds the narrowest seam that starts a named Activity on a
    Conversation. Stage 04 is its first real caller, and the suite is its first
    caller here.
  - While it runs, the Activity holds the Conversation's Driving registration,
    as every driver does, so the stall sweep never finds it undriven.
- **Ending one.**
  - The Outcome comes back to the caller, which maps it. Stage 11's Processes
    do that mapping as data later.
  - A component that **traps**, returns an Outcome it never declared, or cannot
    be instantiated is a **stop** with a Timeline notice saying so. It is never a
    server panic and never a Conversation left looking driven.
- **Its sessions are ordinary sessions.**
  - The prompt is the component's own text with the host's common sections
    appended.
  - The skill path comes from the start facts.
  - `verkstead done` is checked against the evidence the component registered.
- **The proof** is a fixture Activity component that:
  - launches one session on a test skill, under the Implementation role, with
    `committed` evidence;
  - waits;
  - reads the git facts;
  - ends in an Outcome.

  Write it however keeps the suite free of a wasm toolchain at test time: WAT
  through `wit-component` as the loader's tests do, or a built-in-only fixture
  under `plugins/` built by `build.rs`. The existing fixture plugin shows that
  second path.

## Acceptance criteria

- [ ] In the sessions suite, a fixture component started on a Conversation launches a session on a test skill, the session's Done signal is refused until a commit lands and taken after, and the component's Outcome comes back.
- [ ] A component that traps, or returns an undeclared Outcome, leaves the Conversation stopped with a notice saying so, and the server carries on.
- [ ] While a component runs, the Conversation counts as driven, and a Force stop ends both its session and the component.
