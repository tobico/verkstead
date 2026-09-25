//! An app that starts a sidecar and then ends without stopping it.
//!
//! Run as a process of its own by `tests/sidecar.test.ts`, because that is the
//! only way to ask the question it is asking: whether the child goes when the
//! process that started it ends for a reason nobody handled. Node reads the
//! TypeScript beside it by stripping the types, so there is no build step
//! between this and `src/`. Stripping is all it does, though: it resolves the
//! `.js` a compiled module imports to nothing, so what this reaches has to be a
//! module that imports no other — which is why `start` takes what to do with
//! the sidecar's lines rather than importing it.
//!
//! It says the child's pid on stdout and exits. What the test then wants to
//! know is that the pid is not there any more.

import { start } from "../../src/sidecar.ts";

// Nothing is done with what the sidecar says: the log file is the app's and
// this is not the app. What is under test is the child's lifetime — and the
// address is handed over for the same reason, the stand-in being a script that
// records its arguments and binds nothing.
const sidecar = start(process.argv[2], process.argv[3], () => {});
process.stdout.write(`${sidecar.pid}\n`);

// Not `stop()`: what is under test is the path where nobody stopped anything.
// A tick, so the write above has left.
setTimeout(() => process.exit(0), 50);
