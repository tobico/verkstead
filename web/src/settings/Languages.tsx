//! Which languages a session gets build support for — the **Build caches** part
//! of the settings page's Sandbox section: one row per **descriptor** the server loaded — a checkbox, and beside it how much
//! its cache holds against the size it may grow to, opening a panel with the
//! rest.
//!
//! **The page is drawn from the descriptors rather than from a list it holds.**
//! A language is data: the server loads the descriptors embedded in its binary,
//! merges what `config.yaml` says over them, and hands the lot over with a
//! label each. So a language an installer wrote a descriptor for has a row here
//! without a line of this file knowing its name, and there is nothing here that
//! would have to be edited to add one.
//!
//! **The file only.** There is no descriptor editor: the two keys of an entry
//! this page writes are the switch and the size, and an installer who wants a
//! variable, a manifest or a label edits `config.yaml`.
//!
//! Which is why a language whose entry in that file would not load is drawn
//! with its box off and a sentence under it saying why: the two keys they write
//! go into that entry, and the server writes it back exactly as it was typed.
//! What the sentence says is the reason — the variable, where a variable is what
//! was refused — and whether the language is running on the descriptor Verkstead
//! ships or off altogether. A language is never dropped from the list over it:
//! it is still there, and which of the two happened is the thing to know.
//!
//! One of the two parts of the Sandbox section — the Sandbox binds are the
//! other — and the only one of the two that is on with nothing configured.
//! Every path that part adds
//! is a hole somebody typed on purpose; what a language opens is Verkstead's own
//! directory, holding nothing but build output, so it can be opened for a human
//! who never asked and the only control over it here is the one that *closes*
//! it.
//!
//! It is on with nothing configured, which is the whole shape of the feature: a
//! human should not have a slower machine for never having found this section.
//! So a checkbox says where its language stands rather than whether anybody has
//! touched it, and the size field shows the default as a placeholder rather than
//! as a value somebody chose.
//!
//! A part rather than a section of its own: the card is the Sandbox's, at
//! `/settings/sandbox`, and this file hands it [`cachesOn`] and
//! [`cacheWarnings`] for its line — see `Sandbox.tsx`. What the human cannot fix
//! from the browser — no sccache where the server can see one, an entry that
//! would not load — is on that card as well as in the panels of the languages it
//! is about, the way the credentials' warnings are: whoever needs to read it is
//! precisely whoever is not editing.
//!
//! The card and the part read the one query, which is the one the credentials above them
//! read: one payload holds both files, and a read apiece would be two opinions
//! about what is saved.
//!
//! **One line a language, and everything else behind it.** Six of the shipped
//! languages used to draw the same block under their box — a field, a Save, a
//! disk-use sentence, a bullet per store directory, when it was last trimmed and
//! a Clear — which was a page of four sentences said six times over. So the row
//! is the checkbox and one link, and the link says the one figure somebody
//! scanning the page is after: what the cache holds against what it may hold,
//! `Cache usage: 3.2/30GB` — all of what is on disk against the size in force,
//! in the human's own `GB`. Everything else is in the panel the link opens, the
//! viewer's own popover — see [`Menu`] — said once for the language it is about.
//!
//! The link stands *beside* the checkbox's `<label>`, never inside it: the whole
//! row of a [`Check`] is one press forwarded to the box, and a link in there
//! would tick the language and open its panel at once.
//!
//! **An unticked language still holds what it fetched**, and the trimming never
//! reads a language's switch: an off language's store is still held to its size,
//! still measured and still clearable. So a row that is off and holds anything —
//! or might, not having been measured yet — carries a quieter link, `Holds
//! 1.8GB`, to the same panel. Only a store measured at nothing goes without, and
//! even that one keeps its link while its size is live — Rust's, sizing the
//! Compile Server C/C++ is still compiling through.
//!
//! **Two ways to save, because there are two kinds of control.** A checkbox
//! saves itself the moment it is ticked — a box that needed a second press to
//! mean anything is not a box. A size is typed, so it saves when the typing is
//! done: on Enter, or on leaving the field — closing the panel included, by
//! Escape as much as by a press away. Not on a blur with nothing typed, nor on
//! one where what is typed is what the field was already showing, nor on one
//! repeating a size the server has just turned down — every close of the panel
//! leaves the field, and a close is not a request to write `config.yaml`.
//!
//! Which is why **a tick sends the sizes the server last gave it** rather than
//! what the fields hold. One request writes the whole file, so a tick has to say
//! something about every size, and saying what is in a box would commit a number
//! nobody had finished — the `5` of a `50` somebody was halfway through. What was
//! typed stays typed: the tick did not save it, so the field goes on holding it
//! until it is saved or abandoned. The Cleanup pane's two durations are the same
//! rule for the same reason.
//!
//! What has been typed, and why the last size was turned down, are kept by the
//! pane rather than the panel: a panel's contents are built when it opens and
//! thrown away when it closes, and a refusal is still there to read when it is
//! opened again. And the rows are kept by *position* rather than by object, so a
//! save's answer — which is a fresh read of every language — redraws the panel
//! that is open rather than closing it.
//!
//! The size is live while its language is on, and greyed rather than taken away
//! while it is off — a field that vanished would say the setting had, and it has
//! not. On the language sizing the Compile Server it is live while either
//! language naming that server is on and there is an sccache to read it, and
//! while the language itself is on whether or not there is: its cargo half is a
//! store held to the same size, so a server with no sccache still reads it.
//!
//! **Its placeholder is that language's own default** — `30G` for Rust, whose
//! sccache has always been started at it, and `10G` for everything else. And a
//! size is a word the server reads, in sccache's grammar: one it cannot read is
//! refused at the save, with the reason drawn under the field, what was typed
//! left in it and the panel left open; one a hand-edit put into `config.yaml` is
//! drawn as typed with a sentence saying the store is held to the default
//! meanwhile.
//!
//! **One Compile Server, one size, one field.** Two built-ins name the sccache
//! capability — Rust and C/C++ — and the server runs one Compile Server between
//! them, sized by the first language that names it whether or not that one is
//! switched on. C/C++ has no store of its own besides, so its link says which
//! language's cache it compiles through, and its panel holds that and the
//! sccache warning — see [`sizer`]. Switch Rust off and the field stays in
//! Rust's panel, live while C/C++ is on: the store is still Rust's size, and a
//! field that moved to C/C++ would be the store shrinking to C/C++'s default.
//! Rust's link shows that size rather than twice it; that sccache holds itself
//! to the size apart from the rest is said inside.
//!
//! **And a language with no field for its size still sends one.** `size` is a
//! key of every entry whoever wrote it, so a save built out of the fields drawn
//! would write `config.yaml` with C/C++'s size gone, a key nobody was ever
//! shown being emptied by a press about something else. What every save sends
//! is what the server last gave it, per language, for both keys — see
//! [`heldLanguages`]. A language whose descriptor names no store directory and
//! no Compile Server — an installer's own, as often as not — has nothing to the
//! right of its box at all: nothing to measure, and nothing its size would hold.
//!
//! **The panel says each thing once**: the size, what the store holds and how
//! much of that is packages, which directories it is, the exceptions — sccache
//! holding itself to the size, a directory that is never trimmed — and when it
//! was last trimmed. The ordinary case, every directory trimmed together to the
//! one size by whole packages, oldest first, is what all of that leaves unsaid:
//! a line a directory each claiming the size read as the size seven times over.
//!
//! **And a Clear**, at the foot of it, which empties every store of that
//! language — sccache's too, for Rust — and draws the disk use the answer
//! measured again. Live whether the language is on or off, because a language
//! switched off still holds what it downloaded. **Off while a session or a
//! terminal runs**, saying how many of each: both are sandboxed with the stores,
//! and the server refuses a Clear that arrives anyway, which is drawn the same
//! way.
//!
//! The sizes go through the one settings endpoint, which writes both files:
//! the author rides along as it stands and the token is left alone, so saving a
//! store size cannot lose either.

import { faChevronDown } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Index,
  Match,
  Show,
  Switch as Choose,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import { Check } from "../Check";
import { Icon } from "../Icon";
import { Menu } from "../Menu";
import { clearLanguage, loadSettings, saveSettings } from "../api/client";
import type {
  DiskUse,
  LanguageEdit,
  LanguageCleared,
  LanguageView,
  Running,
  SettingsSaved,
  SettingsView,
  UnreadEntry,
} from "../api/types";
import { useReading } from "../freshness";
import { settledAge, utcStamp } from "../set/when";
import { Empty, ErrorLine } from "../notices";
import {
  heldAtOnce,
  heldCleanup,
  heldInstructions,
  heldLanguages,
  heldPaths,
} from "./held";
import styles from "./Languages.module.css";

/// What the section is called, wherever it names itself: the card's heading and
/// the pane's head.
const TITLE = "Build caches";

/// The settings as they stand, read once for the two panes that draw them.
///
/// The same read the credentials are drawn from, by the same key: one payload
/// holds both files, and two queries over it would be two opinions about what is
/// saved.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// What is said where a session's compiling is not cached, which the server has
/// already decided.
///
/// Drawn on the card and in the panel of each language compiling through
/// sccache, because it is the same sentence in both: said where somebody would
/// otherwise wonder why nothing got faster. Not an error — the builds still
/// work, and a language with downloads still shares them.
///
/// Worded for every language that compiles through sccache rather than for
/// Rust's: C/C++ has no downloads to fall back on, so the sentence says what is
/// lost and leaves what is kept to the language.
///
/// **And it is something to go and do**, which it was not always. `cached`
/// carried a third answer for a while: a Windows server was a machine where no
/// session could reach a compile server at all, because the AppContainer a
/// session ran in was refused the loopback the client talks to its server over,
/// and telling somebody to install sccache there would have changed nothing. A
/// session on that platform runs as a local account of Verkstead's own now and
/// reaches the loopback like anything else, so the answer went with the reason
/// for it and what is left everywhere is an instruction.
function uncompiled(): JSX.Element {
  return (
    <p class={styles.warning}>
      No sccache is installed where the server can see it, so <em>compiles</em>{" "}
      are not cached, and every session compiles again what the last one did.
      Install sccache on the server to cache them.
    </p>
  );
}

/// Whether the warning above is worth drawing for one language.
///
/// Only for a language that is switched on, because the rest of it is only true
/// then: switched off, nothing of that language is cached at all, and a line
/// singling out its compiles would be wrong exactly where somebody has just
/// turned it off.
///
/// The setup card's own warning is gated on more than this: it is drawn only
/// for a Repo a switched-on language compiling through sccache detects, because
/// that is a note above a press rather than a page about the machine — see
/// `ConversationView::compiles_uncached`, which the server works out with the
/// switch already in hand.
function warned(language: LanguageView): boolean {
  return (
    language.enabled &&
    language.compiling !== null &&
    language.compiling !== "Cached"
  );
}

/// And whether it is worth drawing on the card, which is about the machine:
/// any language it is true of.
function warnedAny(told: SettingsView | undefined): boolean {
  return (told?.languages ?? []).some(warned);
}

/// Whether a language's size field means anything.
///
/// For most languages, that the language is on and its entry is one the server
/// could read: an entry that would not load is one nothing can be written into.
///
/// For the one sizing the Compile Server, a language naming that server being
/// on — this one's box or another's, because the one size is this one's while
/// either compiles through it — and an sccache being there to read it, as well
/// as the entry being readable: the size is sccache's own word, so a server
/// without one has nowhere to put it. **Or that language being on with a store
/// trimmed to the size** — Rust's cargo half — which reads it whether or not
/// there is an sccache, so a server without one still has a size to set.
///
/// Every one of those is the field's *off*, and the field is drawn greyed
/// rather than taken away — a field that vanished would say the setting had,
/// and it has not.
function sizeable(
  told: SettingsView | undefined,
  language: LanguageView,
): boolean {
  if (language.unread !== null) {
    return false;
  }

  if (language.compiling === null) {
    return language.enabled;
  }

  if (
    language.enabled &&
    language.stores.some((store) => store.eviction === "ByUnit")
  ) {
    return true;
  }

  return (
    (told?.languages ?? []).some(
      (other) => other.compiling !== null && other.enabled,
    ) && language.compiling === "Cached"
  );
}

/// Whether a language draws a size field at all: one with a store of its own,
/// or the one the Compile Server is sized by.
function sized(
  told: SettingsView | undefined,
  language: LanguageView,
): boolean {
  return (
    language.store ||
    (language.compiling !== null && sizer(told)?.name === language.name)
  );
}

/// The units a figure is said in, each 1024 of the one before — the size
/// grammar's `K`, `M`, `G` and `T`, written as the human writes them.
const UNITS = ["KB", "MB", "GB", "TB"];

/// A number of a unit, with one decimal under ten, where the decimal is most of
/// what there is to read, and whole above: `0.4`, `9.6`, `30`.
function figure(value: number): string {
  return value < 10 ? value.toFixed(1) : Math.round(value).toString();
}

/// `bytes` in the units a human writes a size in — binary multiples, spelled
/// `KB`, `MB`, `GB`, `TB` — so a figure reads against the size beside it
/// without a conversion: `9.6GB` of `10GB`.
export function bytesSaid(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} bytes`;
  }

  let value = bytes / 1024;
  let unit = 0;

  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }

  return `${figure(value)}${UNITS[unit]}`;
}

/// The size a language's store is held to now: the one somebody configured,
/// unless it is not a size, and the default otherwise.
function inForce(language: LanguageView): string {
  return language.size_configured && language.size_unread === null
    ? language.size
    : language.default_size;
}

/// What a store holds against the size it is held to, as one fraction in the
/// size's own unit: `3.2/30GB`.
///
/// In the size's unit rather than each in its own, so the two are read against
/// each other without a conversion — except where what is held would round to
/// nothing in it, which is said in its own unit instead: `20MB/10GB` rather than
/// a `0.0` that reads as empty. A size the page cannot read as a number of a
/// unit is said as it was written.
export function usageSaid(bytes: number, size: string): string {
  const parsed = /^\s*(\d+(?:\.\d+)?)\s*([KMGT])B?\s*$/i.exec(size);

  if (parsed === null) {
    return `${bytesSaid(bytes)}/${size}`;
  }

  const unit = "KMGT".indexOf(parsed[2]!.toUpperCase());
  const value = bytes / 1024 ** (unit + 1);
  const limit = `${parsed[1]}${UNITS[unit]}`;

  if (bytes > 0 && value < 0.05) {
    return `${bytesSaid(bytes)}/${limit}`;
  }

  return `${figure(value)}/${limit}`;
}

/// What a language's store holds on disk, as the first line of its panel — and,
/// where some of it is in whole packages, how much. The rest is an index,
/// metadata or what no package names, which nothing trims and so does not count
/// against the size. Nothing for a language with no store directory to measure.
function held(disk: DiskUse): string | undefined {
  if (disk === "NoStore") {
    return undefined;
  }

  if (disk === "NotMeasured") {
    return "Not measured yet: the server measures its store in the background.";
  }

  const { bytes, in_units } = disk.Measured;

  if (in_units === null) {
    return `Holds ${bytesSaid(bytes)} on disk.`;
  }

  return `Holds ${bytesSaid(bytes)} on disk, ${bytesSaid(in_units)} of it in packages.`;
}

/// Whether anything of a language is trimmed to its size by whole packages —
/// which is what says when it last was is worth saying.
function trimmed(language: LanguageView): boolean {
  return language.stores.some((store) => store.eviction === "ByUnit");
}

/// When a language's store was last trimmed to its size, as a sentence and the
/// stamp behind it — or why it has not been, which a store on a machine that is
/// never idle would otherwise look forgotten over.
function sweptSaid(
  language: LanguageView,
  now: number,
): { said: string; stamp?: string } | undefined {
  if (!trimmed(language)) {
    return undefined;
  }

  if (language.swept === null) {
    return {
      said: "Not trimmed since the server started: trimming waits until no session or terminal is running.",
    };
  }

  return {
    said: `Last trimmed ${settledAge(language.swept, now)}.`,
    stamp: utcStamp(language.swept),
  };
}

/// How many of `count` there are, in words: `1 session`, `2 terminals`.
function counted(count: number, one: string): string {
  return `${count} ${one}${count === 1 ? "" : "s"}`;
}

/// Why a Clear is off, where something is running: what it waits on, counted —
/// or nothing, where nothing runs and the button is live.
export function clearWaits(running: Running): string | undefined {
  const what = [
    running.sessions > 0 ? counted(running.sessions, "session") : undefined,
    running.terminals > 0 ? counted(running.terminals, "terminal") : undefined,
  ].filter((said): said is string => said !== undefined);

  if (what.length === 0) {
    return undefined;
  }

  const verb =
    running.sessions + running.terminals === 1 ? "is running" : "are running";

  return `Clear waits until no session or terminal is running: ${what.join(" and ")} ${verb}.`;
}

/// Whether a language's store is held to its size twice over: once by a tool
/// that evicts for itself, and once more by the trimming for the rest — Rust's
/// sccache beside its cargo half.
function twice(language: LanguageView): boolean {
  const evictions = language.stores.map((store) => store.eviction);

  return evictions.includes("ByItsTool") && evictions.includes("ByUnit");
}

/// The language whose size the one Compile Server is started at: the first
/// language naming the sccache capability, switched on or not, which is the
/// answer the server's own `Languages::wanting` gives, over the list in the
/// order the server sent it.
function sizer(told: SettingsView | undefined): LanguageView | undefined {
  return (told?.languages ?? []).find(
    (language) => language.compiling !== null,
  );
}

/// What stands to the right of a language's box: the words of the link that
/// opens its panel, and whether it is the quieter one an unticked row draws —
/// or nothing, for a language with nothing to measure and no Compile Server to
/// compile through.
function linked(
  told: SettingsView | undefined,
  language: LanguageView,
): { said: string; quiet: boolean } | undefined {
  const disk = language.disk_use;
  const quiet = !language.enabled;

  if (disk !== "NoStore") {
    if (language.enabled) {
      return {
        said:
          disk === "NotMeasured"
            ? "Cache usage: not measured yet"
            : `Cache usage: ${usageSaid(disk.Measured.bytes, inForce(language))}`,
        quiet,
      };
    }

    // Off, and holding nothing: nothing to clear and nothing to read — unless
    // its size is still live for a language compiling through it.
    if (disk === "NotMeasured") {
      return { said: "Store not measured yet", quiet };
    }

    if (disk.Measured.bytes > 0 || sizeable(told, language)) {
      return { said: `Holds ${bytesSaid(disk.Measured.bytes)}`, quiet };
    }

    return undefined;
  }

  if (language.compiling !== null) {
    const through = sizer(told);

    return {
      said:
        through === undefined || through.name === language.name
          ? "Compile Server cache"
          : `Compiles through ${through.label}'s cache`,
      quiet,
    };
  }

  return undefined;
}

/// What is said under a language whose entry in `config.yaml` was not used.
///
/// The reason first, because it is what there is to fix, and it is the server's
/// own sentence: what goes wrong in a file somebody hand-wrote is open-ended,
/// and a viewer holding the vocabulary would be a release that could not add a
/// reason. Then what the language is running on meanwhile, which is the other
/// thing to know and the one this page can word for itself — the difference
/// between a cache working exactly as it did before that entry was written and
/// a language that is off until somebody goes and looks.
///
/// Under the row rather than in its panel, and greying the box beside it: the
/// two keys those controls write go into this entry, and a box that sprang back
/// the moment it was ticked would be a worse answer than a box that says why it
/// cannot be.
function unreadable(entry: UnreadEntry): JSX.Element {
  return (
    <p class={styles.warning}>
      Its entry in <code>config.yaml</code> {entry.why}.{" "}
      <Choose>
        <Match when={entry.running_on === "BuiltIn"}>
          Verkstead is using the descriptor it ships, which is what this
          language had before that entry was written.
        </Match>
        <Match when={entry.running_on === "Nothing"}>
          Verkstead ships no descriptor of that name, so this language is off
          until the entry is fixed.
        </Match>
      </Choose>
    </p>
  );
}

/// And which languages those are, in the words the card names them by.
///
/// On the card as well as in the pane, the way the sccache warning is and for
/// the same reason: whoever needs to read it is precisely whoever is not
/// editing. What it cannot say there is the reason — that is a sentence each,
/// and the card is a line.
function unread(told: SettingsView | undefined): string[] {
  return (told?.languages ?? [])
    .filter((language) => language.unread !== null)
    .map((language) => language.label);
}

/// What has build support switched on, in the words the card names them by.
///
/// The labels the server gave, which for a language an installer wrote is the
/// one in their file. A list rather than a sentence, because that is what the
/// card says: the languages a session can build, and nothing under the heading
/// where there are none.
function standing(told: SettingsView | undefined): string[] {
  return (told?.languages ?? [])
    .filter((language) => language.enabled)
    .map((language) => language.label);
}

/// What the Sandbox card says about the build caches where something is waiting
/// on the human — an entry `config.yaml` holds that could not be read, and
/// compiles nothing caches — or nothing at all where nothing is. See
/// `Sandbox.tsx`, which is the card.
export function cacheWarnings(told: SettingsView): string[] {
  return [
    unread(told).length > 0 ? "Unreadable config entry" : null,
    warnedAny(told) ? "No sccache" : null,
  ].filter((said): said is string => said !== null);
}

/// And how many languages have their build cache on, which is what it says
/// otherwise.
export function cachesOn(told: SettingsView): number {
  return standing(told).length;
}

/// And the controls that change them: the Build caches part of the Sandbox
/// pane, under its own heading — see `Sandbox.tsx`, which is the pane.
///
/// There is no Save over the whole of it and no Cancel: a checkbox is its own
/// press, a size saves when the typing is done, and a details pane is left by
/// opening something else or by the way back a narrow window draws.
export function BuildCaches(): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed in a size field, by the language it belongs to — a
  // language with nothing typed is one whose field follows the server, as the
  // author fields do. Here rather than in the panel, which is thrown away when
  // it closes.
  const [typed, setTyped] = createSignal<Record<string, string>>({});

  // And why the last size saved was turned down, by the language it belongs
  // to — the server's own clause, drawn under the field still holding what was
  // typed — with the size it turned down, so leaving the field does not send
  // the same one again.
  const [refused, setRefused] = createSignal<
    Record<string, { why: string; size: string }>
  >({});

  // The clock a last trim is said against, ticking so "2m ago" does not stand
  // still while the pane is open — the Set sheet's clock, for the same words.
  const [now, setNow] = createSignal(Date.now());
  const ticking = setInterval(() => setNow(Date.now()), 60_000);
  onCleanup(() => clearInterval(ticking));

  const told = (): SettingsView | undefined => settings.data;

  /// What a field follows while nothing is typed in it: the size somebody
  /// configured, else nothing at all — because an unconfigured size is drawn
  /// as the placeholder underneath rather than as text in the box.
  const followed = (language: LanguageView) =>
    language.size_configured ? language.size : "";

  /// What a field holds: what was typed, else what it follows.
  const size = (language: LanguageView) =>
    typed()[language.name] ?? followed(language);

  /// Let go of what was typed in one language's field, which then follows the
  /// server again.
  const forget = (name: string) =>
    setTyped((held) => {
      const { [name]: _gone, ...rest } = held;
      return rest;
    });

  /// What a save is asked to do: every language as it is to stand, and the one
  /// whose size was just saved, where one was.
  ///
  /// The second half is what says whether that field should let go of what was
  /// typed and follow the server again. A tick's save carries the sizes the
  /// server holds, so it commits nothing anybody typed and leaves the fields
  /// alone — the same shape, and the same reason, as the Cleanup pane's.
  type Asked = { edit: LanguageEdit[]; committed: string | null };

  const save = useMutation(() => ({
    mutationFn: ({ edit }: Asked) => {
      const author = told()?.git_author ?? { name: "", email: "" };

      return saveSettings({
        git_author: author,
        // Untouched. This form has no business with the credentials, and a
        // blank token field read as *clear this* is exactly what `Keep` is
        // here to stop.
        github_token: "Keep",
        languages: edit,
        // And what becomes of an archived Conversation, likewise — see
        // [`heldCleanup`].
        cleanup: heldCleanup(told()),
        // And how much Verkstead runs at once, likewise — see [`heldAtOnce`].
        at_once: heldAtOnce(told()),
        // And so is how a conflict is resolved, which is the section under it
        // on the page.
        conflict_resolution: told()?.conflict_resolution ?? "Merge",
        // And the switch on the GitHub section, likewise.
        share_on_done: told()?.share_on_done ?? false,
        // And the paths as they stand, which is the same reason again: a save
        // says what the file holds afterwards, so a list left out is a list
        // emptied — see [`heldPaths`].
        ...heldPaths(told()),
        // And the text every session is given, likewise — see
        // [`heldInstructions`].
        ...heldInstructions(told()),
        // And the ignore rules left exactly where they are. Alone among the
        // settings they travel as an action rather than a value: this form has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody — see
        // [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
        // And the declared MCP servers left where they are, for the reason
        // beside it: they are the other setting that travels as an action, and a
        // section that spoke for them could have its own save refused over a
        // name somebody hand-edited into the file weeks ago — see
        // [`McpServersEdit`].
        mcp_servers: "Keep",
      });
    },
    onSuccess: (saved: SettingsSaved, asked: Asked) => {
      // A size turned down is the whole save turned down: nothing was written,
      // so what was typed stays where it is and the reason goes under it.
      setRefused(
        Object.fromEntries(
          saved.refused_sizes.map((refusal) => [
            refusal.language,
            {
              why: refusal.why,
              size:
                asked.edit.find(
                  (language) => language.name === refusal.language,
                )?.size ?? "",
            },
          ]),
        ),
      );

      // What was typed goes, because the answer is now what the field follows —
      // and only for the language this save committed. A tick commits none, so
      // what somebody is halfway through writing is still theirs.
      if (asked.committed !== null && saved.refused_sizes.length === 0) {
        forget(asked.committed);
      }

      // The save's answer *is* a fresh read of both files, so a second read
      // would learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// A Clear pressed, which empties every store of that language and answers
  /// with the settings as they stand after — or, refused, with what is running,
  /// which is written over the read's own count so the button goes off and says
  /// what it waits on.
  const clearing = useMutation(() => ({
    mutationFn: (language: LanguageView) => clearLanguage(language.name),
    onSuccess: (cleared: LanguageCleared) => {
      if (cleared === "NoSuchLanguage") {
        return;
      }

      if ("Running" in cleared) {
        const running = cleared.Running;
        queries.setQueryData<SettingsView>(["settings"], (held) =>
          held === undefined ? held : { ...held, running },
        );
        return;
      }

      queries.setQueryData(["settings"], cleared.Cleared.settings);
    },
  }));

  /// Every language as a save puts it back, with one entry's key replaced —
  /// which is what either press sends. The sizes are the *server's*, so a press
  /// that is not about a size commits none of them: see [`heldLanguages`], and
  /// this module's header for why a tick does not send what the field holds.
  const writing = (name: string, key: Partial<LanguageEdit>): LanguageEdit[] =>
    heldLanguages(told()).languages.map((language) =>
      language.name === name ? { ...language, ...key } : language,
    );

  /// A box ticked, which saves itself.
  const flip = (language: LanguageView, enabled: boolean) =>
    save.mutate({
      edit: writing(language.name, { enabled }),
      committed: null,
    });

  /// A size saved, which sends that language's field.
  const commit = (language: LanguageView) =>
    save.mutate({
      edit: writing(language.name, { size: size(language) }),
      committed: language.name,
    });

  /// And the field left, which saves it only where there is something to save:
  /// something typed, that is not what the field was showing already, and not
  /// the size the server has just turned down. Every close of the panel is one
  /// of these, and a close is not a request to write `config.yaml`.
  const settle = (language: LanguageView) => {
    const written = typed()[language.name];

    if (written === undefined || save.isPending) {
      return;
    }

    if (written === followed(language)) {
      forget(language.name);
      return;
    }

    if (refused()[language.name]?.size === written) {
      return;
    }

    commit(language);
  };

  /// What a language's panel holds, top to bottom, each line only where it
  /// applies. Built when the panel opens and reading the language as it stands
  /// from then on, so an answer arriving redraws what is open.
  ///
  /// And settling the field as it is thrown away, because not every close is a
  /// blur: Escape takes the panel out of the page with the field still focused,
  /// and a focused element removed fires no blur. A close that was one has
  /// already sent its save, which [`settle`] does not send twice.
  const panel = (set: () => SettingsView, language: () => LanguageView) => {
    onCleanup(() => settle(language()));

    return (
      <>
        <Show when={sized(set(), language())}>
          <form
            class={styles.sizing}
            onSubmit={(ev) => {
              ev.preventDefault();
              commit(language());
            }}
          >
            <label for={`language-size-${language().name}`}>
              How large its store may grow
            </label>
            <input
              id={`language-size-${language().name}`}
              type="text"
              autocapitalize="off"
              autocorrect="off"
              spellcheck={false}
              disabled={!sizeable(set(), language())}
              // The default, so an empty box reads as the size nobody has chosen
              // rather than as no size at all.
              placeholder={language().default_size}
              value={size(language())}
              onInput={(ev) =>
                setTyped((held) => ({
                  ...held,
                  [language().name]: ev.currentTarget.value,
                }))
              }
              onBlur={() => settle(language())}
            />

            <Show when={refused()[language().name]}>
              {(turned) => (
                <ErrorLine class={styles.failure}>
                  <code>{turned().size}</code> {turned().why}.
                </ErrorLine>
              )}
            </Show>

            {/* And a size a hand-edit wrote that is not one: kept as written,
              and the store held to the default. */}
            <Show when={language().size_unread}>
              {(why) => (
                <p class={styles.warning}>
                  Its size in <code>config.yaml</code>,{" "}
                  <code>{language().size}</code>, {why()}, so its store is held
                  to {language().default_size}.
                </p>
              )}
            </Show>
          </form>
        </Show>

        {/* A language compiling through another's Compile Server, which is all
          there is to say about its cache. */}
        <Show
          when={
            language().compiling !== null &&
            sizer(set())?.name !== language().name
          }
        >
          <p class={styles.note}>
            Compiles through the Compile Server {sizer(set())?.label} sizes, so
            its cache is {sizer(set())?.label}'s and its size is set there.
          </p>
        </Show>

        <Show when={held(language().disk_use)}>
          {(said) => <p class={styles.note}>{said()}</p>}
        </Show>

        <Show when={language().stores.length > 0}>
          <p class={styles.note}>
            Kept in{" "}
            <For each={language().stores}>
              {(store, at) => (
                <>
                  {at() > 0 && ", "}
                  <code>{store.name}</code>
                </>
              )}
            </For>
            .
          </p>
        </Show>

        <Show when={twice(language())}>
          <p class={styles.note}>
            sccache holds its compile cache to this size by itself, so together
            with the rest the store may hold up to twice it.
          </p>
        </Show>

        <For
          each={language().stores.filter(
            (store) => store.eviction === "NotSwept",
          )}
        >
          {(store) => (
            <p class={styles.note}>
              <code>{store.name}</code> is never trimmed, so it grows as its
              tool fills it.
            </p>
          )}
        </For>

        <Show when={sweptSaid(language(), now())}>
          {(swept) => (
            <p class={styles.note} title={swept().stamp}>
              {swept().said}
            </p>
          )}
        </Show>

        <Show when={warned(language())}>{uncompiled()}</Show>

        {/* And a Clear, on or off: a language switched off still holds what it
          fetched. */}
        <Show when={language().disk_use !== "NoStore"}>
          <div class={styles.clearing}>
            <button
              type="button"
              disabled={
                clearing.isPending || clearWaits(set().running) !== undefined
              }
              onClick={() => clearing.mutate(language())}
            >
              Clear
            </button>
            <span class={styles.note}>
              {clearWaits(set().running) ??
                "Empties its store; the next install fetches again."}
            </span>
          </div>
        </Show>
      </>
    );
  };

  return (
    <section>
      <h3 class={styles.heading}>{TITLE}</h3>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Could not read the settings: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={told()}>
          {(set) => (
            <div class={styles.languages}>
              {/* By position rather than by object: every save answers with a
                  fresh read of every language, and a row keyed by the object
                  would be built again — closing the panel somebody was in the
                  middle of. */}
              <Index each={set().languages}>
                {(language) => (
                  <div>
                    <div class={styles.row}>
                      <Check
                        label={language().label}
                        on={language().enabled}
                        // And off for a language whose entry would not load:
                        // the switch is a key of that entry, and there is
                        // nothing readable there to write it into — see
                        // [`unreadable`], the sentence underneath saying so.
                        disabled={save.isPending || language().unread !== null}
                        flip={(enabled) => flip(language(), enabled)}
                      />

                      {/* Beside the label rather than inside it: a press
                          anywhere in the label ticks the box. */}
                      <Show when={linked(set(), language())}>
                        {(link) => (
                          <Menu
                            panel
                            class={
                              link().quiet
                                ? `${styles.cache} ${styles.quiet}`
                                : styles.cache!
                            }
                            name={`${language().label} cache`}
                            label={`${language().label}: ${link().said}`}
                            trigger={
                              <>
                                {link().said}
                                <Icon
                                  of={faChevronDown}
                                  class={styles.arrow!}
                                />
                              </>
                            }
                          >
                            {() => panel(set, language)}
                          </Menu>
                        )}
                      </Show>
                    </div>

                    {/* And why the box cannot be ticked, where the entry its
                        keys go into would not load. */}
                    <Show when={language().unread}>
                      {(entry) => unreadable(entry())}
                    </Show>
                  </div>
                )}
              </Index>

              <Show when={clearing.isError}>
                <ErrorLine class={styles.failure}>
                  The store could not be cleared: {clearing.error?.message}
                </ErrorLine>
              </Show>

              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </section>
  );
}
