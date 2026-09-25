//! The **Workbench Key**, read out of the **Data Directory** the sidecar was
//! started against.
//!
//! **The file is the documented thing** (ADR-0020). `workbench.key` sits beside
//! the settings files at mode `0600`, made by the server at its first start and
//! read back at every one after it — so the app reads that file rather than
//! parsing a line out of the sidecar's stdout, and rather than inventing a key
//! and handing it in. Both were considered and rejected: a line is a protocol
//! between two programs that already share a directory, and a key handed in
//! would make the app the thing that owns the secret.
//!
//! **The link is the address with the key on it**, and opening one is the whole
//! of logging in: the server sets the cookie and redirects to the same path
//! without the parameter, so the secret is out of the URL bar, the history entry
//! and any referrer before the page is drawn. See `crates/server/src/key.rs`,
//! which is the other end of both halves of this file.
//!
//! **Read afresh every time, never held in a variable.** **Reset key** at the
//! foot of Remote Access re-issues the secret from whatever device pressed it,
//! and a link built from what was read at startup is a 401 with extra steps.

import { readFileSync } from "node:fs";
import { join } from "node:path";

/// What the key's file is called inside the Data Directory — the server's own
/// `key::KEY_FILE`, which names it because the directory is Verkstead's and what
/// is in it is Verkstead's to name.
export const KEY_FILE = "workbench.key";

/// And what the parameter on the link is called — `key::QUERY`.
const QUERY = "key";

/// The key kept in `dataDir` as it is on disk **now**, or `undefined` where there
/// is none to read.
///
/// Three misfortunes and one answer. A file that is not there yet is a server
/// that has not written one — the app waits for health first, and the server
/// writes the key before it serves, so this is the unlucky ordering rather than
/// the usual one. A directory that is not there is the same. And a file that is
/// there and empty is treated as a file that is not there, exactly as the server
/// treats one: nothing writes an empty one, so it is a machine that lost power
/// or a hand that emptied it.
///
/// The caller's answer to all three is the same as well — load the address and
/// let the refusal that comes back ask again — which is why they are one
/// `undefined` rather than three failures.
export function keyIn(dataDir: string): string | undefined {
  let written: string;

  try {
    written = readFileSync(join(dataDir, KEY_FILE), "utf8");
  } catch {
    return undefined;
  }

  // The server writes the secret with a newline after it, and a key still
  // carrying that newline is a link the gate refuses.
  const secret = written.trim();

  return secret === "" ? undefined : secret;
}

/// The login link against `origin`: the address with the key on it, which is the
/// whole of how this window is let in.
///
/// Pointed at the root rather than at a path, as the server's own `link` is: the
/// workbench opens where it always opens and the handshake redirects there with
/// the parameter taken off.
///
/// **The secret is written as the file spells it**, unescaped. The gate reads the
/// parameter by splitting the query rather than by decoding it — see
/// `key::offered_in` — so an escaped one would be a secret that never matched.
/// What makes that safe at both ends is the alphabet a key is spelled in: base64
/// with the URL alphabet and no padding, whose every character is one no URL has
/// to escape.
export function link(origin: string, secret: string): string {
  return `${origin.replace(/\/+$/, "")}/?${QUERY}=${secret}`;
}
