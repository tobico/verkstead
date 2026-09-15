//! The Repos Verkstead has been told about: the card on the settings page
//! saying how many are registered, the pane it opens listing every one of them
//! by name with a Remove beside it, and the registration that adds another —
//! an absolute path, typed or browsed to.
//!
//! **That registration has left the settings.** There was a plus on this
//! section's heading and a pane behind it holding the form; what is left is the
//! form itself, drawn in the one place somebody meets it — the **Open repo**
//! and **Create repo** cards at the foot of the Repo dropdown, on the page
//! where a Conversation is composed and there is nowhere yet to do the work.
//! The settings are where a machine is read rather than where work begins.
//!
//! **And a Repo no longer has a pane of its own.** Its card opened one: the
//! path and the default branch, every branch git had, how much work was on it,
//! the roadmaps in it nothing was driving, a conflict picker, its own binds,
//! and Remove at the foot. What is drawn of a Repo here now is its name. None
//! of those facts was stored — each was a git call or a count, made every time
//! somebody opened one — and each is read where it is used instead: the
//! branches on the composer, the roadmaps waiting in the new conversation
//! dropdown, and how a conflicted pull request is resolved once on the Git
//! pane, for every Repo.
//!
//! So the one thing there is to *do* to a Repo is take it off the registry,
//! which is an unregistering rather than a delete: Verkstead stops offering the
//! repository, the directory is left where it is, and every Conversation ever
//! worked in it goes on saying so. **It is asked about once before it happens**
//! — the way a close over a run in flight is — because it is one press in a
//! list on a phone now rather than the last press on a pane about the
//! repository. Refused while live work is on it, in words, under the row the
//! press was made on: that row is what says which Repo the refusal is about.
//!
//! The form is one piece ([`RepoRegistration`]) drawn twice rather than a
//! second form with a second set of words for the same refusals. Both places
//! that draw it are ways *onto* a repository now, so a path that is registered
//! already is the repository somebody named rather than a dead end — which is
//! what the pane that has gone answered differently.
//!
//! Beside that modal is the other way a Repo arrives, which is this file's too:
//! [`CreateRepo`], where a parent and a name *make* one. Not the same form —
//! a repository that is not there yet has nowhere to be browsed to, so the
//! question is two fields rather than one — but the same card, the same words
//! for the refusals, and the same rule about what a refusal does to the card
//! it was made on. Two fields and a tick: where a GitHub token is saved, the
//! same repository is made there too, and where none is the card says a remote
//! is needed before the work on it can be finished.
//!
//! The path is written into the shared path field — see `PathField.tsx` — which
//! is the box it always was with a dropdown under it that browses the filesystem
//! a directory at a time. What the form is for has not moved: it names a path,
//! and what the server does about that path is still the only thing that decides
//! whether it is taken. The browse reaches anywhere the server can read, this
//! form being bounded by nothing else either; and it marks a repository where it
//! finds one and stops there, that being the one thing this field is looking for
//! and the one place in the workbench a `.git` means anything.
//!
//! Every refusal is shown as a refusal, in words, beside the field the path was
//! typed into: the human has just done something and is owed a reason it did not
//! happen. Which reason it was comes from the server as a named outcome, and
//! this file is where each of them is said — which is why the card is what a
//! refusal keeps up rather than something the answer replaces.
//!
//! The card and the pane read the one query, as every other section of the
//! settings page reads one: they are two views of the same list, and a read
//! apiece would be two reads of it — the cache is what makes the second caller
//! free.
//!
//! A section of the settings page rather than a page of its own: which
//! repositories Verkstead may touch is settled once and then left alone, which
//! is the same kind of thing as everything else on it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";

import { CardButton } from "../CardButton";
import { Modal } from "../Modal";
import { PaneSticky } from "../Panes";
import { PathField } from "../PathField";
import {
  createRepo,
  listRepos,
  registerRepo,
  removeRepo,
} from "../api/client";
import type {
  Created,
  Registered,
  RepoEntry,
  RepoRemoved,
  RepoView,
} from "../api/types";
import { repoParent, setRepoParent } from "../device";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { useSettings } from "../settings/PathEditor";
import { PaneHead } from "../workbench/PaneHead";
import styles from "./RepoList.module.css";

/// The ways a registration is answered with a sentence rather than a Repo.
///
/// Every outcome that left no Repo registered, which is every one that is a bare
/// word on the wire. What is never here is `Added` — nothing is said about a
/// registration that worked — and neither is `AlreadyRegistered`, which carries
/// a Repo: both places this form is drawn are ways onto a repository, so a path
/// registered already is the repository somebody named.
export type RepoRefused = Extract<Registered, string>;

/// What each way of being refused says, once, wherever it is met.
export const REFUSAL: Record<RepoRefused, string> = {
  NotAbsolute: "Give the repo's absolute path, starting with a slash.",
  Missing: "There is nothing at that path.",
  NotARepository:
    "That is not a git repository — name the repository's own directory.",
  NoDefaultBranch:
    "That repository has no branch to call its default. Check one out first.",
};

/// The ways a create is answered with a sentence rather than a Repo, in the same
/// shape and for the same reason: every outcome that is a bare word on the wire.
///
/// `Refused` is not among them — it carries git's own account of what went
/// wrong, which is a sentence the server wrote rather than one this file has —
/// and neither is `Made`, a create that worked being said by the Repo the draft
/// lands on.
export type CreateRefused = Extract<Created, string>;

/// And what each of those says.
export const CREATE_REFUSAL: Record<CreateRefused, string> = {
  ParentMissing: "There is no directory at that path to make it in.",
  AlreadyThere: "Something of that name is in that directory already.",
  BadName: "That is not a name a directory can have.",
  NoAuthor:
    "Nobody is configured to commit as. Settings has a Git author, and the first commit needs one.",
};

/// And what each way of being refused a removal says.
///
/// `Removed` is here for completeness of the mapping and never drawn: the row
/// leaving the list is what says the removal landed.
export const REPO_REMOVAL_REFUSAL: Record<RepoRemoved, string> = {
  Removed: "",
  NoSuchRepo: "That repo is off the registry already.",
  InUse:
    "A conversation that is still going is on it. Finish or close that conversation first.",
};

/// The Repos as they stand, read once for the card and the pane that draw them.
///
/// Read when the page opens, like the Profiles above them: nothing here changes
/// on its own, and what does change is this section's own doing.
///
/// Merged by the id each entry carries flat, and not frozen: registering one
/// reads the list again, and a frozen query is one invalidation cannot reach —
/// the new repo would never appear behind the card that added it.
///
/// Exported because the Paths pane asks it too — see `settings/Paths.tsx`,
/// which needs the registered names to tell a bind written for a Repo from one
/// written for a name nothing is registered under. One query definition rather
/// than a second saying the same thing: the two would share a key and have to
/// agree about freshness anyway.
export function useRepos() {
  return useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,
    freshness: { reconcile: "id" },
  }));
}

/// How many Repos are registered, as the card that opens the list of them.
///
/// A count is the whole of what a card about a list says. What each of them is
/// called is the pane's, and there is nothing else about one to scan for: a
/// Repo is a directory somebody named once.
export function ReposCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const repos = useRepos();

  return (
    <Switch>
      <Match when={repos.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={repos.isError}>
        <ErrorLine>
          Could not read the registered Repos: {repos.error?.message}
        </ErrorLine>
      </Match>
      <Match when={repos.data}>
        {(registered) => (
          <CardButton
            as="article"
            class={styles.reposCard}
            open={props.open}
            press={props.press}
          >
            <h2>Repos</h2>
            <p class={styles.standing}>{counted(registered().length)}</p>
          </CardButton>
        )}
      </Match>
    </Switch>
  );
}

/// How the card says how many there are, in English rather than as `1 repos`.
function counted(many: number): string {
  if (many === 0) {
    return "No repos are registered yet.";
  }

  return `${many} ${many === 1 ? "repo" : "repos"} registered.`;
}

/// And every one of them by name, which is the details pane the card opens.
///
/// A row is the name and the one press there is to make about a Repo. Nothing
/// else: what a repository is holding is read where it is used rather than
/// listed here, and a list somebody scans for the name they know is a list of
/// names.
///
/// There is no Save over the whole of it and no Cancel: each Remove is its own
/// press, and a details pane is left by opening something else or by the way
/// back a narrow window draws.
export function ReposPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const repos = useRepos();

  // The Repo a press has asked about, while the card asking is up — or `null`
  // while none is. The whole entry rather than its id: the card names it, and
  // the removal is made from the card rather than from the row it was pressed
  // on.
  const [asking, setAsking] = createSignal<RepoEntry | null>(null);

  // Why the last removal did not happen, and which row it was pressed on.
  // Nothing clears it but another press: what answers a refusal is doing
  // something about the conversation it named.
  const [refused, setRefused] = createSignal<{
    repo: number;
    said: string;
  } | null>(null);

  const remove = useMutation(() => ({
    mutationFn: (repo: RepoEntry) => removeRepo(repo.id),
    onSuccess: (outcome: RepoRemoved, repo: RepoEntry) => {
      // Whichever it was, this page's account of the Repos is older than the
      // server's now: a refusal is about work that has moved on, and a removal
      // is the list changing. The roadmaps waiting go with them, because they
      // are read off whatever is registered.
      void queries.invalidateQueries({ queryKey: ["repos"] });
      void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });

      if (outcome !== "Removed") {
        setRefused({ repo: repo.id, said: REPO_REMOVAL_REFUSAL[outcome] });
      }
    },
    // A server that could not answer at all, which is the one thing here that
    // is an error rather than an outcome. Said under the row it was pressed on,
    // like a refusal, because that is the whole of what tells two presses on
    // this list apart.
    onError: (error: Error, repo: RepoEntry) =>
      setRefused({
        repo: repo.id,
        said: `The repo could not be removed: ${error.message}`,
      }),
  }));

  /// What a Remove on a row does, which is ask rather than act.
  const ask = (repo: RepoEntry) => {
    setRefused(null);
    setAsking(repo);
  };

  /// And what the press on the card that asked does, which is the removal.
  const confirmed = () => {
    const repo = asking();
    setAsking(null);

    if (repo !== null) {
      remove.mutate(repo);
    }
  };

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title="Repos" />
      </PaneSticky>

      <Switch>
        <Match when={repos.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={repos.isError}>
          <ErrorLine>
            Could not read the registered Repos: {repos.error?.message}
          </ErrorLine>
        </Match>
        <Match when={repos.data?.length === 0}>
          <Empty>No repos are registered yet.</Empty>
        </Match>
        <Match when={repos.data}>
          {(registered) => (
            <ul class={styles.list}>
              <For each={registered()}>
                {(repo) => (
                  <li class={styles.repo}>
                    <div class={styles.row}>
                      <span class={styles.name}>{repo.name}</span>
                      <button
                        type="button"
                        class={styles.remove}
                        disabled={remove.isPending}
                        onClick={() => ask(repo)}
                      >
                        Remove
                      </button>
                    </div>

                    {/* Refused rather than taken out from under the work going
                        on in it, and said under the row the press was made on:
                        the name above it is what says which Repo this is
                        about. */}
                    <Show when={refused()?.repo === repo.id}>
                      <ErrorLine class={styles.failure}>
                        {refused()?.said}
                      </ErrorLine>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
          )}
        </Match>
      </Switch>

      {/* Asked before anything happens, the way a close over a run in flight
          is: one press in a list, on a phone, that cannot be taken back. */}
      <Confirm
        asked={asking()}
        keep={() => setAsking(null)}
        remove={confirmed}
      />
    </>
  );
}

/// What a Remove is answered with before anything has happened: which Repo it
/// would take off the registry, what that leaves alone, and the two ways out.
///
/// The Repo is named on the card because the list it was pressed in is behind
/// it: a card reading *Remove this repo?* over a list of them would be asking
/// about whichever row the human remembers pressing.
function Confirm(props: {
  /// The Repo the press asked about, or `null` while nothing is being asked.
  asked: RepoEntry | null;
  /// The way back, which is what Escape and a press on the backdrop come to as
  /// well: every way out of this card but the one button leaves the registry
  /// alone.
  keep: () => void;
  /// And the press it asked about, made.
  remove: () => void;
}): JSX.Element {
  // The heading's own id, for the `aria-labelledby` that names the card by it.
  const id = createUniqueId();

  return (
    <Modal
      class={styles.confirming!}
      open={props.asked !== null}
      close={props.keep}
      labelledBy={id}
    >
      <p id={id} class={styles.confirmingTitle}>
        Remove {props.asked?.name}?
      </p>
      <p class={styles.confirmingWhy}>
        It comes off the registry. The directory is left where it is, and every
        conversation worked in it goes on saying so.
      </p>
      <div class={styles.confirmingOut}>
        {/* Both classes, as every confirm pair in the app carries them: the
            global one is the paint, and the module's is what the row stands the
            filled press out of. */}
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          onClick={() => props.keep()}
        >
          Keep it
        </button>
        <button type="button" onClick={() => props.remove()}>
          Remove
        </button>
      </div>
    </Modal>
  );
}

/// The registration itself: a path, a press, and every refusal said under the
/// field it was typed into.
///
/// Its own piece because it is asked in two places — the **Open repo** modal
/// the Repo dropdown opens, and the **Create repo** card beside it, which asks
/// a different question and answers it in the same words. A second copy would
/// be a second set of words for the same refusals, which is exactly what
/// [`REFUSAL`] exists to prevent one row further down.
///
/// The field browses, and the press is untouched by it: what is sent is whatever
/// the box holds, tapped together or typed straight in, and every refusal is
/// still the server's answer about it. Anywhere the server can read, because
/// that is where a Repo may be registered from, and looking for a repository,
/// which is what this form is being filled in with.
export function RepoRegistration(props: {
  /// What the button reads, which is what the human came here to do: open a
  /// repository to work in.
  submit: string;

  /// What a registration that left a Repo registered does with it.
  ///
  /// Handed the Repo rather than told it landed, because the caller that opened
  /// this to get *onto* a repository has to put a draft on the one it just
  /// named — and the path that was typed is not the resolved path the Repo is
  /// recorded under.
  landed: (repo: RepoEntry) => void;

  /// What else the form draws, under the button: the modal's way out.
  children?: JSX.Element;
}): JSX.Element {
  const queries = useQueryClient();

  // The path typed into the field.
  const [path, setPath] = createSignal("");

  // What the server said about the last path offered, or `null` while nothing
  // has been. Cleared as soon as the field is touched: a refusal is about the
  // path that was sent, and it stops being about the one being typed.
  const [refused, setRefused] = createSignal<RepoRefused | null>(null);

  const register = useMutation(() => ({
    mutationFn: (asked: string) => registerRepo(asked),
    onSuccess: (outcome: Registered) => {
      // A refusal, which is a bare word on the wire — the two outcomes that
      // leave a Repo registered carry it. Said where the path was typed, which
      // stays up: the path that was refused is the one about to be corrected.
      if (typeof outcome === "string") {
        setRefused(outcome);
        return;
      }

      // A path registered already is the repository somebody named rather than
      // a dead end: this form is a way *onto* one, and the Repo comes back with
      // that outcome too.
      //
      // The list this was filled in over is now out of date — the repo
      // appearing on it is the whole of the confirmation. And the roadmaps
      // waiting go with it, as they do when a repo is removed: they are read off
      // whatever is registered, so a repository arriving with an unadopted
      // roadmap in it has something to offer the moment it lands — and
      // registering a path that was taken away brings a whole repository's worth
      // back at once.
      void queries.invalidateQueries({ queryKey: ["repos"] });
      void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });
      props.landed(
        "Added" in outcome ? outcome.Added : outcome.AlreadyRegistered,
      );
    },
  }));

  const add = (ev: SubmitEvent) => {
    ev.preventDefault();

    const asked = path().trim();
    if (asked === "") {
      return;
    }

    register.mutate(asked);
  };

  return (
    /* Every way it can be refused is said inside it, because a refusal is
       answered by correcting the path. */
    <form class={styles.form} onSubmit={add}>
      <label for="repo-path">Absolute path of a git repository</label>
      <PathField
        id="repo-path"
        repositories
        placeholder="/home/you/src/verkstead"
        value={path()}
        write={(asked) => {
          setPath(asked);
          setRefused(null);
        }}
      />

      <div class={styles.buttons}>
        <button
          type="submit"
          disabled={register.isPending || path().trim() === ""}
        >
          {props.submit}
        </button>
        {props.children}
      </div>

      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>{REFUSAL[outcome()]}</ErrorLine>
        )}
      </Show>
      {/* A server that could not answer at all, which is the one thing here
          that is an error rather than an outcome. */}
      <Show when={register.isError}>
        <ErrorLine class={styles.failure}>
          The repo could not be registered: {register.error?.message}
        </ErrorLine>
      </Show>
    </form>
  );
}

/// The registration as the Repo dropdown asks it: **Open repo**, on the modal
/// every other card in front of the page is drawn on.
///
/// The one way a repository that already exists is taken on, now that the
/// settings have no registration of their own: somebody who has nowhere to do
/// the work says where it is, at the moment they are starting it.
///
/// A refusal keeps the modal up: a refused path is the one about to be
/// corrected, and taking the card away would take the field with it.
export function OpenRepo(props: {
  /// Said when the modal has closed itself — Escape, a press on the backdrop,
  /// or the Cancel beside the press.
  close: () => void;
  /// And what a registration that landed does with the Repo, which is the
  /// caller's: the draft goes onto it.
  landed: (repo: RepoEntry) => void;
}): JSX.Element {
  // The heading's own id, for the `aria-labelledby` that names the card by it.
  // Generated rather than written, the two Repo dropdowns being drawable on one
  // page and an id being the page's to keep unique.
  const id = createUniqueId();

  return (
    <Modal class={styles.openRepo!} open close={props.close} labelledBy={id}>
      <h3 id={id}>Open a repo</h3>

      <RepoRegistration submit="Open" landed={props.landed}>
        {/* Drawn as well as the ways out the modal already has: Escape and a
            press on the backdrop are for a keyboard and a cursor, and this is
            the one a thumb has. */}
        <button
          type="button"
          class={`${styles.cancel} secondary`}
          onClick={props.close}
        >
          Cancel
        </button>
      </RepoRegistration>
    </Modal>
  );
}

/// And the other row at that foot: **Create repo**, which makes one rather than
/// taking on one that is there.
///
/// Two fields, because the two halves are answered differently: the parent is
/// browsed for — the same field the Open modal beside it stands on, without
/// repositories marked, since what is being picked is where a repository will go
/// rather than one that is already there — and the name is typed. They are sent
/// as two, joining them into a path here being the one place the browser would
/// build one out of a separator the server never agreed to.
///
/// **The parent is remembered on the device.** Somebody making a second
/// repository is almost certainly putting it beside the first, and where they
/// keep their code is a fact about the machine in front of them rather than
/// something to tell the server — so it is kept where the wrap setting is, in
/// `device.ts`, and the field opens inside it. Where there is none, which a
/// first run always is, the field stands empty and browses the server's own
/// home, which is where an unbounded browse already opens.
///
/// **And a tick that makes the same repository on GitHub**, drawn only where a
/// token is saved. The settings say whether there is one without ever handing it
/// over, which is the whole of what this has to know. Where there is one the
/// tick is drawn and starts on, and what it makes is private: a repository made
/// from here is somebody's work before it is anybody else's business, and public
/// is a decision to take deliberately rather than by leaving a box alone.
///
/// Where there is none the tick is not drawn at all, and the card says a remote
/// is needed before the work is finished — the pipeline ends in a push and a
/// pull request, so a repository with nowhere to push is one that will stop
/// halfway through the first Conversation. A sentence rather than a refusal: the
/// local repository is still worth making, and the token can be saved
/// afterwards.
///
/// **And neither of them until the settings have answered**, which is a state of
/// its own rather than the second one said early. Nothing else on either page
/// this card opens over reads the settings, so that query is cold every time it
/// opens: a card that took *not read yet* for *no token* would put the note in
/// front of somebody who has one, every time, and swap it for the tick it was
/// denying a moment later. So it says nothing about GitHub while it does not
/// know, and takes no create either — a create that went out then would ask
/// nothing of GitHub with the card never having said so.
///
/// A read that *failed* is an answer, though, and the answer is the same as no
/// token: the local repository is still worth making, and a card that stayed
/// shut because the settings were unreadable would be a create nobody could
/// make for a reason that has nothing to do with creating.
///
/// A refusal keeps the modal up with the reason under the fields, for the
/// registration's reason: what answers a refusal is correcting what was typed,
/// and a modal that closed on one would take the correction away with it.
///
/// **A create GitHub would not finish is not a refusal.** The repository is
/// there and registered, and what is missing is a remote that can be added
/// afterwards — so the card stops being a form and becomes what happened:
/// `gh`'s own words for it, and one press out. The Repo goes onto the draft on
/// the way out rather than the moment it arrives, because landing one is what
/// takes this card away — on the compose page the dropdown it was opened from
/// *becomes* the panel as soon as a repo is on the draft — and a card that
/// landed it at once would be a card that vanished with the reason unread.
export function CreateRepo(props: {
  /// Said when the modal has closed itself — Escape, a press on the backdrop,
  /// or the Cancel beside the press.
  close: () => void;
  /// And what a create that landed does with the Repo it made, which is the
  /// caller's: the draft goes onto it, and the card is spent.
  landed: (repo: RepoView) => void;
}): JSX.Element {
  const queries = useQueryClient();

  // What Verkstead has been told, for the one thing this card asks of it:
  // whether a GitHub token is saved. Never the token itself — what comes back
  // about it is that there is one and what its last four characters are.
  //
  // Read from here rather than already in hand: nothing else on the compose
  // page or on a draft's Repo panel reads the settings, so this card is always
  // what starts that read — which is why the answer's absence has to be a state
  // of its own below.
  const settings = useSettings();

  // Whether there is one — and `null` while the read above is still out, which
  // is neither.
  //
  // Three states rather than two, because a card that read an unanswered query
  // as *no token* would say so: the note under the fields would stand where the
  // tick belongs, be read, and be replaced a moment later by the tick it was
  // denying. What the card does while it does not know is say nothing about
  // GitHub and take no create — see the two `Show`s and the press below.
  //
  // Pending rather than absent data, so that a settings read which *failed* is
  // an answer like any other: false, which is the true one — a token nothing
  // can read is a token nothing can make a repository as — and the local
  // repository is still worth making, which a card held shut would not be.
  const tokened = (): boolean | null =>
    settings.isPending ? null : settings.data?.github_token != null;

  // And whether it is ticked, which starts on: somebody who has saved a token
  // has said what they mean to do with it.
  const [onGithub, setOnGithub] = createSignal(true);

  // The heading's own id, for [`OpenRepo`]'s reason: two Repo dropdowns may be
  // drawn on one page, and an id is the page's to keep unique.
  const id = createUniqueId();

  // Where the last repo on this device went, which is where this one starts —
  // and, because that is a path handed over rather than one being typed, where
  // its browse opens: see `opened` on `PathField`, which is the tap that wrote
  // it, made on an earlier visit. Empty on the first run, and an empty field
  // browses the server's own home.
  const remembered = repoParent();

  const [parent, setParent] = createSignal(remembered);

  // And what it is called, which is the directory's name and so the Repo's.
  const [name, setName] = createSignal("");

  // Why the last pair offered was refused, in the words it is shown in — or
  // `null` while nothing has been offered. Worded as it arrives rather than
  // where it is drawn, because one of the five is git's own sentence and the
  // other four are this file's. Cleared as soon as either field is touched, the
  // registration's refusal being cleared the same way: a refusal is about what
  // was sent, and it stops being about what is being typed.
  const [refused, setRefused] = createSignal<string | null>(null);

  // The repository a create made that GitHub would not finish, held here rather
  // than handed over the moment it arrives.
  //
  // Landing it is what takes this card away: on the compose page the dropdown
  // the card was opened from *becomes* the panel as soon as a repo is on the
  // draft, and the card goes with the dropdown. So a card that landed one at
  // once would be a card that vanished with the reason still unread. It goes
  // over when the human has done reading, which is what closing the card says —
  // and it is registered from the moment the server answered either way, so
  // nothing is waiting on this but the draft.
  const [unpushed, setUnpushed] = createSignal<RepoView | null>(null);

  /// The one way out of this card, however it was taken — Escape, the backdrop,
  /// or the press. A repository this card made goes onto the draft on the way.
  const leave = () => {
    const held = unpushed();

    if (held !== null) {
      // Which shuts the card as well: landing a Repo is what spends it.
      props.landed(held);
      return;
    }

    props.close();
  };

  const create = useMutation(() => ({
    mutationFn: (asked: { parent: string; name: string; github: boolean }) =>
      createRepo(asked.parent, asked.name, asked.github),
    onSuccess: (outcome: Created) => {
      // Four of the refusals are a bare word, which this file has the sentence
      // for; the fifth is git's own account of what it would not do, said in
      // git's words because nothing here could put it better.
      if (typeof outcome === "string") {
        setRefused(CREATE_REFUSAL[outcome]);
        return;
      }

      if ("Refused" in outcome) {
        setRefused(outcome.Refused);
        return;
      }

      // A create GitHub would not finish is not a refused create: the directory,
      // the commit and the registration all stand, and what is missing is a
      // remote that can be added afterwards. So the card stops being a form and
      // becomes what happened — the reason in `gh`'s own words, and the repo it
      // is holding for the draft.
      if ("MadeWithoutRemote" in outcome) {
        setRefused(outcome.MadeWithoutRemote.why);
        made(outcome.MadeWithoutRemote.repo);
        setUnpushed(outcome.MadeWithoutRemote.repo);
        return;
      }

      made(outcome.Made);
      props.landed(outcome.Made);
    },
  }));

  /// What every create that left a repository behind does with it, whether or
  /// not GitHub was reached — everything but landing it on the draft, which is
  /// the one thing the two answer differently.
  const made = (repo: RepoView) => {
    // The parent as the server resolved it rather than as it was typed: that
    // is the directory the repository is actually in, and so the one the next
    // create should open in.
    const cut = repo.path.lastIndexOf("/");
    setRepoParent(cut > 0 ? repo.path.slice(0, cut) : "/");

    // The list this was made over is now out of date, and so are the roadmaps
    // waiting to be adopted — a registration invalidates both for the same
    // reason, and a repository that was just made is a repository that has
    // just arrived.
    void queries.invalidateQueries({ queryKey: ["repos"] });
    void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });
  };

  const make = (ev: SubmitEvent) => {
    ev.preventDefault();

    const where = parent().trim();
    const called = name().trim();
    // Nothing while the settings are unread, for the press's own reason: what
    // this card would ask of GitHub is not settled yet.
    if (where === "" || called === "" || tokened() === null) {
      return;
    }

    // With nothing saved to make it as, nothing is asked of GitHub whatever a
    // stale tick would have said.
    create.mutate({
      parent: where,
      name: called,
      github: tokened() === true && onGithub(),
    });
  };

  return (
    <Modal class={styles.createRepo!} open close={leave} labelledBy={id}>
      <h3 id={id}>Create a repo</h3>

      {/* Once there is a repository the card is no longer a form: what is left
          to do about it is read what GitHub said and get on, and a Create press
          under a repository that exists would only ever be refused. */}
      <Show when={unpushed()}>
        {(repo) => (
          <div class={styles.made}>
            <Note>
              {repo().name} is made here and registered, and the draft goes on
              it. It has no remote yet — add one, or make it on GitHub yourself,
              before the work on it is finished.
            </Note>

            <ErrorLine class={styles.failure}>{refused()}</ErrorLine>

            <div class={styles.buttons}>
              <button type="button" onClick={leave}>
                Done
              </button>
            </div>
          </div>
        )}
      </Show>

      <Show when={unpushed() === null}>
        <form class={styles.form} onSubmit={make}>
          <label for="repo-parent">Where it goes</label>
          <PathField
            id="repo-parent"
            opened={remembered !== ""}
            placeholder="/home/you/src"
            value={parent()}
            write={(where) => {
              setParent(where);
              setRefused(null);
            }}
          />

          <label class={styles.second} for="repo-name">
            What it is called
          </label>
          <input
            id="repo-name"
            class={styles.name}
            type="text"
            placeholder="verkstead"
            value={name()}
            onInput={(ev) => {
              setName(ev.currentTarget.value);
              setRefused(null);
            }}
          />

          {/* And the third question, asked only where there is a token to answer
            it with — see the note below, which is what stands here instead.
            Neither of them until the settings have answered: what is drawn there
            is what the card is about to do about GitHub, and it does not know
            yet. */}
          <Show when={tokened() === true}>
            <label class={styles.github}>
              <input
                type="checkbox"
                checked={onGithub()}
                onChange={(ev) => setOnGithub(ev.currentTarget.checked)}
              />
              Create it on GitHub too, privately
            </label>
          </Show>

          {/* A sentence rather than a refusal: the local repository is worth
            making, and the token can be saved afterwards. But the pipeline ends
            in a push and a pull request, so a repository with nowhere to push
            is one that will stop halfway through the first conversation. */}
          <Show when={tokened() === false}>
            <Note class={styles.remote}>
              No GitHub token is saved, so this repo is made here only. It needs
              a remote before the work on it can be finished — Settings has the
              token.
            </Note>
          </Show>

          <div class={styles.buttons}>
            {/* Held while the settings are unread as well as while a field is
                empty, and for the same kind of reason: a create that went out
                then would ask nothing of GitHub without the card ever having
                said so, which is the sentence above going missing rather than
                being wrong. */}
            <button
              type="submit"
              disabled={
                create.isPending ||
                tokened() === null ||
                parent().trim() === "" ||
                name().trim() === ""
              }
            >
              Create
            </button>
            {/* Drawn as well as the ways out the modal already has, for the reason
              the Open modal draws one: Escape and a press on the backdrop are
              for a keyboard and a cursor, and this is the one a thumb has. */}
            <button
              type="button"
              class={`${styles.cancel} secondary`}
              onClick={leave}
            >
              Cancel
            </button>
          </div>

          <Show when={refused()}>
            {(why) => <ErrorLine class={styles.failure}>{why()}</ErrorLine>}
          </Show>
          {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
          <Show when={create.isError}>
            <ErrorLine class={styles.failure}>
              The repo could not be made: {create.error?.message}
            </ErrorLine>
          </Show>
        </form>
      </Show>
    </Modal>
  );
}
