//! The paths, the Cleanup, the limits and the instructions a save carries when
//! the form in front of the human is not about them.
//!
//! One request writes the whole of `config.yaml`, so every section's save sends
//! every value in it — the author, the languages, the share-on-Done switch, the
//! sandbox binds and the text every session is given. A section that left one
//! out would be a section that emptied it: what is sent is what the file holds
//! afterwards.
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
  AtOnceEdit,
  CleanupEdit,
  CleanupStepEdit,
  CleanupStepView,
  LanguageEdit,
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
    ...heldLanguages(told),
    // And what becomes of an archived Conversation, likewise.
    cleanup: heldCleanup(told),
    // And how much Verkstead runs at once — see [`heldAtOnce`].
    at_once: heldAtOnce(told),
    // And how a conflicted pull request is resolved, which is one of two words
    // and never absent: there is no third state for a form to send.
    conflict_resolution: told?.conflict_resolution ?? "Merge",
    // And the binds the settings hold, again for that reason — a list a form
    // left out would be a list it emptied. See [`heldPaths`].
    ...heldPaths(told),
    // And the text every session is given, likewise: a form that left it out
    // would be a form that cleared it.
    ...heldInstructions(told),
  };
}

/// And the instructions every session is given, as they stand, ready to be
/// spread into a save.
///
/// Empty where the read has not landed, which is what the server writes for a
/// Verkstead nobody has told anything — and what a form sending it would ask
/// for is the state the file is already in.
///
/// Verbatim, because verbatim is what a harness is handed: a form riding this
/// along has nothing to say about the words in it, and is not entitled to trim
/// them on the way past.
export function heldInstructions(told: SettingsView | undefined): {
  instructions: string;
} {
  return { instructions: told?.instructions ?? "" };
}

/// And the languages as they stand, ready to be spread into a save by a section
/// that is not about them — or by a checkbox on the section that is.
///
/// One entry per language the last read listed, carrying the two keys the page
/// writes. An empty list where the read has not landed, which is the one thing
/// a form can honestly say about languages it has never been told: the server
/// keeps what the file already holds under whatever a save sends, so an entry
/// nobody sent is an entry nobody changed.
///
/// A size nobody typed goes back as the empty string rather than as the default
/// it is being shown as — see [`heldCleanup`], which says the same about a
/// duration.
///
/// **Every language's size, whether the pane draws a field for it or not.** It
/// draws one only under a language whose store an sccache bounds, and a save
/// built out of the fields alone would write `config.yaml` with every other
/// language's `size` gone — a key of an entry the page never showed anybody.
///
/// **The size here is the server's rather than the field's**, which is what the
/// language checkboxes want of it. A box saves itself the moment it is ticked,
/// so it has to say something about the size beside it; saying what is in the
/// box would commit a number nobody pressed Save on — the `5` of a `50`
/// somebody was halfway through and thought better of. The size's own Save is
/// what commits the size, and this is what a tick sends instead.
export function heldLanguages(told: SettingsView | undefined): {
  languages: LanguageEdit[];
} {
  return { languages: (told?.languages ?? []).map(asEdit) };
}

/// One of them, as a save puts it back.
function asEdit(language: SettingsView["languages"][number]): LanguageEdit {
  return {
    name: language.name,
    enabled: language.enabled,
    size: language.size_configured ? language.size : "",
  };
}

/// And how much Verkstead runs at once, as it stands, ready to be sent by a
/// section that is not about it — both numbers, because both are in the one
/// section and a save writes the section whole.
///
/// A number nobody typed goes back as the empty string rather than as the default
/// it is being shown as — see [`heldCleanup`], which says the same about a
/// duration: the number that comes back is always there, and a section that
/// echoed it would be writing a choice into the file on behalf of somebody who
/// never made it.
///
/// How many places are *taken* goes back nowhere. It is a reading rather than a
/// setting, and there is nothing in the edit for it to be sent as.
export function heldAtOnce(told: SettingsView | undefined): AtOnceEdit {
  return {
    roadmap_stages: told?.at_once.roadmap_stages_configured
      ? String(told.at_once.roadmap_stages)
      : "",
    conversations: told?.at_once.conversations_configured
      ? String(told.at_once.conversations)
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
