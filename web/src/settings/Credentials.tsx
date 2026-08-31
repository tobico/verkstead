//! The credentials Verkstead has been told, and the one form that tells it: the
//! GitHub token every session is handed, and who its commits are by.
//!
//! The head of the settings pane rather than a page of its own. What sits under
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
//! the card rather than in the form, because whoever needs to read them is
//! precisely whoever is not editing.
//!
//! Two halves in two panes, which is what the settings page is now. The card is
//! a [`CardButton`](../CardButton.tsx) in the middle pane, carrying what is
//! configured — the token's state, the author, and the warnings about whichever
//! is missing — and pressing it opens the form in the details pane beside it, at
//! `/settings/github`. The modal it was is gone: a form that stood over the page
//! is a pane of its own now, and the card reads as open while that pane is,
//! which is what every other card in this app says about itself.
//!
//! Both halves read the one query. They are two views of the same two files, and
//! a read apiece would be two reads of them — the cache is what makes the second
//! caller free.
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
//!
//! And one switch stands beside them in the pane, saving itself: whether a
//! Conversation's record is published and linked on its pull request when the
//! work settles to Done. It is here rather than in a section of its own because
//! what it turns on is done with the token above it and under the account that
//! token belongs to — a switch two panes away from the credential it spends
//! would be a switch nobody could see the cost of. It saves on the flip, the way
//! the build cache's does, and it carries the author as the *server* holds it
//! rather than as the fields hold it: a flip is not a submit, and half a typed
//! email address is nobody's author.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { Switch as Toggle } from "../Switch";
import { loadSettings, saveSettings } from "../api/client";
import { useReading } from "../freshness";
import type {
  SettingsEdit,
  SettingsSaved,
  SettingsView,
  TokenEdit,
  TokenSaved,
  Verified,
} from "../api/types";
import { Empty, ErrorLine, Note } from "../notices";
import { utcStamp } from "../set/when";
import { PaneHead } from "../workbench/PaneHead";
import { heldPaths } from "./held";
import styles from "./Credentials.module.css";

/// The two files as they stand, read once for the two panes that draw them.
///
/// Merged rather than frozen, because this is not a payload that cannot change:
/// another device may write the same two files, and a frozen query is one the
/// catch-up read on reconnect could never reach. There is no list in it for the
/// key to match by — what the merge does here is leave the fields that did not
/// change alone, and whatever is drawn from them with them.
function useCredentials() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// What is known about the saved token, which is everything but the token.
///
/// Drawn in both panes: on the card it is what is configured, and in the form it
/// is what Replace and Clear are about.
function standing(saved: TokenSaved): JSX.Element {
  return (
    <p class={styles.tokenStanding}>
      A token ending <code class={styles.lastFour}>{saved.last_four}</code>,
      saved <span>{utcStamp(saved.at)}</span>.
    </p>
  );
}

/// Half an author is as broken as none: git complains by name about whichever
/// half it has not been given.
function authorless(told: SettingsView): boolean {
  return told.git_author.name === "" || told.git_author.email === "";
}

/// And nothing at all is worth no line of its own, because the warning says it:
/// what the card is for is showing what *is* configured.
function authored(told: SettingsView): boolean {
  return told.git_author.name !== "" || told.git_author.email !== "";
}

/// The credentials as they stand, as the card that opens them.
///
/// A card rather than a section with an Edit button on its heading, because that
/// is what the rest of this pane is: something standing in a pane that is
/// selected and opened into the pane beside it. Drawn as an `article`, the way
/// every card holding more than a run of text is — a button may not have
/// paragraphs inside it, and `CardButton` puts the press, the keyboard and the
/// role that says what it is on the article instead.
export function GithubCard(props: {
  /// Whether the form beside this is the pane that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useCredentials();

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
      <Match when={settings.data}>
        {(told) => (
          <CardButton
            as="article"
            class={styles.githubCard}
            open={props.open}
            press={props.press}
          >
            <h2>GitHub and git author</h2>

            {/* What a token that is not there costs, said here rather than
                found out by a session that could not push at midnight. */}
            <Show when={told().github_token === null}>
              <p class={styles.warning}>
                No GitHub token is configured, so sessions cannot reach GitHub.
              </p>
            </Show>
            <Show when={told().github_token}>{(saved) => standing(saved())}</Show>

            <Show when={authorless(told())}>
              <p class={styles.warning}>
                No git author is configured, so commits inside a session fail
                asking who the author is.
              </p>
            </Show>
            {/* Written the way git writes an author, which is the form it is
                going to be used in. */}
            <Show when={authored(told())}>
              <p class={styles.authorStanding}>
                Commits are by{" "}
                <span class={styles.authorName}>{told().git_author.name}</span>{" "}
                <span class={styles.authorEmail}>
                  &lt;{told().git_author.email}&gt;
                </span>
                .
              </p>
            </Show>
          </CardButton>
        )}
      </Match>
    </Switch>
  );
}

/// And the form that rewrites them, which is the details pane the card opens.
///
/// One form for both files, because the server writes them in one request, so
/// there is one Save. There is no Cancel: a details pane is left by opening
/// something else or by the way back a narrow window draws, and a button that
/// said the same thing again would be a second way out of a pane that has
/// one.
export function GithubPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useCredentials();

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
  // has been saved from here. Not part of the settings: it is the answer to a
  // question asked at the moment of the save, and nothing reads it back
  // afterwards. Said in this pane rather than on the card, because this is where
  // the press that asked it was made and the pane is still standing to hear the
  // answer.
  const [verified, setVerified] = createSignal<Verified | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const author = () => told()?.git_author;
  const configured = () => told()?.github_token ?? null;

  const authorName = () => name() ?? author()?.name ?? "";
  const authorEmail = () => email() ?? author()?.email ?? "";

  /// Everything in `config.yaml` this pane is not about, as it stands.
  ///
  /// The endpoint writes the whole file in one request, so a save leaving one of
  /// these out would be a save emptying it. The defaults are what the server
  /// would write anyway, for the moment before the read has landed.
  const held = () => ({
    rust_build_cache: {
      enabled: told()?.rust_build_cache.enabled ?? true,
      size: told()?.rust_build_cache.size_configured
        ? (told()?.rust_build_cache.size ?? "")
        : "",
    },
    // And how a conflicted pull request is resolved, likewise.
    conflict_resolution: told()?.conflict_resolution ?? "Merge",
    // And the Watched Paths and the binds the settings hold, again for that
    // reason — a list this form left out would be a list it emptied, and one
    // of them is the boundary itself. See [`heldPaths`].
    ...heldPaths(told()),
  });

  /// Where the share-on-Done switch sits, which is off until somebody has been
  /// here.
  const sharing = () => told()?.share_on_done ?? false;

  const typing = () => replacing() || configured() === null;

  /// The account the last saved token authenticates as, and the words GitHub
  /// or `gh` refused it in — one of the two at a time, and neither until this
  /// pane has saved a token.
  const account = () => {
    const what = verified();
    return what && "Account" in what ? what.Account.login : null;
  };

  const refused = () => {
    const what = verified();
    return what && "Refused" in what ? what.Refused.why : null;
  };

  /// And the scopes GitHub says it has not been given, of the ones Verkstead
  /// needs. Empty on a token that can do everything asked of it — and on one
  /// GitHub named no scopes for at all, which says nothing either way.
  const missing = () => {
    const what = verified();
    return what && "Account" in what ? what.Account.missing : [];
  };

  /// Spend the form. The pane stays — what was saved is what the fields are
  /// showing — but the write-only field goes back to empty, and the author
  /// fields go back to following the server, which has just been told what they
  /// said.
  const spent = () => {
    setName(null);
    setEmail(null);
    setToken("");
    setReplacing(false);
  };

  const save = useMutation(() => ({
    mutationFn: (edit: SettingsEdit) => saveSettings(edit),
    onSuccess: (saved: SettingsSaved) => {
      setVerified(saved.verified);
      spent();

      // Taken from the answer rather than asked for again, which is the one
      // place this differs from the Repos and the Profiles: what a save
      // comes back with *is* a fresh read of the two files rather than an echo
      // of what was sent, so a second read would learn nothing and could only
      // disagree with what the human is looking at.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// Save, with whatever is to become of the token.
  ///
  /// The switch below rides along as it stands, along with everything else this
  /// form is not about — see [`held`]: pressing Save is about the fields, and a
  /// switch that came back off with the author would be a switch the form
  /// flipped.
  const write = (github_token: TokenEdit) =>
    save.mutate({
      git_author: { name: authorName(), email: authorEmail() },
      github_token,
      share_on_done: sharing(),
      ...held(),
    });

  /// And the switch's own save, which is its own press.
  ///
  /// Not the form's: the author goes back as the server holds it rather than as
  /// the fields do, so a flip made halfway through typing an address writes
  /// nothing but the switch, and nothing spends the fields either — what was
  /// being typed is still being typed afterwards.
  const flipping = useMutation(() => ({
    mutationFn: (share_on_done: boolean) =>
      saveSettings({
        git_author: told()?.git_author ?? { name: "", email: "" },
        // Untouched, for the reason a blank field is a `Keep`.
        github_token: "Keep",
        share_on_done,
        ...held(),
      }),
    onSuccess: (saved: SettingsSaved) =>
      queries.setQueryData(["settings"], saved.settings),
  }));

  const submit = (ev: SubmitEvent) => {
    ev.preventDefault();

    // A field left blank is the token left alone. The server reads a token of
    // nothing but whitespace as nothing configured, so a blank one sent as a
    // Set would take the credentials away — which is exactly what `Keep` is
    // here to stop.
    write(
      typing() && token().trim() !== "" ? { Set: { token: token() } } : "Keep",
    );
  };

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Settings", go: props.back }}
          title="GitHub and git author"
        />
      </PaneSticky>

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
          <form class={styles.form} onSubmit={submit}>
            <section>
              <h3>GitHub token</h3>
              <Show when={configured()}>{(saved) => standing(saved())}</Show>

              {/* What GitHub said about the last token saved from here: the
                  account it authenticates as, or why nobody could be asked.
                  Both are worth showing — a token saved against the wrong
                  account is the failure this form exists to catch. */}
              <Show when={account()}>
                {(login) => (
                  <p class={styles.verified}>
                    GitHub says it is <span class={styles.login}>{login()}</span>
                    .
                  </p>
                )}
              </Show>

              {/* And what it may not do, on a token that authenticates
                  perfectly and cannot publish a share. A line of its own rather
                  than part of the one above, because it is a different thing to
                  do about it: the account is right and the token needs re-
                  issuing with another box ticked. */}
              <Show when={missing().length > 0}>
                <p class={styles.unscoped}>
                  It cannot publish a share: GitHub has not given it the{" "}
                  <code class={styles.scope}>{missing().join(", ")}</code>{" "}
                  scope. Re-issue the token on GitHub with that ticked and save
                  it here again.
                </p>
              </Show>
              <Show when={refused()}>

                {(why) => (
                  <ErrorLine class={styles.unverified}>
                    It is saved, but GitHub would not say whose it is: {why()}
                  </ErrorLine>
                )}
              </Show>

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
                    from it. */}
                <Show when={configured() !== null}>
                  <div class={styles.tokenActions}>
                    <button
                      type="button"
                      class={styles.keep}
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
              <h3>Sharing</h3>

              {/* In a wrapper of its own, so the form's own label rule — which
                  is about the labels over the fields — leaves the switch's to
                  `Switch.module.css`. */}
              <div class={styles.sharing}>
                <Toggle
                  label="Share to pull request on Done"
                  on={sharing()}
                  disabled={flipping.isPending}
                  flip={(on) => flipping.mutate(on)}
                />
              </div>

              <Note>
                When a conversation settles to Done, its record is published as
                a secret gist and linked in a comment on its pull request. It is
                published with the token above, which needs the gist scope.
              </Note>

              <Show when={flipping.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {flipping.error?.message}
                </ErrorLine>
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
            </div>

            {/* A server that could not write the files, which is the one
                thing here that is an error rather than an answer. Said
                loudly: a settings page that quietly saved nothing is how
                credentials go missing. */}
            <Show when={save.isError}>
              <ErrorLine class={styles.failure}>
                The settings could not be saved: {save.error?.message}
              </ErrorLine>
            </Show>
          </form>
        </Match>
      </Switch>
    </>
  );
}
