//! Which languages a session gets build support for, on the settings page:
//! today that is Rust, and what it means is the shared build cache every
//! sandbox is given and how big its compiled half may grow.
//!
//! One of two things on this page that are about a **Sandbox** rather than about
//! who Verkstead is — the Paths section is the other — and the only one of the
//! two that is on with nothing configured. Every bind the Paths section adds is
//! a hole somebody typed on purpose; this one is Verkstead's own directory,
//! holding nothing but build output, so it can be opened for a human who never
//! asked and the only control over it here is the one that *closes* it.
//!
//! It is on with nothing configured, which is the whole shape of the feature: a
//! human should not have a slower machine for never having found this section.
//! So the checkbox says where it stands rather than whether anybody has touched
//! it, and the size field shows the default as a placeholder rather than as a
//! value somebody chose.
//!
//! Two halves in two panes, which is what the settings page is: a card in the
//! middle pane naming the languages that have build support on, and the controls
//! that change them in the details pane it opens, at `/settings/languages`. What
//! the human cannot fix from the browser — no sccache where the server can see
//! one — is on the card as well as in the pane, the way the credentials'
//! warnings are: whoever needs to read it is precisely whoever is not editing.
//!
//! Both halves read the one query, which is the one the credentials above them
//! read: one payload holds both files, and a read apiece would be two opinions
//! about what is saved.
//!
//! Two ways to save, because there are two kinds of control. The checkbox saves
//! itself the moment it is ticked — a box that needed a second press to mean
//! anything is not a box. The size is typed, so it saves on a press of its own;
//! nothing is committed while somebody is still halfway through writing `30`.
//!
//! The size hangs off the checkbox, which is the page's one pattern for
//! configuration that only means something while something else is on: indented
//! under the box it belongs to, and disabled while that box is off — see
//! [`Nested`]. It is disabled for a second reason here as well, and the reason
//! is the warning above it: the size is sccache's own, so a server with no
//! sccache has nothing to read it.
//!
//! Both go through the one settings endpoint, which writes both files: the
//! author rides along as it stands and the token is left alone, so saving a
//! cache size cannot lose either.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { Check, Nested } from "../Check";
import { PaneSticky } from "../Panes";
import { loadSettings, saveSettings } from "../api/client";
import type { BuildCacheView, SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldCleanup, heldPaths } from "./held";
import styles from "./Languages.module.css";

/// What the section is called, wherever it names itself: the card's heading and
/// the pane's head.
const TITLE = "Language support";

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
/// Drawn on the card and in the pane alike, because it is the same sentence in
/// both: said where somebody would otherwise wonder why nothing got faster. Not
/// an error — the downloads are still shared, and the builds still work.
///
/// **And it is something to go and do**, which it was not always. `compiles`
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
      No sccache is installed where the server can see it, so dependency{" "}
      <em>compiles</em> are not cached — only the crate downloads. Install
      sccache on the server to cache the compiling too.
    </p>
  );
}

/// Whether the warning above is worth drawing at all.
///
/// Only while the cache is on, because the rest of it is only true then:
/// switched off, nothing is cached at all, and a line saying the downloads still
/// are would be wrong exactly where somebody has just turned it off.
///
/// The setup card's own warning is gated on more than this: it is drawn only
/// for a Repo that builds Rust, because that is a note above a press rather
/// than a page about the machine — see `ConversationView::compiles_uncached`,
/// which the server works out with the switch already in hand.
function warned(cache: BuildCacheView): boolean {
  return cache.enabled && cache.compiles !== "Cached";
}

/// Whether the size hanging off the checkbox means anything.
///
/// The checkbox being on and an sccache being there to read it: the size is
/// sccache's own word, so a server without one has nowhere to put it. Both are
/// the group's *off*, and the group is drawn greyed rather than taken away —
/// a field that vanished would say the setting had, and it has not.
function sizeable(cache: BuildCacheView): boolean {
  return cache.enabled && cache.compiles === "Cached";
}

/// What has build support switched on, in the words the card names them by.
///
/// One language today, and the cache's switch is the whole of whether it is on.
/// A list rather than a sentence, because that is what the card says: the
/// languages a session can build, and nothing under the heading where there are
/// none.
function languages(cache: BuildCacheView): string[] {
  return cache.enabled ? ["Rust"] : [];
}

/// What has build support, as the card that opens it.
///
/// What is on the card is what somebody scanning the page is after: which
/// languages a session builds with help, and the one thing about that which
/// wants doing somewhere else. The controls are in the pane.
export function LanguagesCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <Choose>
      <Match when={settings.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          Could not read the settings: {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data?.rust_build_cache}>
        {(cache) => (
          <CardButton
            as="article"
            class={styles.languagesCard}
            open={props.open}
            press={props.press}
          >
            <h2>{TITLE}</h2>

            <Show when={warned(cache())}>{uncompiled()}</Show>

            <Show when={languages(cache()).length > 0}>
              <p class={styles.standing}>{languages(cache()).join(", ")}</p>
            </Show>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the controls that change it, which is the details pane the card opens.
///
/// There is no Save over the whole of it and no Cancel: the checkbox is its own
/// press and the size has one of its own, and a details pane is left by opening
/// something else or by the way back a narrow window draws.
export function LanguagesPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed in the size field, or `null` while nothing has — the
  // field follows the server until somebody touches it, as the author fields do.
  const [typed, setTyped] = createSignal<string | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const cache = () => told()?.rust_build_cache;

  /// What the field holds: what was typed, else the size somebody configured,
  /// else nothing at all — because an unconfigured size is drawn as the
  /// placeholder underneath rather than as text in the box.
  const size = () =>
    typed() ?? (cache()?.size_configured ? (cache()?.size ?? "") : "");

  const save = useMutation(() => ({
    mutationFn: (edit: { enabled: boolean; size: string }) => {
      const author = told()?.git_author ?? { name: "", email: "" };

      return saveSettings({
        git_author: author,
        // Untouched. This form has no business with the credentials, and a
        // blank token field read as *clear this* is exactly what `Keep` is
        // here to stop.
        github_token: "Keep",
        rust_build_cache: edit,
        // And what becomes of an archived Conversation, likewise — see
        // [`heldCleanup`].
        cleanup: heldCleanup(told()),
        // And so is how a conflict is resolved, which is the section under it
        // on the page.
        conflict_resolution: told()?.conflict_resolution ?? "Merge",
        // And the switch on the GitHub section, likewise.
        share_on_done: told()?.share_on_done ?? false,
        // And the paths as they stand, which is the same reason again: a save
        // says what the file holds afterwards, so a list left out is a list
        // emptied — see [`heldPaths`].
        ...heldPaths(told()),
        // And the ignore rules left exactly where they are. Alone among the
        // settings they travel as an action rather than a value: this form has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody — see
        // [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
      });
    },
    onSuccess: (saved: SettingsSaved) => {
      // What was typed goes, because the answer is now what the field follows.
      setTyped(null);

      // The save's answer *is* a fresh read of both files, so a second read
      // would learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  const flip = (enabled: boolean) => save.mutate({ enabled, size: size() });

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();
    save.mutate({ enabled: cache()?.enabled ?? true, size: size() });
  };

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title={TITLE} />
      </PaneSticky>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Could not read the settings: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={cache()}>
          {(set) => (
            <div class={styles.languages}>
              <Check
                label="Rust"
                on={set().enabled}
                disabled={save.isPending}
                flip={flip}
              />

              <Show when={warned(set())}>{uncompiled()}</Show>

              {/* The size is sccache's, so the group is off where there is no
                  sccache to read it as well as where the box is unticked. */}
              <Nested on={sizeable(set())}>
                <form class={styles.sizing} onSubmit={commit}>
                  <label for="build-cache-size">
                    How large the compiled half may grow
                  </label>
                  <div class={styles.field}>
                    <input
                      id="build-cache-size"
                      type="text"
                      autocapitalize="off"
                      autocorrect="off"
                      spellcheck={false}
                      // The default, so an empty box reads as the size nobody
                      // has chosen rather than as no size at all.
                      placeholder={set().size}
                      value={size()}
                      onInput={(ev) => setTyped(ev.currentTarget.value)}
                    />
                    <button type="submit" disabled={save.isPending}>
                      Save
                    </button>
                  </div>
                </form>
              </Nested>

              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </>
  );
}
