//! The log file, which is the app's to write.
//!
//! The sidecar goes on logging to stdout exactly as `verkstead serve` always
//! has — nothing about its logging changes and `RUST_LOG` filters it as it
//! always did — and what is new is that somebody is reading it (ADR-0020). The
//! app takes those lines, puts its own beside them, and writes both into
//! `verkstead.log` under the **Log Directory**. Interleaved as they happened,
//! because the two halves of a startup only make sense read against each other:
//! the app saying it started a sidecar, the server saying it is listening, the
//! app saying the window is opening on it.
//!
//! **Because the stdout of an app launched from an icon goes nowhere.** That is
//! the whole reason there is a file at all, and it is why the file rather than
//! the stream is where this run's lines go. The one line that goes both ways is
//! the one naming the file, said as [`keep`] opens it: a developer running
//! `pnpm start` has a terminal, and a line inside the file cannot tell anybody
//! where the file is.
//!
//! **Bounded rather than endless.** The file rolls at [`ROLL_AT`] to the one
//! file behind it — at most two files, at most twice the bound, whatever the
//! machine has been up to — and the cut is between lines rather than through
//! one, which is what `byLine` in [`sidecar.ts`](./sidecar.js) hands it lines
//! for.
//!
//! **And a file this app starts opens with a byte-order mark.** The one thing
//! here that is about the reader rather than the writing: Verkstead's own
//! messages have em-dashes in them, and the viewers Windows opens a `.log` in
//! read a file with no mark in the machine's code page — so a log written
//! without one is mojibake to the very person being asked to report what it
//! says. The mark belongs to a file being *started*, so it goes in at the head
//! of a fresh file and at the head of each roll, and a run appending to a file
//! that already has content adds none.
//!
//! **Nowhere to put one is not a failure.** A machine that names no Log
//! Directory, or one whose Log Directory cannot be opened, gets no file: the app
//! says so, logs to its standard error instead and goes on running. That is the
//! opposite of what a missing Data Directory means, and deliberately — a
//! Verkstead with nowhere to keep its database has nothing to serve, while one
//! with nowhere to keep a log file has only lost the log.
//!
//! All of it is `crates/desktop/src/logs.rs` kept as it was: the same file
//! names, the same bound, the same mark, so that a machine which has run both
//! apps has one log rather than two conventions.

import { closeSync, fstatSync, mkdirSync, openSync, renameSync, writeSync } from "node:fs";
import { join } from "node:path";

/// What the log file is called inside the Log Directory.
export const FILE = "verkstead.log";

/// And what it rolls over to. One file behind the live one and no more: two
/// runs' worth of the recent past is what somebody reporting a problem is asked
/// for, and the whole point of rolling over is that this directory has a size
/// nobody has to think about.
export const PREVIOUS = "verkstead.log.1";

/// What a file this app starts opens with: the UTF-8 byte-order mark, which is
/// three bytes saying what the encoding is to a reader that would otherwise
/// guess.
export const MARK = "﻿";

/// How large the live file may get before it rolls over.
///
/// Big enough to hold a long run of an ordinary machine's logging, small enough
/// to open in a text editor and to attach to a report of what went wrong.
export const ROLL_AT = 4 * 1024 * 1024;

/// How many bytes the mark is, which is the size a file holding nothing else
/// has.
const MARKED = Buffer.byteLength(MARK);

/// The log file itself: appended to, and rolled over where a write would take
/// it past [`ROLL_AT`].
export interface Log {
  /// The file being written, which is what **View Logs** will open.
  readonly file: string;

  /// Write one line's worth, rolling first where this would take the file past
  /// the bound. Whole lines, so that the cut is never through one.
  write(text: string): void;
}

/// Open the log file in `dir`, **making the directory**, and pick up where a
/// previous run left off.
///
/// Appended to rather than truncated: a crash is exactly when the run before
/// this one is worth reading, and it is the rolling over rather than the
/// starting that keeps this bounded.
///
/// Throws where the directory cannot be made or the file cannot be opened,
/// which [`keep`] is what answers.
export function bounded(dir: string): Log {
  mkdirSync(dir, { recursive: true });

  const file = join(dir, FILE);
  const previous = join(dir, PREVIOUS);

  // Synchronous, all of it. A log is written from the one thread the main
  // process has and read by whoever opens the file afterwards, so the order
  // lines land in is the order they were said in — and an app that queued its
  // last words behind an event loop it is about to leave would lose exactly the
  // lines somebody is looking for.
  let handle = openSync(file, "a");

  // Kept as it is written rather than asked of the filesystem: a `stat` per
  // line would be a syscall for nothing.
  let written = marked(handle, fstatSync(handle).size);

  /// The live file becomes the previous one, and a new live file is opened
  /// behind it. A rename rather than a copy, so nothing is ever half-written:
  /// the roll costs one directory entry whatever the file grew to.
  const roll = (): void => {
    closeSync(handle);
    renameSync(file, previous);
    handle = openSync(file, "a");
    written = marked(handle, 0);
  };

  return {
    file,
    write(text) {
      const bytes = Buffer.byteLength(text);

      // A file holding nothing but its mark does not roll: a first line longer
      // than the whole bound would otherwise shuffle an empty file along and
      // start again, which throws the line away to keep a rule about size.
      if (written > MARKED && written + bytes > ROLL_AT) {
        roll();
      }

      written += writeSync(handle, text);
    },
  };
}

/// Put [`MARK`] at the head of a file that is being started, and say how much is
/// in it afterwards.
///
/// A file with something in it already is a run picking up where the one before
/// it left off, and the mark is where *that* run put it.
function marked(handle: number, written: number): number {
  return written > 0 ? written : writeSync(handle, MARK);
}

/// Where this run's logging went, which is what **View Logs** will open — or
/// say instead of opening.
export type Kept =
  /// In the log file at this path.
  | { readonly file: string }
  /// Nowhere: there is no file, for the reason this carries, and the logging is
  /// going to standard error. Worded for a human because a later stage shows it
  /// to one.
  | { readonly nowhere: string };

/// Where a line goes when there is no file for it.
///
/// Standard error rather than standard output, because the lines it carries are
/// the ones about there being no file — and because an app launched from an icon
/// has neither, which is what makes this the fallback rather than the plan.
const STREAM = (text: string): void => void process.stderr.write(text);

/// Where a line goes: the log file once [`keep`] has opened one, and [`STREAM`]
/// until then and instead.
let writing = STREAM;

/// Send this run's logging to the log file in `dir`, and say where that turned
/// out to be — on standard error as well as in the file, because a developer
/// running `pnpm start` has to be told which file to read.
///
/// Called once, at startup, before anything has anything to report. `undefined`
/// is a machine that names nowhere to put one, which is the same answer as a
/// directory that cannot be made: no file, the lines on standard error, and the
/// app going on running.
export function keep(dir: string | undefined): Kept {
  if (dir === undefined) {
    return nowhere(
      "Verkstead has nowhere to keep a log file on this machine, so this run's log is going " +
        "to the standard error of whatever started it.",
    );
  }

  let log: Log;
  try {
    log = bounded(dir);
  } catch (trouble) {
    return nowhere(
      `Verkstead could not open its log file in ${dir} (${String(trouble)}), so this run's ` +
        `log is going to the standard error of whatever started it.`,
    );
  }

  writing = (text) => log.write(text);

  // **The one line that goes both ways**, and it has to: a line inside the file
  // cannot tell anybody where the file is, and a developer who has just run
  // `pnpm start` is looking at a terminal rather than at a log. Everything else
  // this run says is the file's alone — which is the whole point of there being
  // one, an app launched from an icon having no terminal for a line to land in.
  const line = `this run's log is ${log.file}`;
  say(line);
  STREAM(`${line}\n`);

  return { file: log.file };
}

/// The other ending: no file, the logging on standard error, and `why` said
/// there as well as handed back for the menu item that would have opened one.
function nowhere(why: string): Kept {
  writing = STREAM;
  say(why);

  return { nowhere: why };
}

/// Say one line as the app.
///
/// Stamped and named, because these lines share a file with the sidecar's own
/// `tracing` output and a reader has to be able to tell which half is speaking.
export function say(line: string): void {
  writing(`${new Date().toISOString()}  INFO verkstead_desktop: ${line}\n`);
}

/// What a terminal reads as colour and a text editor reads as gibberish.
///
/// `verkstead serve` writes for the shell that started it, which means
/// `tracing`'s own colouring — and the sidecar's shell is a pipe into this
/// process. The Rust app never met this because it owned the writer and turned
/// the colouring off at the source; here the source is a binary whose logging
/// is deliberately unchanged (ADR-0020), so the escapes come off on the way
/// into the file instead. The whole CSI form rather than the colours alone:
/// what has no business in a file has none of it whatever it was for.
const ESCAPES = /\u001B\[[0-?]*[ -/]*[@-~]/g;

/// Write one line the sidecar said, as a file should hold it.
///
/// Unstamped and unnamed: the server has already stamped and named it, and a
/// second stamp would be the app's clock put in front of the server's own.
export function heard(line: string): void {
  writing(`${line.replace(ESCAPES, "")}\n`);
}
