//! Everything git is told, on one pane: the GitHub token every session is
//! handed, who its commits are by, how a conflicted pull request is resolved,
//! and whether Done shares the record to that pull request.
//!
//! **Git**, at `/settings/git`. It was *GitHub and git author* at
//! `/settings/github`, with the resolution a section of its own at
//! `/settings/conflicts`; both slugs are no such page now rather than
//! redirects. One pane, because all four are about the one remote and the one
//! branch — and a machine is set up by reading them in a row.
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
//! is missing — and pressing it opens the form in the details pane beside it.
//! The modal it was is gone: a form that stood over the page is a pane of its
//! own now, and the card reads as open while that pane is, which is what every
//! other card in this app says about itself.
//!
//! **Every Author, one at a time.** The token and the Git Author's fields are
//! about one Author, and a dropdown at the form's head picks which, with **Add
//! author** at its foot behind a rule the way the Repo dropdown carries Open
//! repo and Create repo. Drawn only once there are two to choose between, and
//! the display name with it, it being read nowhere but in those rows: with one
//! Author the pane reads as it did before there could be two, and the way to a
//! second is a quiet press under the first one's fields. Switching drops what
//! was typed and not saved, which was about the Author being left. An Author
//! being added is a uuid minted here, saved like any other — the server makes
//! a new Author of a uuid it does not hold. Removing one is not here yet.
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
//! Above it is the one thing this pane explains: what a token has to be able to
//! do. The words are [`TokenScopes`](./scopes.tsx)'s, drawn here and on the
//! wizard's git step, and they stand whether or not a token has ever been saved
//! — where the lines under them are about the token that *was* saved and only
//! appear after a save. It is here rather than in `CONTEXT.md` because the
//! moment it is useful is the moment somebody is on GitHub's own form, and the
//! failure it prevents is a session's push, which comes back naming no scope.
//!
//! One button saves both files, because the server writes both in one request:
//! the author fields are values and the token is an action, so correcting an
//! email address leaves the credentials alone. Clearing is its own press for the
//! same reason — an empty write-only field means nothing was typed.
//!
//! Two controls in the pane save themselves rather than waiting for that Save:
//! the select that says how a conflicted pull request is resolved, and the
//! checkbox that says whether a Conversation's record is published and linked
//! on its pull request when the work settles to Done. A pick and a tick are
//! each their own press — a choice that needed confirming afterwards is a
//! choice the human has to make twice.
//!
//! The sharing checkbox is here rather than in a section of its own because
//! what it turns on is done with the token above it and under the account that
//! token belongs to: a control two panes away from the credential it spends
//! would be one nobody could see the cost of. The select is here because a
//! conflict is a thing that happens to a branch on a remote, which is the
//! whole of what this pane is about.
//!
//! Both carry the author as the *server* holds it rather than as the fields
//! hold it — see [`beside`]: neither is a submit, and half a typed email
//! address is nobody's author. Neither spends the form either, so what was
//! being typed is still being typed afterwards.
//!
//! And a last section under the Save: the comments nobody wants an agent
//! addressing, as a row of two patterns apiece. They are here because this is
//! the pane about what Verkstead does on somebody else's GitHub. Each box is a
//! regular expression matched anywhere in the text, a rule's boxes must all
//! match, and an empty one constrains nothing — see **Ignore rule** in
//! `CONTEXT.md`, which is where that is written down now: nothing on this pane
//! explains a control it stands under, the scopes above being about the value
//! rather than about the field. The one note is what becomes of a pasted token,
//! said beside its field rather than in the label, as the copy guide has it.
//!
//! The rules travel as an action rather than a value for a reason of their own
//! — a save is *refused* over a pattern that will not compile — so the rows are
//! sent only once somebody has touched one, and a save about an email address
//! is one the server cannot turn down.
//!
//! **A row nobody wrote anything in is not a rule**, and comes off the page as
//! the save goes out. That is how a rule is deleted — clearing both boxes says
//! as much as pressing Remove does — and it is what keeps a row somebody added
//! and never filled in from turning down the save it rode along with. The
//! server refuses a rule that constrains nothing, and this is what means the
//! page never asks it to.
//!
//! A refusal is the whole request refused: neither file is written, and what
//! comes back names the row and the box it is about. So it is drawn at that
//! row, with everything the human typed left exactly where they left it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Index, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { Check } from "../Check";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import { useReading } from "../freshness";
import { Listbox, Picker } from "../picking";
import type {
  Author,
  AuthorSaved,
  ConflictResolution,
  IgnoreRule,
  IgnoredCommentsEdit,
  RuleField,
  RuleRefused,
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
import { heldAuthor, heldConfig } from "./held";
import { TokenScopes } from "./scopes";
import styles from "./Git.module.css";
import { SettingsCard } from "./SettingsCard";

/// What the section is called, wherever it names itself: the card's heading and
/// the pane's head.
const TITLE = "Git";

/// The two files as they stand, read once for the two panes that draw them.
///
/// Merged rather than frozen, because this is not a payload that cannot change:
/// another device may write the same two files, and a frozen query is one the
/// catch-up read on reconnect could never reach. There is no list in it for the
/// key to match by — what the merge does here is leave the fields that did not
/// change alone, and whatever is drawn from them with them.
function useGit() {
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
///
/// Handed an accessor rather than the value, because the pane's is a different
/// Author's token the moment its dropdown is switched, and a `Show` that stayed
/// shown would otherwise keep saying the first one's.
function standing(saved: () => TokenSaved): JSX.Element {
  return (
    <p class={styles.tokenStanding}>
      A token ending <code class={styles.lastFour}>{saved().last_four}</code>,
      saved <span>{utcStamp(saved().at)}</span>.
    </p>
  );
}

/// Half an author is as broken as none: git complains by name about whichever
/// half it has not been given.
function authorless(author: Author | AuthorSaved): boolean {
  return author.name === "" || author.email === "";
}

/// What an Author is called in the dropdown's rows and on the card: the display
/// name somebody gave it, or its git name where nobody did.
///
/// And something for an Author that has neither yet, which is the one **Add
/// author** has just opened — a row reading as nothing would be a row nobody
/// could find again.
export function authorLabel(author: AuthorSaved): string {
  return author.display_name || author.name || "New author";
}

/// What one Author is missing, in the card's words — or `null` where it has
/// everything a session needs.
function lacking(tokenless: boolean, author: boolean): string | null {
  if (tokenless && author) return "No GitHub token or git author";
  if (tokenless) return "No GitHub token";
  if (author) return "No git author";

  return null;
}

/// Whether the card is drawn in the warning colour: an Author missing either
/// half, or no Author at all.
function warning(told: SettingsView): boolean {
  return told.authors.length > 1
    ? told.authors.some(
        (author) => author.github_token === null || authorless(author),
      )
    : told.github_token === null || authorless(told.git_author);
}

/// What the card says, in a line: who commits and which token is saved — or,
/// where either is missing, that it is, which is the line in the warning colour.
///
/// With several Authors the warning names the Author it is about, the first
/// that is missing anything, and the line that has nothing to warn about names
/// every one of them. With one it is exactly what it was before there could be
/// two: the one Author's name and token.
function summary(told: SettingsView): string {
  if (told.authors.length > 1) {
    for (const author of told.authors) {
      const missing = lacking(author.github_token === null, authorless(author));

      if (missing) return `${missing} for ${authorLabel(author)}`;
    }

    return told.authors.map(authorLabel).join(", ");
  }

  return (
    lacking(told.github_token === null, authorless(told.git_author)) ??
    `${told.git_author.name} · token …${told.github_token!.last_four}`
  );
}

/// What git has been told, as the card that opens it.
///
/// A card rather than a section with an Edit button on its heading, because that
/// is what the rest of this pane is: something standing in a pane that is
/// selected and opened into the pane beside it. Drawn as an `article`, the way
/// every card holding more than a run of text is — a button may not have
/// paragraphs inside it, and `CardButton` puts the press, the keyboard and the
/// role that says what it is on the article instead.
export function GitCard(props: {
  /// Whether the form beside this is the pane that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useGit();

  return (
    <Switch>
      <Match when={settings.isPending}>
        <Empty>Reading the settings…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          The settings couldn't be read. {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data}>
        {(told) => (
          <SettingsCard
            class={styles.gitCard}
            title={TITLE}
            open={props.open}
            press={props.press}
            warn={warning(told())}
            status={summary(told())}
          />
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
export function GitPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useGit();

  // Which Author the fields are about, by uuid, or `null` for the first —
  // which is the only one there is, until somebody adds a second.
  const [chosen, setChosen] = createSignal<string | null>(null);

  // And the uuid **Add author** minted, while the Author it opened has not been
  // saved: it is a row of the dropdown until then, and an Author in the file
  // from the save that names it on.
  const [adding, setAdding] = createSignal<string | null>(null);

  // What has been typed, or `null` while nothing has — the fields follow the
  // server until somebody touches them, the way a branch name does.
  const [displayName, setDisplayName] = createSignal<string | null>(null);
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
  //
  // Kept with the uuid of the Author it was about, so that switching the
  // dropdown does not say one Author's account under another's fields.
  const [verifiedAs, setVerifiedAs] = createSignal<{
    id: string | null;
    verified: Verified;
  } | null>(null);

  // The ignore rules as they are being edited, or `null` while nobody has
  // touched a row — the rows follow the server until then, the way the author
  // fields do. `null` is also what says the save has nothing to say about
  // them, which is what keeps a save about the author or the token one that
  // cannot be refused: see [`IgnoredCommentsEdit`].
  const [rules, setRules] = createSignal<IgnoreRule[] | null>(null);

  // And what the last save was turned down over, by the row and the box it
  // named. Emptied as the next save goes out, because it describes what was
  // sent rather than what is on the page.
  const [refusals, setRefusals] = createSignal<RuleRefused[]>([]);

  const told = (): SettingsView | undefined => settings.data;

  /// Every Author there is to pick between: the ones the files hold, and the
  /// one **Add author** opened on the end while it is still unsaved.
  const authors = (): AuthorSaved[] => {
    const saved = told()?.authors ?? [];
    const minted = adding();

    return minted === null || saved.some((one) => one.id === minted)
      ? saved
      : [
          ...saved,
          {
            id: minted,
            display_name: "",
            name: "",
            email: "",
            github_token: null,
          },
        ];
  };

  /// Whether there are two to choose between, which is what draws the
  /// dropdown — and the display name, which is only ever read in its rows.
  const several = () => authors().length > 1;

  /// The Author the fields are about: the one picked, or the first where the
  /// pick is gone or was never made.
  ///
  /// With one, the first Author as the read's own single fields have it,
  /// which is what the card is drawn from too — the pane reads exactly as it
  /// did before there could be two. Its uuid is empty on a Verkstead with no
  /// Author yet, which a save sends as none.
  const author = (): AuthorSaved | undefined => {
    if (several()) {
      return authors().find((one) => one.id === chosen()) ?? authors()[0];
    }

    const standing = told();

    return (
      standing && {
        ...standing.git_author,
        id: standing.git_author.id ?? "",
        github_token: standing.github_token,
      }
    );
  };

  /// Its uuid as a save names it: none, where there is no Author yet.
  const authorId = (): string | null => author()?.id || null;

  /// Whether that is the Author **Add author** opened, still unsaved.
  const drafting = () => author() !== undefined && author()!.id === adding();

  const configured = () => author()?.github_token ?? null;

  const authorDisplayName = () => displayName() ?? author()?.display_name ?? "";
  const authorName = () => name() ?? author()?.name ?? "";
  const authorEmail = () => email() ?? author()?.email ?? "";

  /// What GitHub said about the token last saved from here, where it was saved
  /// under the Author the fields are about now.
  const verified = (): Verified | null => {
    const what = verifiedAs();
    return what && what.id === authorId() ? what.verified : null;
  };

  /// Everything in `config.yaml` this form is not about, as it stands — see
  /// [`heldConfig`], which the wizard's git step sends the same way. The
  /// resolution is among it: the select below saves itself, so what a press of
  /// Save says about it is what the server already holds.
  const held = () => heldConfig(told());

  /// Where the share-on-Done checkbox sits, which is off until somebody has
  /// been here.
  const sharing = () => told()?.share_on_done ?? false;

  /// And where the conflict-resolution select sits, which is one of two words
  /// and never absent: a merge is what a Verkstead nobody has configured does.
  const resolution = (): ConflictResolution =>
    told()?.conflict_resolution ?? "Merge";

  /// What a control that saves itself sends: both files as the server holds
  /// them, with the two values those controls own written over the top.
  ///
  /// The author goes back as the *server* holds it rather than as the fields
  /// do, and the token and the rules travel as `Keep`. Neither control is a
  /// submit: a pick made halfway through typing an address writes the pick and
  /// nothing else, and half a typed email address is nobody's author.
  ///
  /// Both values on every call, each control passing its own and leaving the
  /// other where the server has it — one place that knows what a save beside
  /// the form carries, rather than one per control to fall out of step.
  const beside = (owned: {
    share_on_done: boolean;
    conflict_resolution: ConflictResolution;
  }): SettingsEdit => ({
    git_author: heldAuthor(told()),
    // Untouched, for the reason a blank field is a `Keep`.
    github_token: "Keep",
    // And the rules left exactly where they are: neither control is the form's
    // save, and one that spoke for them could be turned down over a pattern it
    // never showed anybody — see [`ruleEdit`].
    ignored_comments: "Keep",
    // And the declared MCP servers, for the reason beside it: they are the other
    // setting that travels as an action, and this form has never drawn one.
    mcp_servers: "Keep",
    ...held(),
    ...owned,
  });

  const typing = () => replacing() || configured() === null;

  /// The rows as they are drawn: what has been typed, or what is written down
  /// while nothing has been.
  const rows = (): IgnoreRule[] => rules() ?? told()?.ignored_comments ?? [];

  /// One box rewritten, which is what makes the next save a `Set` of the whole
  /// list — the server writes the rules as one list, so a row edited is the
  /// list sent.
  const rewrite = (at: number, part: Partial<IgnoreRule>) =>
    setRules(
      rows().map((rule, which) => (which === at ? { ...rule, ...part } : rule)),
    );

  /// A row added, empty, for the human to write a rule into. Its errors are
  /// dropped with it and with every other row's: a refusal names a row by where
  /// it stood, so a list that has gained or lost one is a list they no longer
  /// point into. Typing in a box leaves them, because the rows are still the
  /// rows the server was talking about.
  const add = () => {
    setRefusals([]);
    setRules([...rows(), { author: "", body: "" }]);
  };

  const remove = (at: number) => {
    setRefusals([]);
    setRules(rows().filter((_, which) => which !== at));
  };

  /// Why the row standing at `at` was turned down, against one of its boxes or
  /// against the rule itself — `null` where the save was not refused over it.
  const trouble = (at: number, field: RuleField | null): string | null =>
    refusals().find((was) => was.rule === at && was.field === field)?.why ??
    null;

  /// Whether a row is a rule at all, which is whether anything was written in
  /// either of its boxes. Emptiness is decided the way the server decides it —
  /// by what is left after the spaces — so the two halves agree about which
  /// rows are rules.
  const written = (rule: IgnoreRule): boolean =>
    rule.author.trim() !== "" || rule.body.trim() !== "";

  /// The rows nobody has written anything in, taken off the page as the save
  /// goes out — which is how a rule is deleted, and what stops a row the human
  /// added and never filled in refusing the save it rode along with.
  ///
  /// Taken off the page rather than merely left out of the request, so that
  /// what is drawn and what was sent are one list: a refusal names a rule by
  /// where it stood in what was sent, and a request that quietly left rows out
  /// would have those numbers pointing at the wrong boxes.
  const tidied = () => {
    const edited = rules();

    if (edited !== null) {
      setRules(edited.filter(written));
    }
  };

  /// What the save says about the rules: nothing at all until somebody has
  /// touched a row, and the whole list in the order it is drawn once they have
  /// — the blank rows having been taken off it by [`tidied`] first.
  const ruleEdit = (): IgnoredCommentsEdit => {
    const edited = rules();

    return edited === null ? "Keep" : { Set: { rules: edited } };
  };

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
    unedited();

    // And the rows go back to following the server, which has just been told
    // what they said: what is drawn from here on is what was written down.
    setRules(null);
    setRefusals([]);
  };

  /// The Author fields back to following the server, and the token field
  /// shut and empty — what a save leaves, and what a switch of the dropdown
  /// does, so that nothing typed about one Author is sent as another's.
  const unedited = () => {
    setDisplayName(null);
    setName(null);
    setEmail(null);
    setToken("");
    setReplacing(false);
  };

  /// Make the fields about another Author. Whatever was typed and not saved
  /// stays with the Author it was typed about, which is to say it goes: it
  /// was never written down. And an Author **Add author** opened and nobody
  /// saved goes with it, there being nothing of it anywhere but this page.
  const pick = (id: string) => {
    if (id === author()?.id) return;

    unedited();
    if (id !== adding()) setAdding(null);
    setChosen(id);
  };

  /// **Add author**: the same fields over a uuid minted here, which the save
  /// that names it makes an Author of.
  const addAuthor = () => {
    const id = mintedUuid();

    unedited();
    setAdding(id);
    setChosen(id);
  };

  const save = useMutation(() => ({
    mutationFn: (edit: SettingsEdit) => saveSettings(edit),
    onSuccess: (saved: SettingsSaved) => {
      // A refusal is the whole save refused — neither file was touched — so
      // nothing here is spent and nothing is read back: what the human typed
      // stays in front of them, with the errors drawn at the rows they name.
      if (saved.refused.length > 0) {
        setRefusals(saved.refused);
        return;
      }

      setVerifiedAs(
        saved.verified === null
          ? null
          : {
              // A save about no Author yet made the first one, so the answer
              // is about whichever uuid the server minted for it.
              id: sentAbout() ?? saved.settings.git_author.id,
              verified: saved.verified,
            },
      );
      spent();

      // An Author that was being added is one now, where the save named it:
      // the read below carries it, so it is no longer this page's to draw.
      if (saved.settings.authors.some((one) => one.id === adding())) {
        setAdding(null);
      }

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
  const write = (github_token: TokenEdit) => {
    // Whatever the last save was turned down over went with the press that is
    // being made now: the answer to this one says what is wrong with what is
    // being sent this time.
    setRefusals([]);

    // And a row nobody wrote anything in is not a rule, so it goes before the
    // list is read off the page — see [`tidied`]. Which is what makes clearing
    // a row a way to delete it, and what keeps a blank row somebody added and
    // left from turning down a save about their own email address.
    tidied();

    const about = editing(github_token);
    setSentAbout(about.id);

    save.mutate({
      git_author: about,
      github_token: drafting() && !said(github_token) ? "Keep" : github_token,
      share_on_done: sharing(),
      // And the ignore rules, which this pane is the one that owns: the whole
      // list where somebody has touched a row, and `Keep` where nobody has —
      // see [`ruleEdit`]. That is what stops a corrected email address being
      // refused over a pattern somebody hand-edited into the file weeks ago.
      ignored_comments: ruleEdit(),
      // And the declared servers left where they are: this form has nothing to
      // say about them, and one that spoke for them could be turned down over a
      // name somebody hand-edited into the file weeks ago.
      mcp_servers: "Keep",
      ...held(),
    });
  };

  /// Whether a save says anything about the Author **Add author** opened: a
  /// field typed in, or a token. One that says nothing writes no Author — a
  /// row nobody filled in is no more an Author than it is a rule.
  const said = (github_token: TokenEdit): boolean =>
    [authorDisplayName(), authorName(), authorEmail()].some(
      (field) => field.trim() !== "",
    ) || github_token !== "Keep";

  /// The Author a press of Save is about: the one the fields were read from,
  /// with what they hold now, by its uuid — which is what says which Author the
  /// save lands on, and makes a new one of a uuid the file does not hold.
  ///
  /// The first Author as it stands, where the fields are about an Author that
  /// was added and never filled in: the rest of the form is still saved, and
  /// nothing is made out of nothing.
  const editing = (github_token: TokenEdit): Author => {
    if (drafting() && !said(github_token)) return heldAuthor(told());

    return {
      id: authorId(),
      display_name: authorDisplayName(),
      name: authorName(),
      email: authorEmail(),
    };
  };

  /// And the uuid the last save went out about, for the answer to be kept
  /// against — see [`verified`].
  const [sentAbout, setSentAbout] = createSignal<string | null>(null);

  /// The checkbox's own save, which is its own press — see [`beside`] for what
  /// rides along with it.
  const flipping = useMutation(() => ({
    mutationFn: (share_on_done: boolean) =>
      saveSettings(beside({ share_on_done, conflict_resolution: resolution() })),
    onSuccess: (saved: SettingsSaved) =>
      queries.setQueryData(["settings"], saved.settings),
  }));

  /// And the select's, which is the same shape: picking is the save.
  ///
  /// A mutation of its own rather than the checkbox's, so that a save one of
  /// them could not make is said under the control that made it.
  const picking = useMutation(() => ({
    mutationFn: (conflict_resolution: ConflictResolution) =>
      saveSettings(beside({ share_on_done: sharing(), conflict_resolution })),
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
          title={TITLE}
        />
      </PaneSticky>

      <Switch>
        <Match when={settings.isPending}>
          <Empty>Reading the settings…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            The settings couldn't be read. {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={told()}>
          <form class={styles.form} onSubmit={submit}>
            {/* Which Author everything down to the Git Author's fields is
                about — drawn only once there are two to choose between, so
                that a Verkstead with one Author reads as it did before there
                could be two. **Add author** is at its foot, behind the rule,
                the way the Repo dropdown carries Open repo and Create repo. */}
            <Show when={several()}>
              <section>
                <h3>
                  <label for="git-author-pick">Author</label>
                </h3>

                <Listbox
                  id="git-author-pick"
                  options={authors()}
                  value={(one) => one.id}
                  label={authorLabel}
                  chosen={author()?.id ?? ""}
                  pick={pick}
                  actions={[{ label: "Add author", press: addAuthor }]}
                />
              </section>
            </Show>

            <section>
              <h3>GitHub Token</h3>

              {/* What a token has to be able to do, above everything about the
                  one that is saved: this is the list to tick against before
                  pasting one, and it stands whether or not anything has ever
                  been saved here. */}
              <TokenScopes class={styles.tokenScopes} />

              <Show when={configured()}>{(saved) => standing(saved)}</Show>

              {/* What GitHub said about the last token saved from here: the
                  account it authenticates as, or why nobody could be asked.
                  Both are worth showing — a token saved against the wrong
                  account is the failure this form exists to catch. */}
              <Show when={account()}>
                {(login) => (
                  <p class={styles.verified}>
                    GitHub says it's <span class={styles.login}>{login()}</span>
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
                  It can't publish a share without the{" "}
                  <code class={styles.scope}>{missing().join(", ")}</code>{" "}
                  scope. Re-issue the token on GitHub with it ticked, and save
                  it here again.
                </p>
              </Show>
              <Show when={refused()}>

                {(why) => (
                  <ErrorLine class={styles.unverified}>
                    It's saved, but GitHub wouldn't say whose it is. {why()}
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
                        setVerifiedAs(null);
                        write("Clear");
                      }}
                    >
                      Clear
                    </button>
                  </div>
                }
              >
                <label for="github-token">Token</label>
                <Note class={styles.stored}>Stored, and never shown again.</Note>
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
                      class={`${styles.keep} secondary`}
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
              <h3>Git Author</h3>

              {/* What the dropdown's rows call this Author, which is the
                  only place it is read — so drawn only where they are. */}
              <Show when={several()}>
                <label for="author-display-name">Display name</label>
                <input
                  id="author-display-name"
                  type="text"
                  autocapitalize="words"
                  autocorrect="off"
                  spellcheck={false}
                  placeholder={authorName() || "Work"}
                  value={authorDisplayName()}
                  onInput={(ev) => setDisplayName(ev.currentTarget.value)}
                />
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

              {/* With one Author, the way to a second: a quiet press under
                  its fields rather than a dropdown with one row in it. Once
                  there are two, it is the dropdown's own last row instead. */}
              <Show when={!several() && authorId() !== null}>
                <QuietButton class={styles.addAuthor} onClick={addAuthor}>
                  Add author
                </QuietButton>
              </Show>
            </section>

            <section>
              {/* The subheading is the select's label rather than standing over
                  one: what a second line under it could say is the heading's
                  own words again. */}
              <h3>
                <label for="conflict-resolution">Conflict Resolution</label>
              </h3>

              <Picker
                id="conflict-resolution"
                options={["Merge", "Rebase"] satisfies ConflictResolution[]}
                value={(resolution) => resolution}
                label={(resolution) => resolution}
                chosen={resolution()}
                disabled={picking.isPending}
                pick={(picked) => picking.mutate(picked as ConflictResolution)}
              />

              <Show when={picking.isError}>
                <ErrorLine class={styles.failure}>
                  The settings couldn't be saved. {picking.error?.message}
                </ErrorLine>
              </Show>
            </section>

            <section>
              <h3>Sharing</h3>

              {/* In a wrapper of its own, so the form's own label rule — which
                  is about the labels over the fields — leaves the checkbox's to
                  `Check.module.css`. */}
              <div class={styles.sharing}>
                <Check
                  label="Share to pull request on Done"
                  on={sharing()}
                  disabled={flipping.isPending}
                  flip={(on) => flipping.mutate(on)}
                />
              </div>

              <Show when={flipping.isError}>
                <ErrorLine class={styles.failure}>
                  The settings couldn't be saved. {flipping.error?.message}
                </ErrorLine>
              </Show>
            </section>

            <section>
              <h3>Ignored Comments</h3>

              <Show
                when={rows().length > 0}
                fallback={
                  <Empty>
                    No comments are ignored. Add a rule with{" "}
                    <strong>Add a rule</strong>.
                  </Empty>
                }
              >
                <ul class={styles.rules}>
                  <Index each={rows()}>
                    {(rule, at) => (
                      <li class={styles.rule}>
                        <label for={`ignored-author-${at}`}>Author</label>
                        <input
                          id={`ignored-author-${at}`}
                          type="text"
                          autocapitalize="off"
                          autocorrect="off"
                          spellcheck={false}
                          placeholder="coderabbitai"
                          value={rule().author}
                          onInput={(ev) =>
                            rewrite(at, { author: ev.currentTarget.value })
                          }
                        />
                        <Show when={trouble(at, "Author")}>
                          {(why) => (
                            <ErrorLine class={styles.trouble}>
                              {why()}
                            </ErrorLine>
                          )}
                        </Show>

                        <label for={`ignored-body-${at}`}>Body</label>
                        <input
                          id={`ignored-body-${at}`}
                          type="text"
                          autocapitalize="off"
                          autocorrect="off"
                          spellcheck={false}
                          placeholder="billing"
                          value={rule().body}
                          onInput={(ev) =>
                            rewrite(at, { body: ev.currentTarget.value })
                          }
                        />
                        <Show when={trouble(at, "Body")}>
                          {(why) => (
                            <ErrorLine class={styles.trouble}>
                              {why()}
                            </ErrorLine>
                          )}
                        </Show>

                        {/* And what is wrong with the rule rather than with
                            either box, which is the one refusal there is no
                            field to draw at. */}
                        <Show when={trouble(at, null)}>
                          {(why) => (
                            <ErrorLine class={styles.trouble}>
                              {why()}
                            </ErrorLine>
                          )}
                        </Show>

                        <QuietButton
                          class={styles.remove}
                          onClick={() => remove(at)}
                        >
                          Remove
                        </QuietButton>
                      </li>
                    )}
                  </Index>
                </ul>
              </Show>

              <QuietButton class={styles.add} onClick={add}>
                Add a rule
              </QuietButton>
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
                The settings couldn't be saved. {save.error?.message}
              </ErrorLine>
            </Show>
          </form>
        </Match>
      </Switch>
    </>
  );
}

/// A fresh version-4 uuid for an Author **Add author** opens, which is what
/// the save names it by and the server keeps it under.
///
/// Out of `getRandomValues` rather than `randomUUID`, which a browser gives
/// only to a secure context — and the workbench is reached over plain HTTP on
/// a LAN as often as on `localhost`.
function mintedUuid(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));

  bytes[6] = (bytes[6]! & 0x0f) | 0x40;
  bytes[8] = (bytes[8]! & 0x3f) | 0x80;

  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"));

  return [
    hex.slice(0, 4),
    hex.slice(4, 6),
    hex.slice(6, 8),
    hex.slice(8, 10),
    hex.slice(10, 16),
  ]
    .map((part) => part.join(""))
    .join("-");
}
