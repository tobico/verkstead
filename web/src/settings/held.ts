//! The paths and the Cleanup a save carries when the form in front of the human
//! is not about them.
//!
//! One request writes the whole of `config.yaml`, so every section's save sends
//! every value in it — the author, the build cache, the share-on-Done switch and
//! the sandbox binds. A section that left the list out would be a section that
//! emptied it: what is sent is what the file holds afterwards.
//!
//! Only the settings' own go back. The installation's entries come back on every
//! read labelled as the unit's word, they were never in this file, and sending
//! one would be asking the server to write down a flag — so they are filtered
//! out here, once, rather than in each of the sections that has to ride them
//! along.
//!
//! A bind goes back as the path it names, which is the whole of what one is. An
//! entry the view dropped — a bind written for one Repo, in the grammar that is
//! gone — goes back nowhere: it is not on the list that was read, so a save
//! takes it out of the file.

import type {
  CleanupEdit,
  CleanupStepEdit,
  CleanupStepView,
  SettingsView,
} from "../api/types";

/// Everything in `config.yaml` a form about the credentials is not about, as it
/// stands — ready to be spread into that form's save.
///
/// The whole file goes back in one request, so a save leaving one of these out
/// would be a save emptying it. Two forms send them: the settings page's own
/// credentials pane, and the onboarding wizard's git step, which asks for the
/// same three fields on a machine nobody has set up yet.
///
/// The defaults are what the server would write for a Verkstead nobody has told
/// anything, which is what the moment before the read has landed is.
export function heldConfig(told: SettingsView | undefined) {
  return {
    rust_build_cache: heldCache(told),
    // And what becomes of an archived Conversation, likewise.
    cleanup: heldCleanup(told),
    // And how a conflicted pull request is resolved, which is one of two words
    // and never absent: there is no third state for a form to send.
    conflict_resolution: told?.conflict_resolution ?? "Merge",
    // And the binds the settings hold, again for that reason — a list a form
    // left out would be a list it emptied. See [`heldPaths`].
    ...heldPaths(told),
  };
}

/// And the build cache as it stands, ready to be sent by a section that is not
/// about it — or by the checkbox on the section that is.
///
/// A size nobody typed goes back as the empty string rather than as the default
/// it is being shown as — see [`heldCleanup`], which says the same about a
/// duration.
///
/// **The size here is the server's rather than the field's**, which is what the
/// Rust checkbox wants of it. A box saves itself the moment it is ticked, so it
/// has to say something about the size beside it; saying what is in the box
/// would commit a number nobody pressed Save on — the `5` of a `50` somebody
/// was halfway through and thought better of. The size's own Save is what
/// commits the size, and this is what a tick sends instead.
export function heldCache(told: SettingsView | undefined): {
  enabled: boolean;
  size: string;
} {
  return {
    enabled: told?.rust_build_cache.enabled ?? true,
    size: told?.rust_build_cache.size_configured
      ? (told?.rust_build_cache.size ?? "")
      : "",
  };
}

/// The binds as they stand, ready to be spread into a save.
///
/// An empty list where the read has not landed, which is the same thing the
/// server would write for a Verkstead nobody has told anything.
export function heldPaths(told: SettingsView | undefined): {
  sandbox_binds: string[];
} {
  return {
    sandbox_binds: (told?.paths?.binds ?? [])
      .filter((entry) => entry.source === "Settings")
      .map((entry) => entry.path),
  };
}

/// And the Cleanup as it stands, ready to be sent by a section that is not
/// about it.
///
/// A duration nobody typed goes back as the empty string rather than as the
/// default it is being shown as: the days that come back are always a number,
/// and a section that echoed one would be writing a choice into the file on
/// behalf of somebody who never made it.
///
/// The switches where the read left them, and the trim on where it has not
/// landed — which is what the server writes for a Verkstead nobody has told
/// anything.
export function heldCleanup(told: SettingsView | undefined): CleanupEdit {
  return {
    trim: heldStep(told?.cleanup.trim, true),
    delete: heldStep(told?.cleanup.delete, false),
  };
}

/// One of its two rows, given what to say where the read has not landed.
function heldStep(
  step: CleanupStepView | undefined,
  standing: boolean,
): CleanupStepEdit {
  return {
    enabled: step?.enabled ?? standing,
    days: step?.days_configured ? String(step.days) : "",
  };
}
