//! The wizard's last step: who the work is committed as, and the press that
//! closes the wizard.
//!
//! **What git needs, and nothing beside it.** A name and an email, both
//! required, because that is what git asks every commit for — and the GitHub
//! token, optional, because GitHub may not be in use at all. The objective is
//! the two halves of an author (ADR-0016); the token rides along because this
//! is where somebody is being asked for credentials and there is nowhere better
//! to put it.
//!
//! **Prefilled from what the server could see, and told by nobody but the
//! human.** The settings module's rule is *told, not found* — a session gets
//! the author through git's environment rather than out of a home directory —
//! and a prefill somebody confirms is still telling. So the fields open with
//! whatever `git config --global` and the server's own environment could offer,
//! each labelled with where it was found, and **nothing is written until
//! Next**. A field the server found nothing for opens empty, and one
//! Verkstead already holds a value for shows that value rather than a prefill:
//! see `loadGitPrefill` in `../api/client.ts`.
//!
//! **The save is the settings page's own**, verification included. One request
//! writes both files, so everything in `config.yaml` this step is not about
//! rides along untouched — see [`heldConfig`](../settings/held.ts) — and the
//! GitHub login is what that request answers with rather than anything typed
//! here: a token that authenticates as the *wrong* account is the mistake this
//! catches, and it can only be caught by showing whose it is.
//!
//! **Which is why a token stops the wizard for one more press.** A save that
//! verified something has something to be read — the login, a scope GitHub did
//! not give it, or why GitHub would not answer — so the step stays up with the
//! answer on it and Next closes the wizard next time. A save with no token
//! to verify has nothing to stop for and goes straight on.
//!
//! **And the last Next takes the mode off for this run.** Nothing is
//! written by it: the author went through the save a moment before, and the
//! mode is a fact about this process — see `crates/server/src/onboarding.rs`.
//! What comes back is the machine read again with the mode off, which is what
//! makes every URL a page again; the app lands on `/compose` from there.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";

import {
  finishOnboarding,
  loadGitPrefill,
  loadSettings,
  saveSettings,
} from "../api/client";
import type {
  Prefilled,
  SettingsEdit,
  SettingsSaved,
  Source,
  TokenEdit,
} from "../api/types";
import { useReading } from "../freshness";
import { ErrorLine, Note } from "../notices";
import { heldConfig } from "../settings/held";
import { utcStamp } from "../set/when";
import { COMPOSE } from "./steps";
import styles from "./Git.module.css";

/// Where a prefilled value was found, in the words the human would go and look
/// in. The viewer's own, like the install commands on the step before this one:
/// what the server says is *where*, and how that place is named is the same
/// sentence on every Verkstead.
///
/// A function apiece rather than an element apiece: an element is a node, and a
/// node drawn under two fields would be one field taking it off the other.
const SOURCES: Record<Source, () => JSX.Element> = {
  GitConfig: () => (
    <>
      From this machine's <code>git config --global</code>.
    </>
  ),
  GhToken: () => (
    <>
      From <code>GH_TOKEN</code>, in this server's own environment.
    </>
  ),
  GithubToken: () => (
    <>
      From <code>GITHUB_TOKEN</code>, in this server's own environment.
    </>
  ),
  HostGh: () => (
    <>
      From the login this machine's <code>gh</code> is holding.
    </>
  ),
};

/// What the machine offered for one field: a prefill, `null` where it offered
/// nothing, and `undefined` for the moment before the read has landed. The
/// three are one thing to a field — there is nothing to draw under it.
type Offered = Prefilled | null | undefined;

/// The step. It reads what it needs itself: what Verkstead has been told, and
/// what the machine could offer for whatever it has not.
export function Git(): JSX.Element {
  const queries = useQueryClient();

  // What is configured — the same query the settings page reads, under the same
  // key, so the save's answer lands in one cache rather than two.
  const settings = useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));

  // And what the machine could fill the empty fields with. Read once: it is
  // what `git config` and the environment said a moment ago, and neither of
  // them changes while somebody types in a box.
  const prefill = useReading(() => ({
    queryKey: ["onboarding", "git"],
    queryFn: loadGitPrefill,
    freshness: "static",
  }));

  // What has been typed, or `null` while nothing has been: a field follows the
  // server until somebody touches it, the way the settings form's does.
  const [name, setName] = createSignal<string | null>(null);
  const [email, setEmail] = createSignal<string | null>(null);
  const [token, setToken] = createSignal<string | null>(null);

  // And what the last save answered, or `null` while there is something in the
  // fields that has not been saved. This is what makes the press that follows a
  // save the press that closes the wizard rather than a second save — and
  // typing anything clears it, so a corrected token is saved rather than
  // skipped past.
  const [saved, setSaved] = createSignal<SettingsSaved | null>(null);

  const told = () => settings.data;
  const found = () => prefill.data;

  /// One field as it stands: what was typed, what Verkstead holds, or what the
  /// machine offered — in that order, which is the order of who said it most
  /// recently.
  const standing = (
    typed: string | null,
    held: string | undefined,
    offered: Offered,
  ): string => typed ?? held ?? offered?.value ?? "";

  /// And where a value came from, where it is still the machine's own: a
  /// prefill drawn over by typing is nobody's finding any more, and one
  /// Verkstead holds a value for is never sent in the first place.
  const from = (
    typed: string | null,
    held: string | undefined,
    offered: Offered,
  ): Offered => (typed === null && !held ? offered : null);

  /// What Verkstead has been told, where it has been told anything: the server
  /// sends an author it holds nothing for as empty strings.
  const held = (value: string | undefined) => value || undefined;

  const authorName = () =>
    standing(name(), held(told()?.git_author.name), found()?.name);
  const authorEmail = () =>
    standing(email(), held(told()?.git_author.email), found()?.email);

  /// The token as the field holds it. Nothing is *held* here: a token that is
  /// already configured is never sent back to a page, so what a field can show
  /// is what was typed or what the machine offered.
  const authorToken = () => standing(token(), undefined, found()?.token);

  /// Whether both halves of an author are there, which is what git asks for and
  /// what this step is judged by.
  const authored = () =>
    authorName().trim() !== "" && authorEmail().trim() !== "";

  /// What the save says about the token: the field's contents where it holds
  /// anything, and *leave it alone* where it does not — an empty write-only
  /// field means nothing was typed rather than take the credentials away.
  const tokenEdit = (): TokenEdit =>
    authorToken().trim() === "" ? "Keep" : { Set: { token: authorToken() } };

  /// The whole save: the three fields, and everything else in `config.yaml`
  /// exactly as it stands — one request writes the whole file, so a section
  /// left out would be a section emptied.
  ///
  /// The ignore rules go as `Keep`, which is the one thing a save can be
  /// refused over: this step has never drawn a rule, and a wizard turned down
  /// over a pattern somebody hand-edited into the file would be a wizard nobody
  /// could finish.
  const edit = (): SettingsEdit => ({
    git_author: { name: authorName().trim(), email: authorEmail().trim() },
    github_token: tokenEdit(),
    share_on_done: told()?.share_on_done ?? false,
    ignored_comments: "Keep",
    ...heldConfig(told()),
  });

  const save = useMutation(() => ({
    mutationFn: (written: SettingsEdit) => saveSettings(written),
    onSuccess: (answer: SettingsSaved) => {
      // Taken from the answer rather than read again: what a save comes back
      // with *is* a fresh read of the two files, so a second read would learn
      // nothing and could only disagree with what is on the page.
      queries.setQueryData(["settings"], answer.settings);
      setSaved(answer);

      // A save with no token to verify has nothing to be read, so the press
      // that made it is the press that finishes.
      if (answer.verified === null) {
        finish.mutate();
      }
    },
  }));

  const finish = useMutation(() => ({
    mutationFn: finishOnboarding,
    onSuccess: (reading) => {
      // Where the app is to land, put in place *before* the mode goes off. The
      // router that is up is the wizard's, whose every other path redirects
      // back here, and the one that will read this URL is the workbench's,
      // built the moment the reading below says the mode is off. So this is a
      // replace rather than a navigation — there is nothing yet to navigate
      // with, and the back button has no business walking into a wizard that is
      // over.
      window.history.replaceState({}, "", COMPOSE);

      // And the verdict itself, which is what says which app this is: handed
      // over rather than invalidated, because the press already has the answer
      // and a re-read would be the wizard standing for one more round trip.
      queries.setQueryData(["onboarding"], reading);
    },
  }));

  /// What Next does: save what is in the fields, or — where that has been
  /// done and there was an answer about the token to read — close the wizard.
  const onwards = () => {
    if (saved()) {
      finish.mutate();
      return;
    }

    save.mutate(edit());
  };

  /// Anything typed is a field the last save no longer speaks for.
  const typing = (write: (value: string) => void) => (value: string) => {
    setSaved(null);
    write(value);
  };

  /// The account the saved token authenticates as, and what GitHub says it may
  /// not do — neither of them until a token has been saved from here.
  const verified = () => saved()?.verified ?? null;

  const login = () => {
    const answer = verified();
    return answer && "Account" in answer ? answer.Account.login : null;
  };

  const missing = () => {
    const answer = verified();
    return answer && "Account" in answer ? answer.Account.missing : [];
  };

  const refused = () => {
    const answer = verified();
    return answer && "Refused" in answer ? answer.Refused.why : null;
  };

  const working = () => save.isPending || finish.isPending;

  return (
    <div class={styles.step}>
      <p class={styles.standing}>
        Every commit a session makes is by this author, and every pull request
        it opens is by this token. git needs both halves of the author; GitHub
        is a choice, and the token can be left empty.
      </p>

      {/* Fields in a plain block rather than in a `form`: the press is a
          button's the way the two steps before this one press theirs, and a
          form here would be a second way to make the same request — one of
          them saving and closing the wizard from a stray Enter. */}
      <div class={styles.form}>
        <section>
          <h3>Git author</h3>

          <Field
            id="setup-author-name"
            label="Name"
            placeholder="Ada Lovelace"
            value={authorName()}
            source={from(name(), held(told()?.git_author.name), found()?.name)}
            write={typing(setName)}
          />

          <Field
            id="setup-author-email"
            label="Email"
            type="email"
            placeholder="ada@example.com"
            value={authorEmail()}
            source={from(
              email(),
              held(told()?.git_author.email),
              found()?.email,
            )}
            write={typing(setEmail)}
          />
        </section>

        <section>
          <h3>GitHub token</h3>

          <Note>
            Every session is handed it, and Verkstead reads pull requests and
            reviews with it. Leave it empty if GitHub is not in use here.
          </Note>

          {/* A token already configured is never sent back to a page — see the
              settings pane, where the same rule is why the field there is
              write-only. So what is said about one is its last four characters
              and when it was written, and the box under it replaces it. */}
          <Show when={told()?.github_token}>
            {(configured) => (
              <p class={styles.source}>
                A token ending <code>{configured().last_four}</code> is already
                saved, from {utcStamp(configured().at)}. Typing here replaces
                it.
              </p>
            )}
          </Show>

          <Field
            id="setup-github-token"
            label="Token, pasted — it is stored and never shown again"
            type="password"
            placeholder="ghp_…"
            value={authorToken()}
            source={from(token(), undefined, found()?.token)}
            write={typing(setToken)}
          />

          {/* What GitHub made of it, which is the whole reason a token stops
              this step for one more press: an account that is not the one the
              human meant is the mistake nothing else would catch. */}
          <Show when={login()}>
            {(who) => (
              <p class={styles.verified}>
                GitHub says it is <span class={styles.login}>{who()}</span>.
              </p>
            )}
          </Show>

          <Show when={missing().length > 0}>
            <p class={styles.unscoped}>
              It cannot publish a share: GitHub has not given it the{" "}
              <code class={styles.scope}>{missing().join(", ")}</code> scope.
              That is the settings page's to put right later; it holds nothing
              up here.
            </p>
          </Show>

          <Show when={refused()}>
            {(why) => (
              <ErrorLine class={styles.unverified}>
                It is saved, but GitHub would not say whose it is: {why()}.
                Correct it above, or press Next again to go on without it
                working.
              </ErrorLine>
            )}
          </Show>
        </section>

        <div class={styles.onwards}>
          <button
            type="button"
            class={styles.next}
            disabled={!authored() || working()}
            onClick={onwards}
          >
            Next
          </button>

          <Show when={!authored()}>
            <Note>
              git refuses a commit with no author. Both halves are needed.
            </Note>
          </Show>

          <Show when={save.isError}>
            <ErrorLine class={styles.failure}>
              Nothing was saved: {save.error?.message}
            </ErrorLine>
          </Show>
          <Show when={finish.isError}>
            <ErrorLine class={styles.failure}>
              It is saved, but the wizard could not be closed:{" "}
              {finish.error?.message}
            </ErrorLine>
          </Show>
        </div>
      </div>
    </div>
  );
}

/// One field: its label, its box, and — under a value nobody typed — where it
/// was found.
function Field(props: {
  id: string;
  label: string;
  type?: string;
  placeholder: string;
  value: string;
  source: Offered;
  write: (value: string) => void;
}): JSX.Element {
  return (
    <div class={styles.field}>
      <label for={props.id}>{props.label}</label>
      <input
        id={props.id}
        type={props.type ?? "text"}
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck={false}
        placeholder={props.placeholder}
        value={props.value}
        onInput={(ev) => props.write(ev.currentTarget.value)}
      />
      <Show when={props.source}>
        {(found) => (
          <p class={styles.source} data-source={found().source}>
            {SOURCES[found().source]()}
          </p>
        )}
      </Show>
    </div>
  );
}
