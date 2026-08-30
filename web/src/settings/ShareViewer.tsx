//! The share viewer on the settings page: where a published share is read
//! through, and how to make that somewhere of your own.
//!
//! A Share downloads as one file and opens off a disk, which is the whole of
//! what an emailed one needs. A **published** one is a secret gist, and a gist
//! link on its own draws nothing — GitHub renders a gist as source, and its raw
//! URL is served as plain text a browser refuses to draw. The gap between a
//! link and a read is one small static page, and this section is about where
//! that page is.
//!
//! **It is somewhere already.** Verkstead keeps a copy on its own GitHub Pages
//! — [`HOSTED`] — and every link it hands out is composed through that unless
//! this field says otherwise, so a Verkstead nobody has been to the settings
//! page of still hands out links that draw. The field is an override: the same
//! page, served by the human, for anybody who would rather nothing about their
//! shares went past a site of Verkstead's.
//!
//! So the pane is the file, what to do with it, and the field — a job for
//! whoever wants it rather than a job everybody has to do. The card says which
//! of the two viewers this Verkstead is using, because that is the part that
//! has consequences and the part that can be looked up later.
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
import styles from "./ShareViewer.module.css";

/// The copy of the viewer Verkstead hosts, which is what a published share is
/// read through while this field stands empty.
///
/// `HOSTED` in `crates/server/src/sharing.rs` is the other spelling of it, and
/// the server's is the one that composes the links — this is only what the page
/// *says* is happening. Nothing composes the two, so `tests/viewing.test.ts` is
/// what holds them together, the way `tests/template.test.ts` holds the share's
/// slots to the server's.
export const HOSTED = "https://tobico.github.io/verkstead/share-viewer.html";

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

            {/* Which of the two viewers this Verkstead is using, said the same
                way either way: an address, because that is what somebody
                scanning the page came to check. Nothing here is a warning —
                a blank field is a working Verkstead rather than a job left
                undone. */}
            <p class={styles.standing}>
              Published shares are read through{" "}
              <span class={styles.hosted}>
                {told().share_viewer_url || HOSTED}
              </span>
              {told().share_viewer_url === "" ? ", which Verkstead hosts." : "."}
            </p>
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
        // Untouched, for the reason the rest of it is: the endpoint writes the
        // whole of `config.yaml`, and the section under this one is where it is
        // set.
        conflict_resolution: settings?.conflict_resolution ?? "Merge",
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
              link and a read. Verkstead hosts one at{" "}
              <span class={styles.hosted}>{HOSTED}</span>, and links are
              composed through it while the field below is empty.
            </Note>

            {/* And the steps for hosting your own, which is what the field is
                for. A job rather than a setting, so it reads in the order it is
                done: the file, the site, and then the field. Nobody has to —
                the address above is a page that works — so this says who would
                want to before it says how. */}
            <p class={styles.instead}>
              To serve it yourself instead — so that nothing about your shares
              goes past a site of Verkstead's:
            </p>

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

            {/* What a link looks like, spelled out with whichever address is in
                force: the one a comment, a toast and the Share row all carry.
                The gist's id goes after the `#`, where no server ever sees
                it. */}
            <p class={styles.linking}>
              A published share is linked as{" "}
              <code class={styles.link}>
                {told()?.share_viewer_url || HOSTED}#the-gist-id
              </code>
              .
            </p>

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
