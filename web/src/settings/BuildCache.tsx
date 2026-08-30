//! The shared Rust build cache, on the settings page: whether sessions get one,
//! and how big its compiled half may grow.
//!
//! The one thing on this page that is about a **Sandbox** rather than about who
//! Verkstead is. Everything else a sandbox is given is configured where the
//! watched paths are, because every extra bind is a hole in a boundary somebody
//! opened on purpose; this one is Verkstead's own directory, holding nothing but
//! build output, and the switch here is the one that *closes* it. That is what
//! makes it safe to reach from a phone.
//!
//! It is on with nothing configured, which is the whole shape of the feature: a
//! human should not have a slower machine for never having found this section.
//! So the switch says where it stands rather than whether anybody has touched
//! it, and the size field shows the default as a placeholder rather than as a
//! value somebody chose.
//!
//! Two halves in two panes, which is what the settings page is: a card in the
//! middle pane saying how the cache stands, and the controls that change it in
//! the details pane it opens, at `/settings/build-cache`. What the human cannot
//! fix from the browser — no sccache where the server can see one — is on the
//! card as well as in the pane, the way the credentials' warnings are: whoever
//! needs to read it is precisely whoever is not editing.
//!
//! Both halves read the one query, which is the one the credentials above them
//! read: one payload holds both files, and a read apiece would be two opinions
//! about what is saved.
//!
//! Two ways to save, because there are two kinds of control. The switch saves
//! itself the moment it is flipped, the way the notifications switch does — a
//! switch that needed a second press to mean anything is not a switch. The size
//! is typed, so it saves on a press of its own; nothing is committed while
//! somebody is still halfway through writing `30`.
//!
//! Both go through the one settings endpoint, which writes both files: the
//! author rides along as it stands and the token is left alone, so saving a
//! cache size cannot lose either.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { Switch } from "../Switch";
import { loadSettings, saveSettings } from "../api/client";
import type { BuildCacheView, SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import styles from "./BuildCache.module.css";

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

/// What is said where there is no sccache for the server to compile through.
///
/// Drawn on the card and in the pane alike, because it is the same sentence in
/// both: something the human would have to leave the browser to fix, said where
/// they would otherwise wonder why nothing got faster. Not an error — the
/// downloads are still shared, and the builds still work.
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
/// are would be wrong exactly where somebody has just turned it off. The setup
/// card's own warning is gated the same way — see
/// `ConversationView::compiles_uncached`, which the server works out with the
/// switch already in hand.
function warned(cache: BuildCacheView): boolean {
  return cache.enabled && !cache.compiles_cached;
}

/// The cache as it stands, as the card that opens it.
///
/// What is on the card is what somebody scanning the page is after: whether
/// sessions share a cache at all, how large its compiled half may grow, and the
/// one thing about it that wants doing somewhere else. The controls are in the
/// pane.
export function BuildCacheCard(props: {
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
            class={styles.buildCacheCard}
            open={props.open}
            press={props.press}
          >
            <h2>Rust build cache</h2>

            <Show when={warned(cache())}>{uncompiled()}</Show>

            <p class={styles.standing}>
              <Show
                when={cache().enabled}
                fallback="Off, so every session downloads and compiles its own crates."
              >
                Crates are downloaded{" "}
                <Show when={cache().compiles_cached}>and compiled </Show>
                once for this machine
                <Show when={cache().compiles_cached}>
                  , in up to <span class={styles.size}>{cache().size}</span> of
                  compiled output
                </Show>
                .
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the controls that change it, which is the details pane the card opens.
///
/// There is no Save over the whole of it and no Cancel: the switch is its own
/// press and the size has one of its own, and a details pane is left by opening
/// something else or by the way back a narrow window draws.
export function BuildCachePane(props: {
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
        // Untouched, for the reason the author is: the endpoint writes the
        // whole of `config.yaml`, and the section below is where this is set.
        share_viewer_url: told()?.share_viewer_url ?? "",
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
        <PaneHead
          back={{ to: "Settings", go: props.back }}
          title="Rust build cache"
        />
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
            <div class={styles.buildCache}>
              <Switch
                label="Share one build cache between all sessions"
                on={set().enabled}
                disabled={save.isPending}
                flip={flip}
              />

              <Note>
                Crates are downloaded and compiled once for this machine instead
                of once per conversation. A change applies to the next session
                started; one already running keeps what it began with.
              </Note>

              <Show when={warned(set())}>{uncompiled()}</Show>

              {/* The size is sccache's, so it is asked for only where there is
                  an sccache to read it. */}
              <Show when={set().enabled && set().compiles_cached}>
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
    </>
  );
}
