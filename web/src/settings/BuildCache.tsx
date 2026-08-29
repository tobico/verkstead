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

import { Switch } from "../Switch";
import { loadSettings, saveSettings } from "../api/client";
import type { SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import app from "../App.module.css";
import styles from "./BuildCache.module.css";

/// The build cache section, whole.
export function BuildCache(): JSX.Element {
  const queries = useQueryClient();

  // The same read the credentials above are drawn from, by the same key: one
  // payload holds both files, and two queries over it would be two opinions
  // about what is saved.
  const settings = useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));

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

  const flip = (enabled: boolean) =>
    save.mutate({ enabled, size: size() });

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();
    save.mutate({ enabled: cache()?.enabled ?? true, size: size() });
  };

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
      <Match when={cache()}>
        {(set) => (
          <section class={styles.buildCache}>
            <div class={app.sectionHead}>
              <h2>Rust build cache</h2>
            </div>

            <Switch
              label="Share one build cache between all sessions"
              on={set().enabled}
              disabled={save.isPending}
              flip={flip}
            />

            <Note>
              Crates are downloaded and compiled once for this machine instead of
              once per conversation. A change applies to the next session
              started; one already running keeps what it began with.
            </Note>

            {/* What the human cannot fix from here, said where they would
                otherwise wonder why nothing got faster. Not an error: the
                downloads are still shared, and the builds still work. */}
            <Show when={!set().compiles_cached}>
              <p class={styles.warning}>
                No sccache is installed where the server can see it, so
                dependency <em>compiles</em> are not cached — only the crate
                downloads. Install sccache on the server to cache the compiling
                too.
              </p>
            </Show>

            {/* The size is sccache's, so it is asked for only where there is an
                sccache to read it. */}
            <Show when={set().enabled && set().compiles_cached}>
              <form class={styles.size} onSubmit={commit}>
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
                    // The default, so an empty box reads as the size nobody has
                    // chosen rather than as no size at all.
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
          </section>
        )}
      </Match>
    </Choose>
  );
}
