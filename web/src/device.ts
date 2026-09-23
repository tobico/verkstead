//! What the device in front of you remembers: `localStorage`, and the settings
//! kept in it.
//!
//! Deliberately per device and never sent to the server — a phone and a laptop
//! answer the same Set from different places, and neither has any business
//! deciding how the other draws a Diff. Push notifications are per device for
//! the same reason, but the browser is the one that remembers those, so nothing
//! about them is kept here.
//!
//! Storage is a convenience the whole way down: a browser that refuses it costs
//! the human their drafts and their settings and nothing else, so nothing on
//! this path is worth a thrown error. Every read comes back `null` and every
//! write is dropped.

/// Where the wrap setting lives. Namespaced like the drafts beside it, so
/// everything this app leaves in a browser is legible as its own.
const WRAP = "verkstead.diff-wrap";

/// Whether Diffs are drawn wrapped on this device.
///
/// Anything but the value [`setWrapping`] writes reads as off, which is also
/// what an untouched browser answers: unwrapped is the setting nobody has
/// expressed an opinion about.
export function wrapping(): boolean {
  return read(WRAP) === "on";
}

/// Remember how this device wants Diffs drawn.
export function setWrapping(on: boolean): void {
  if (on) {
    write(WRAP, "on");
  } else {
    // Removed rather than written as "off": the absence is already the default,
    // and this way turning it off leaves nothing behind.
    forget(WRAP);
  }
}

/// And where the last repo this device made was put.
///
/// Somebody making a second repository is almost certainly putting it beside the
/// first, and where they keep their code is a fact about the machine in front of
/// them rather than something to tell the server: a laptop's `~/src` and a
/// phone's nothing at all are two answers to the same question, and neither is
/// the other's to write down.
const PARENT = "verkstead.repo-parent";

/// The directory the Create repo modal starts its browse in — empty where this
/// device has not made one yet, which is what a first run always is.
///
/// Empty rather than `null` because empty is what the field is given: a path
/// field standing empty browses the server's own home, so nothing here has to
/// know what that home is.
export function repoParent(): string {
  return read(PARENT) ?? "";
}

/// Remember where a repo was just made, for the next one.
export function setRepoParent(path: string): void {
  write(PARENT, path);
}

/// What is being held under this key, if anything — and `null` when there is no
/// storage to be had, which is a browser that blocks it or one in a context
/// that has none.
export function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/// Write `body` out, replacing whatever was under the key — and say whether it
/// landed.
///
/// Nearly every caller drops the answer, storage being a convenience: a setting
/// that could not be written is one the next visit does without. What reads it
/// is a caller holding two things of unequal worth, which is Code — its layout
/// is written apart from its unsaved text so that a storage too full for the
/// text still comes back to the splits, and a text that would not fit is taken
/// away rather than left behind to be restored over a file it no longer
/// describes (see `workbench/remembering.ts`).
export function write(key: string, body: string): boolean {
  try {
    localStorage.setItem(key, body);
    return true;
  } catch {
    // Full, or refused: what was being written is gone, and the page carries on
    // regardless.
    return false;
  }
}

/// Drop whatever is under this key.
export function forget(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // As above: there was nothing to lose but the setting.
  }
}
