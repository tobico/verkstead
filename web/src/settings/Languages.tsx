//! The **Build caches** part of the settings page's Sandbox section, and the
//! one thing in it: whether the server can see an sccache.
//!
//! **No language has a row here.** Every language is a Language plugin's —
//! the seven Verkstead ships are six built-in plugins, and an installer's own
//! `languages:` entry in `config.yaml` is drawn as a plugin of its own — so a
//! language's switch is its plugin's tick on the Plugins page, cluster-wide
//! (Set 1762, Q11a), its size its plugin's `size` setting, and its cache usage
//! and Clear its plugin's pane (Set 1784, Q2) — see `LanguageCache.tsx`. An
//! entry of that file that won't load is its plugin's trouble, on its row.
//!
//! What this part keeps is the one thing that is not a language's (Set 1784,
//! Q3): the line saying there is no sccache where the server can see one,
//! drawn on the Sandbox card as well — see `Sandbox.tsx` — because whoever
//! needs to read it is precisely whoever is not editing.
//!
//! The card and the part read the one query, which is the one the credentials
//! above them read: one payload holds both files, and a read apiece would be
//! two opinions about what is saved.

import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import { loadSettings } from "../api/client";
import type { LanguageView, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import styles from "./Languages.module.css";

/// What the section is called, wherever it names itself: the card's heading and
/// the pane's head.
const TITLE = "Build Caches";

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
      The server can't see an sccache, so <em>compiles</em> aren't cached and
      every session compiles again what the last one did. Install sccache on the
      server to cache them.
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

/// What the Sandbox card says about the build caches where something is waiting
/// on the human — compiles nothing caches — or nothing at all where nothing is.
/// See `Sandbox.tsx`, which is the card.
export function cacheWarnings(told: SettingsView): string[] {
  return warnedAny(told) ? ["No sccache"] : [];
}

/// And the part itself: the Build caches part of the Sandbox pane, under its
/// own heading — see `Sandbox.tsx`, which is the pane.
export function BuildCaches(): JSX.Element {
  const settings = useSettings();

  return (
    <section>
      <h3 class={styles.heading}>{TITLE}</h3>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Reading the settings…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            The settings couldn't be read. {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={settings.data}>
          {(set) => (
            <div class={styles.languages}>
              {/* Whether compiles are cached, which is no language's: the
                  server's own environment. */}
              <Show when={warnedAny(set())}>{uncompiled()}</Show>

              <Note>
                A language's build cache is in its plugin's settings, on the
                Plugins page.
              </Note>
            </div>
          )}
        </Match>
      </Choose>
    </section>
  );
}
