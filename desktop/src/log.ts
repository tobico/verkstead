//! The app's own lines.
//!
//! Standard output for now, which is where a developer running `pnpm start`
//! reads them. The log *file* is a later task's — `verkstead.log` under the Log
//! Directory, the sidecar's stdout beside these lines, the byte-order mark and
//! the roll — and this function is the seam it will be written behind, so that
//! what says something and what decides where it goes are never the same line.

/// Say one line as the app.
///
/// Stamped and named, because these lines share a stream with the sidecar's own
/// `tracing` output and a reader has to be able to tell which half is speaking.
export function say(line: string): void {
  process.stdout.write(`${new Date().toISOString()}  INFO verkstead_desktop: ${line}\n`);
}
