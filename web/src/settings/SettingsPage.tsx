//! What Verkstead has been told, and the one form that tells it: the GitHub
//! token every session is handed, and who its commits are by.
//!
//! Said rather than found. Neither of these is read out of a home directory any
//! more — a session gets the token as `GH_TOKEN` and the author through git's
//! environment — so this page is where they come from, and a Verkstead nobody
//! has told anything is one whose sessions cannot reach GitHub and cannot
//! commit. That is what the warnings here say, in those words, rather than
//! leaving it to be found out by a session that failed at midnight.
//!
//! The token field is write-only. What the page shows of a saved token is its
//! last four characters and when it was written — never the token — because a
//! page that could show one is a page that puts it in a history, a screenshot
//! and a scroll-back, and telling one token from another is the whole of what
//! the human needs from it here.
//!
//! One button saves both files, because the server writes both in one request:
//! the author fields are values and the token is an action, so correcting an
//! email address leaves the credentials alone. Clearing is its own press for the
//! same reason — an empty write-only field means nothing was typed.

import { A } from "@solidjs/router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { loadSettings, saveSettings } from "../api/client";
import type {
  SettingsEdit,
  SettingsSaved,
  SettingsView,
  TokenEdit,
  Verified,
} from "../api/types";
import { utcStamp } from "../set/when";

/// The settings as they stand, and the form that writes them.
export function SettingsPage(): JSX.Element {
  const queries = useQueryClient();

  // Read once when the page opens, like the Repos and the Profiles: the files
  // are read afresh on every request, and what changes them is this page.
  const settings = useQuery(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
  }));

  // What has been typed, or `null` while nothing has — the fields follow the
  // server until somebody touches them, the way a branch name does.
  const [name, setName] = createSignal<string | null>(null);
  const [email, setEmail] = createSignal<string | null>(null);

  // The write-only field, which starts empty every time and is emptied again the
  // moment it is spent.
  const [token, setToken] = createSignal("");

  // Whether the token field is open. It always is where there is nothing
  // configured; where there is, replacing is a press, so that the field to paste
  // a credential into is one somebody asked for.
  const [replacing, setReplacing] = createSignal(false);

  // What GitHub made of the token that was last saved, or `null` where this page
  // has not saved one. Not part of the settings: it is the answer to a question
  // asked at the moment of the save, and nothing reads it back afterwards.
  const [verified, setVerified] = createSignal<Verified | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const author = () => told()?.git_author;
  const configured = () => told()?.github_token ?? null;

  const authorName = () => name() ?? author()?.name ?? "";
  const authorEmail = () => email() ?? author()?.email ?? "";

  const typing = () => replacing() || configured() === null;

  /// The account the last saved token authenticates as, and the words GitHub
  /// or `gh` refused it in — one of the two at a time, and neither until this
  /// page has saved a token.
  const account = () => {
    const what = verified();
    return what && "Account" in what ? what.Account.login : null;
  };

  const refused = () => {
    const what = verified();
    return what && "Refused" in what ? what.Refused.why : null;
  };

  const save = useMutation(() => ({
    mutationFn: (edit: SettingsEdit) => saveSettings(edit),
    onSuccess: (saved: SettingsSaved) => {
      setVerified(saved.verified);

      // The fields go back to following the files, and the write-only one is
      // spent: the standing line underneath is the whole of the confirmation.
      setName(null);
      setEmail(null);
      setToken("");
      setReplacing(false);

      // Taken from the answer rather than asked for again, which is the one
      // place this page differs from the Repos and the Profiles: what a save
      // comes back with *is* a fresh read of the two files rather than an echo
      // of what was sent, so a second read would learn nothing and could only
      // disagree with what the human is looking at.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// Save, with whatever is to become of the token.
  const write = (github_token: TokenEdit) =>
    save.mutate({
      git_author: { name: authorName(), email: authorEmail() },
      github_token,
    });

  const submit = (ev: SubmitEvent) => {
    ev.preventDefault();

    // A field left blank is the token left alone. The server reads a token of
    // nothing but whitespace as nothing configured, so a blank one sent as a
    // Set would take the credentials away — which is exactly what `Keep` is
    // here to stop.
    write(
      typing() && token().trim() !== ""
        ? { Set: { token: token() } }
        : "Keep",
    );
  };

  return (
    <div class="list-page">
      {/* The way back to the workbench, in the slot every page keeps for where
          else there is to go. */}
      <A class="back" href="/">
        ← Workbench
      </A>
      <div class="page-head">
        <h1>Settings</h1>
      </div>

      <Switch>
        <Match when={settings.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={settings.isError}>
          <p class="error">
            Could not read the settings: {settings.error?.message}
          </p>
        </Match>
        <Match when={told()}>
          <form class="settings" onSubmit={submit}>
            <section class="github-token">
              <h2>GitHub token</h2>
              {/* What a token that is not there costs, said here rather than
                  found out by a session that could not push at midnight. */}
              <Show when={configured() === null}>
                <p class="warning">
                  No GitHub token is configured, so sessions cannot reach
                  GitHub.
                </p>
              </Show>

              <Show when={configured()}>
                {(saved) => (
                  <p class="token-standing">
                    A token ending{" "}
                    <code class="last-four">{saved().last_four}</code>, saved{" "}
                    <span class="when">{utcStamp(saved().at)}</span>.
                  </p>
                )}
              </Show>

              {/* What GitHub said about the last token saved from this page:
                  the account it authenticates as, or why nobody could be asked.
                  Both are worth showing — a token saved against the wrong
                  account is the failure this page exists to catch. */}
              <Show when={account()}>
                {(login) => (
                  <p class="verified">
                    GitHub says it is <span class="login">{login()}</span>.
                  </p>
                )}
              </Show>
              <Show when={refused()}>
                {(why) => (
                  <p class="error unverified">
                    It is saved, but GitHub would not say whose it is: {why()}
                  </p>
                )}
              </Show>

              <Show
                when={typing()}
                fallback={
                  <div class="token-actions">
                    <button type="button" onClick={() => setReplacing(true)}>
                      Replace
                    </button>
                    <button
                      type="button"
                      class="clear"
                      disabled={save.isPending}
                      onClick={() => {
                        setVerified(null);
                        write("Clear");
                      }}
                    >
                      Clear
                    </button>
                  </div>
                }
              >
                <label for="github-token">
                  Token, pasted — it is stored and never shown again
                </label>
                <input
                  id="github-token"
                  type="password"
                  autocapitalize="off"
                  autocorrect="off"
                  autocomplete="off"
                  spellcheck={false}
                  placeholder="ghp_…"
                  value={token()}
                  onInput={(ev) => setToken(ev.currentTarget.value)}
                />
                {/* Only where there was one to go back to: with nothing
                    configured, the field is the page rather than a detour from
                    it. */}
                <Show when={configured() !== null}>
                  <div class="token-actions">
                    <button
                      type="button"
                      class="cancel"
                      onClick={() => {
                        setReplacing(false);
                        setToken("");
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                </Show>
              </Show>
            </section>

            <section class="git-author">
              <h2>Git author</h2>
              {/* Half an author is as broken as none: git complains by name
                  about whichever half it has not been given. */}
              <Show
                when={
                  (author()?.name ?? "") === "" || (author()?.email ?? "") === ""
                }
              >
                <p class="warning">
                  No git author is configured, so commits inside a session fail
                  asking who the author is.
                </p>
              </Show>

              <label for="author-name">Name</label>
              <input
                id="author-name"
                type="text"
                autocapitalize="words"
                autocorrect="off"
                spellcheck={false}
                placeholder="Ada Lovelace"
                value={authorName()}
                onInput={(ev) => setName(ev.currentTarget.value)}
              />

              <label for="author-email">Email</label>
              <input
                id="author-email"
                type="email"
                inputmode="email"
                autocapitalize="off"
                autocorrect="off"
                spellcheck={false}
                placeholder="ada@example.com"
                value={authorEmail()}
                onInput={(ev) => setEmail(ev.currentTarget.value)}
              />
            </section>

            <div class="settings-buttons">
              <button type="submit" disabled={save.isPending}>
                Save
              </button>
            </div>

            {/* A server that could not write the files, which is the one thing
                here that is an error rather than an answer. Said loudly: a
                settings page that quietly saved nothing is how credentials go
                missing. */}
            <Show when={save.isError}>
              <p class="error">
                The settings could not be saved: {save.error?.message}
              </p>
            </Show>
          </form>
        </Match>
      </Switch>
    </div>
  );
}
