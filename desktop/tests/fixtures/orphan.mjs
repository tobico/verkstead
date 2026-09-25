//! An app that starts a sidecar and then ends without stopping it.
//!
//! Run as a process of its own by `tests/sidecar.test.ts`, because that is the
//! only way to ask the question it is asking: whether the child goes when the
//! process that started it ends for a reason nobody handled. Node reads the
//! TypeScript beside it by stripping the types, so there is no build step
//! between this and `src/`.
//!
//! It says the child's pid on stdout and exits. What the test then wants to
//! know is that the pid is not there any more.

import { start } from "../../src/sidecar.ts";

const sidecar = start(process.argv[2]);
process.stdout.write(`${sidecar.pid}\n`);

// Not `stop()`: what is under test is the path where nobody stopped anything.
// A tick, so the write above has left.
setTimeout(() => process.exit(0), 50);
