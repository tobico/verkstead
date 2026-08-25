//! The credentials Verkstead has been told, and the one form that tells it: the
//! GitHub token every session is handed, and who its commits are by.
//!
//! The top of the settings page rather than a page of its own. What sits under
//! it there — the Agent Profiles, the Repos — is what a Conversation is settled
//! against, and this is what Verkstead itself was told, which is why it is
//! first.
//!
//! Said rather than found. Neither of these is read out of a home directory any
//! more — a session gets the token as `GH_TOKEN` and the author through git's
//! environment — so this form is where they come from, and a Verkstead nobody
//! has told anything is one whose sessions cannot reach GitHub and cannot
//! commit. That is what the warnings here say, in those words, rather than
//! leaving it to be found out by a session that failed at midnight. They are on
//! the page rather than in the form, because whoever needs to read them is
//! precisely whoever is not editing.
//!
//! What the page holds is a summary — the token's state and the author, with the
//! warnings about whichever of them is missing — and the form that rewrites it is
//! a modal, as the Profiles' and the Repos' are. What this section is read for is
//! checking that a machine is set up; the form is wanted once and then not again
//! for months, and it had the top of the page every time.
//!
//! The token field is write-only. What is shown of a saved token is its
//! last four characters and when it was written — never the token — because a
//! page that could show one is a page that puts it in a history, a screenshot
//! and a scroll-back, and telling one token from another is the whole of what
//! the human needs from it here.
//!
//! One button saves both files, because the server writes both in one request:
//! the author fields are values and the token is an action, so correcting an
//! email address leaves the credentials alone. Clearing is its own press for the
//! same reason — an empty write-only field means nothing was typed.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { Modal } from "../Modal";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import { useReading } from "../freshness";
import type {
  SettingsEdit,
  SettingsSaved,
  SettingsView,
  TokenEdit,
  Verified,
} from "../api/types";
import { Empty, ErrorLine } from "../notices";
import { utcStamp } from "../set/when";
import app from "../App.module.css";
import styles from "./Credentials.module.css";

/// The credentials as they stand, and the form that writes them.
export function Credentials(): JSX.Element {
  const queries = useQueryClient();

  // Read once when the page opens, like the Repos and the Profiles under it:
  // the files are read afresh on every request, and what changes them is this
  // form.
  const settings = useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,

    // Merged rather than frozen, because this is not a payload that cannot
    // change: another device may write the same two files, and a frozen query
    // is one the catch-up read on reconnect could never reach. There is no list
    // in it for the key to match by — what the merge does here is leave the
    // fields that did not change alone, and the summary above them with them.
    freshness: { reconcile: "id" },
  }));

  // Whether the form is up.
  const [open, setOpen] = createSignal(false);

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

  // What GitHub made of the token that was last saved, or `null` where nothing
  // has been saved from here. Not part of the settings: it is the answer to a question
  // asked at the moment of the save, and nothing reads it back afterwards. It is
  // said on the page rather than in the form, because the form is gone by the
  // time there is an answer to say.
  const [verified, setVerified] = createSignal<Verified | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const author = () => told()?.git_author;
  const configured = () => told()?.github_token ?? null;

  const authorName = () => name() ?? author()?.name ?? "";
  const authorEmail = () => email() ?? author()?.email ?? "";

  const typing = () => replacing() || configured() === null;

  /// Half an author is as broken as none: git complains by name about whichever
  /// half it has not been given.
  const authorless = () =>
    (author()?.name ?? "") === "" || (author()?.email ?? "") === "";

  /// And nothing at all is worth no line of its own, because the warning says
  /// it: what the summary is for is showing what *is* configured.
  const authored = () =>
    (author()?.name ?? "") !== "" || (author()?.email ?? "") !== "";

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

  /// Take the form away, saved or not. What was typed goes with it: a form
  /// opened again follows what the files say now, and a half-typed one kept from
  /// last time would be a draft nothing here promised to keep.
  const shut = () => {
    setOpen(false);
    setName(null);
    setEmail(null);
    setToken("");
    setReplacing(false);
  };

  const save = useMutation(() => ({
    mutationFn: (edit: SettingsEdit) => saveSettings(edit),
    onSuccess: (saved: SettingsSaved) => {
      setVerified(saved.verified);

      // The form goes, and the summary underneath — which the answer has just
      // rewritten — is the whole of the confirmation.
      shut();

      // Taken from the answer rather than asked for again, which is the one
      // place this differs from the Repos and the Profiles: what a save
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

  /// What is known about the saved token, which is everything but the token.
  /// Drawn in both places: on the page it is the summary, and in the form it is
  /// what Replace and Clear are about.
  const standing = () => (
    <Show when={configured()}>
      {(saved) => (
        <p class={styles.tokenStanding}>
          A token ending <code class={styles.lastFour}>{saved().last_four}</code>,
          saved <span>{utcStamp(saved().at)}</span>.
        </p>
      )}
    </Show>
  );

  return (
    <Switch>
      <Match when={settings.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          Could not read the settings: {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={told()}>
        <section class={styles.credentials}>
          {/* The heading, with the one thing there is to do to what is under it
              on the other end of its line — as the two lists below have. */}
          <div class={app.sectionHead}>
            <h2>GitHub and git author</h2>
            <QuietButton onClick={() => setOpen(true)}>Edit</QuietButton>
          </div>

          {/* What a token that is not there costs, said here rather than found
              out by a session that could not push at midnight. */}
          <Show when={configured() === null}>
            <p class={styles.warning}>
              No GitHub token is configured, so sessions cannot reach GitHub.
            </p>
          </Show>
          {standing()}

          {/* What GitHub said about the last token saved from here: the account
              it authenticates as, or why nobody could be asked. Both are worth
              showing — a token saved against the wrong account is the failure
              this form exists to catch. */}
          <Show when={account()}>
            {(login) => (
              <p class={styles.verified}>
                GitHub says it is <span class={styles.login}>{login()}</span>.
              </p>
            )}
          </Show>
          <Show when={refused()}>
            {(why) => (
              <ErrorLine class={styles.unverified}>
                It is saved, but GitHub would not say whose it is: {why()}
              </ErrorLine>
            )}
          </Show>

          <Show when={authorless()}>
            <p class={styles.warning}>
              No git author is configured, so commits inside a session fail
              asking who the author is.
            </p>
          </Show>
          {/* Written the way git writes an author, which is the form it is
              going to be used in. */}
          <Show when={authored()}>
            <p class={styles.authorStanding}>
              Commits are by{" "}
              <span class={styles.authorName}>{author()?.name}</span>{" "}
              <span class={styles.authorEmail}>
                &lt;{author()?.email}&gt;
              </span>
              .
            </p>
          </Show>

          {/* One form for both files, drawn over the page: the server writes
              them in one request, so there is one Save. */}
          <Modal
            class={styles.form!}
            open={open()}
            close={shut}
            name="GitHub and git author"
          >
            <form onSubmit={submit}>
              <section>
                <h3>GitHub token</h3>
                {standing()}

                <Show
                  when={typing()}
                  fallback={
                    <div class={styles.tokenActions}>
                      <button type="button" onClick={() => setReplacing(true)}>
                        Replace
                      </button>
                      <button
                        type="button"
                        class={styles.clear}
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
                      configured, the field is the form rather than a detour
                      from it. Named for what it does rather than "cancel",
                      which is the way out of the whole form on the line
                      below. */}
                  <Show when={configured() !== null}>
                    <div class={styles.tokenActions}>
                      <button
                        type="button"
                        class={styles.cancel}
                        onClick={() => {
                          setReplacing(false);
                          setToken("");
                        }}
                      >
                        Keep the saved token
                      </button>
                    </div>
                  </Show>
                </Show>
              </section>

              <section>
                <h3>Git author</h3>

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

              <div class={styles.buttons}>
                <button type="submit" disabled={save.isPending}>
                  Save
                </button>
                <button type="button" class={styles.cancel} onClick={shut}>
                  Cancel
                </button>
              </div>

              {/* A server that could not write the files, which is the one
                  thing here that is an error rather than an answer, and the one
                  that keeps the form up. Said loudly: a settings page that
                  quietly saved nothing is how credentials go missing. */}
              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </form>
          </Modal>
        </section>
      </Match>
    </Switch>
  );
}
