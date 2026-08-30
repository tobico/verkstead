//! The share viewer on the settings page: the page Verkstead ships, and where
//! the human has hosted it.
//!
//! A Share downloads as one file and opens off a disk, which is the whole of
//! what an emailed one needs. A **published** one is a secret gist, and a gist
//! link on its own draws nothing — GitHub renders a gist as source, and its raw
//! URL is served as plain text a browser refuses to draw. The gap between a
//! link and a read is one small static page, and this section is where that
//! page is collected and where the human says what they did with it.
//!
//! So the two halves here are one job read in order: take the file away, put it
//! on a public site of your own, and say where it went. The card carries the
//! last of those, because that is the part that can be looked up later and the
//! part that has consequences — a Verkstead that does not know where the viewer
//! is comments on a pull request with the gist itself.
//!
//! Nothing here is a secret. The URL is a public page's, and it goes into a
//! comment the moment a share is published through it, so it reads back exactly
//! as it was written — unlike the token two cards above.
//!
//! Two panes and one query, like the sections above: a card in the middle pane
//! and the controls in the details pane it opens, at `/settings/share-viewer`,
//! both drawn from the one read of the two settings files.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { loadSettings, saveSettings, shareViewerPath } from "../api/client";
import type { SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldPaths } from "./held";
import styles from "./ShareViewer.module.css";

/// The settings as they stand, read once for the two panes that draw them —
/// the same read, by the same key, that the sections above this one make.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// Where the viewer is hosted, as the card that opens the section.
export function ShareViewerCard(props: {
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
      <Match when={settings.data}>
        {(told) => (
          <CardButton
            as="article"
            class={styles.shareViewerCard}
            open={props.open}
            press={props.press}
          >
            <h2>Share viewer</h2>

            {/* What is lost by leaving this alone, said here rather than found
                out by whoever opens the link on a pull request. Not an error:
                the share is published either way, and what suffers is the
                read. */}
            <Show when={told().share_viewer_url === ""}>
              <p class={styles.warning}>
                No share viewer is hosted, so a published share is linked as the
                gist itself — which GitHub draws as source rather than as the
                conversation.
              </p>
            </Show>

            <Show when={told().share_viewer_url !== ""}>
              <p class={styles.standing}>
                Published shares are read through{" "}
                <span class={styles.hosted}>{told().share_viewer_url}</span>.
              </p>
            </Show>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the details pane the card opens: the page to take away, what to do with
/// it, and the field that says where it went.
export function ShareViewerPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed, or `null` while nothing has — the field follows the
  // server until somebody touches it, as every other field on this page does.
  const [typed, setTyped] = createSignal<string | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const url = () => typed() ?? told()?.share_viewer_url ?? "";

  const save = useMutation(() => ({
    mutationFn: (share_viewer_url: string) => {
      const settings = told();

      return saveSettings({
        // The rest of both files as they stand: the endpoint writes them whole,
        // and this form has no business with any of it.
        git_author: settings?.git_author ?? { name: "", email: "" },
        github_token: "Keep",
        rust_build_cache: {
          enabled: settings?.rust_build_cache.enabled ?? true,
          size: settings?.rust_build_cache.size_configured
            ? (settings?.rust_build_cache.size ?? "")
            : "",
        },
        share_viewer_url,
        // And the paths as the read left them: one request writes the whole of
        // `config.yaml`, so a list this form left out would be a list it
        // emptied — see [`heldPaths`].
        ...heldPaths(settings),
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

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();
    save.mutate(url());
  };

  return (
    <>
      <PaneHead back={{ to: "Settings", go: props.back }} title="Share viewer" />

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
          <div class={styles.shareViewer}>
            <Note>
              A published share is a secret gist, and GitHub draws a gist as
              source. The share viewer is a small page that draws it as the
              conversation instead: it fetches the share from GitHub in the
              reader's own browser, and it is the whole of what stands between a
              link and a read.
            </Note>

            {/* The steps, in the order they are done, because this is a job
                rather than a setting: the file, the site, and then the field
                below. */}
            <ol class={styles.steps}>
              <li>
                <a
                  class={styles.download}
                  href={shareViewerPath()}
                  download="verkstead-share-viewer.html"
                >
                  Download the share viewer
                </a>
              </li>
              <li>
                Put it on a public site of your own — a GitHub Pages repository
                is what it was written for. It needs no build and reaches
                nothing but GitHub.
              </li>
              <li>Paste the page's address here.</li>
            </ol>

            <form class={styles.hosting} onSubmit={commit}>
              <label for="share-viewer-url">Where you hosted it</label>
              <div class={styles.field}>
                <input
                  id="share-viewer-url"
                  type="url"
                  inputmode="url"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck={false}
                  placeholder="https://you.github.io/verkstead-shares/"
                  value={url()}
                  onInput={(ev) => setTyped(ev.currentTarget.value)}
                />
                <button type="submit" disabled={save.isPending}>
                  Save
                </button>
              </div>
            </form>

            {/* What the setting buys, spelled out with the address in it: the
                link a pull request comment carries once a share is published.
                The gist's id goes after the `#`, where no server ever sees
                it. */}
            <Show when={told()?.share_viewer_url !== ""}>
              <p class={styles.linking}>
                A published share is linked as{" "}
                <code class={styles.link}>
                  {told()?.share_viewer_url}#the-gist-id
                </code>
                .
              </p>
            </Show>

            <Show when={save.isError}>
              <ErrorLine class={styles.failure}>
                The settings could not be saved: {save.error?.message}
              </ErrorLine>
            </Show>
          </div>
        </Match>
      </Choose>
    </>
  );
}
