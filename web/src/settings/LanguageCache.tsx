//! What a **Language plugin's pane** draws under its fields (Set 1784, Q2):
//! for each language the plugin provides, what its store holds against the
//! size in force, the store directories and the exceptions, when it was last
//! trimmed, and a **Clear**. The first part of a plugin's pane that is its
//! Category's rather than its manifest's — see `PluginSettings.tsx`.
//!
//! **Drawn whether the plugin is ticked or not**: an unticked language still
//! holds what it fetched, and the trimming never reads a language's switch, so
//! an off language's store is still held to its size, still measured and still
//! clearable.
//!
//! **The part says each thing once**: what the store holds against the size and
//! how much of that is packages, which directories it is, the exceptions —
//! sccache holding itself to the size, a directory that is never trimmed — and
//! when it was last trimmed. The ordinary case, every directory trimmed together
//! to the one size by whole packages, oldest first, is what all of that leaves
//! unsaid.
//!
//! **One Compile Server, one store.** C/C++ has no store of its own: its
//! compiles are kept in Rust's, which this part says rather than drawing C/C++ a
//! figure of nothing.
//!
//! **And a Clear**, under each language with a store, which empties every store
//! of that language — sccache's too, for Rust — and draws the figure the answer
//! measured again. **Off while a session or a terminal runs**, saying how many
//! of each: both are sandboxed with the stores, and the server refuses a Clear
//! that arrives anyway, which is drawn the same way.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createSignal, onCleanup, type JSX } from "solid-js";

import { clearLanguage } from "../api/client";
import type {
  DiskUse,
  LanguageCleared,
  PluginLanguageCache,
  PluginSettingsView,
  Running,
} from "../api/types";
import { ErrorLine } from "../notices";
import { settledAge, utcStamp } from "../set/when";
import styles from "./LanguageCache.module.css";

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

  return `Clear waits until no session or terminal is running. ${what.join(" and ")} ${verb}.`;
}

/// What a language's store holds against its size, as the part's first line —
/// and, where some of it is in whole packages, how much. The rest is an index,
/// metadata or what no package names, which nothing trims.
function held(disk: DiskUse, size: string): string | undefined {
  if (disk === "NoStore") {
    return undefined;
  }

  if (disk === "NotMeasured") {
    return "Cache usage: not measured yet. The server measures its store in the background.";
  }

  const { bytes, in_units } = disk.Measured;
  const usage = `Cache usage: ${usageSaid(bytes, size)}`;

  if (in_units === null) {
    return `${usage}.`;
  }

  return `${usage}, ${bytesSaid(in_units)} of it in packages.`;
}

/// Whether anything of a language is trimmed to its size by whole packages —
/// which is what says when it last was is worth saying.
function trimmed(language: PluginLanguageCache): boolean {
  return language.stores.some((store) => store.eviction === "ByUnit");
}

/// Whether a language's store is held to its size twice over: once by a tool
/// that evicts for itself, and once more by the trimming for the rest — Rust's
/// sccache beside its cargo half.
function twice(language: PluginLanguageCache): boolean {
  const evictions = language.stores.map((store) => store.eviction);

  return evictions.includes("ByItsTool") && evictions.includes("ByUnit");
}

/// When a language's store was last trimmed to its size, as a sentence and the
/// stamp behind it — or why it has not been, which a store on a machine that is
/// never idle would otherwise look forgotten over.
function sweptSaid(
  language: PluginLanguageCache,
  now: number,
): { said: string; stamp?: string } | undefined {
  if (!trimmed(language)) {
    return undefined;
  }

  if (language.swept === null) {
    return {
      said: "Not trimmed since the server started. Trimming waits until no session or terminal is running.",
    };
  }

  return {
    said: `Last trimmed ${settledAge(language.swept, now)}.`,
    stamp: utcStamp(language.swept),
  };
}

/// Whether a language says anything here: a store to measure, or a Compile
/// Server to compile through.
function drawn(language: PluginLanguageCache): boolean {
  return language.disk_use !== "NoStore" || language.compiling !== null;
}

/// The part, under a Language plugin's fields: nothing for a plugin providing
/// no language with a store or a Compile Server.
export function LanguageCaches(props: {
  /// The plugin's pane as the server last drew it.
  view: PluginSettingsView;
}): JSX.Element {
  const queries = useQueryClient();
  const key = () => ["plugins", props.view.uuid, "settings"];

  // The clock a last trim is said against, ticking so "2m ago" does not stand
  // still while the pane is open — the Set sheet's clock, for the same words.
  const [now, setNow] = createSignal(Date.now());
  const ticking = setInterval(() => setNow(Date.now()), 60_000);
  onCleanup(() => clearInterval(ticking));

  /// A Clear pressed, which empties every store of that language. Cleared, the
  /// pane is read again for the figure measured after; refused, what is running
  /// is written over the pane's own count so the button goes off and says what
  /// it waits on.
  const clearing = useMutation(() => ({
    mutationFn: (language: PluginLanguageCache) => clearLanguage(language.name),
    onSuccess: async (cleared: LanguageCleared) => {
      if (cleared === "NoSuchLanguage") {
        return;
      }

      if ("Running" in cleared) {
        const running = cleared.Running;
        queries.setQueryData<PluginSettingsView>(key(), (view) =>
          view === undefined ? view : { ...view, running },
        );
        return;
      }

      queries.setQueryData(["settings"], cleared.Cleared.settings);
      await queries.invalidateQueries({ queryKey: key() });
    },
  }));

  const languages = () => props.view.languages.filter(drawn);

  return (
    <Show when={languages().length > 0}>
      <section class={styles.caches}>
        <h3 class={styles.heading}>Build cache</h3>

        <For each={languages()}>
          {(language) => (
            <div class={styles.language} data-language={language.name}>
              <Show when={props.view.languages.length > 1}>
                <h4 class={styles.named}>{language.label}</h4>
              </Show>

              {/* A language compiling through another's Compile Server, which
                  is all there is to say about its cache. */}
              <Show when={language.through}>
                {(through) => (
                  <p class={styles.note}>
                    Compiles through the compile server, so its cache is{" "}
                    {through()}'s.
                  </p>
                )}
              </Show>

              <Show when={held(language.disk_use, language.size)}>
                {(said) => <p class={styles.note}>{said()}</p>}
              </Show>

              <Show when={language.stores.length > 0}>
                <p class={styles.note}>
                  Kept in{" "}
                  <For each={language.stores}>
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

              <Show when={twice(language)}>
                <p class={styles.note}>
                  sccache holds its compile cache to this size by itself, so the
                  store may hold up to twice it.
                </p>
              </Show>

              <For
                each={language.stores.filter(
                  (store) => store.eviction === "NotSwept",
                )}
              >
                {(store) => (
                  <p class={styles.note}>
                    <code>{store.name}</code> is never trimmed, so it grows as
                    its tool fills it.
                  </p>
                )}
              </For>

              <Show when={sweptSaid(language, now())}>
                {(swept) => (
                  <p class={styles.note} title={swept().stamp}>
                    {swept().said}
                  </p>
                )}
              </Show>

              <Show when={language.disk_use !== "NoStore"}>
                <div class={styles.clearing}>
                  <button
                    type="button"
                    class="secondary"
                    aria-label={`Clear ${language.label}'s cache`}
                    disabled={
                      clearing.isPending ||
                      clearWaits(props.view.running) !== undefined
                    }
                    onClick={() => clearing.mutate(language)}
                  >
                    Clear
                  </button>
                  <span class={styles.note}>
                    {clearWaits(props.view.running) ??
                      "Empties its store. The next install fetches again."}
                  </span>
                </div>
              </Show>
            </div>
          )}
        </For>

        <Show when={clearing.isError}>
          <ErrorLine>
            The store couldn't be cleared. {clearing.error?.message}
          </ErrorLine>
        </Show>
      </section>
    </Show>
  );
}
