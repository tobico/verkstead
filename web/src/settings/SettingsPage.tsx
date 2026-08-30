//! Everything the human configures, on the same three panes the workbench
//! stands on: the conversations down the left, the settings themselves in the
//! middle, and whatever of them is being rewritten in the details pane beside
//! it.
//!
//! One page rather than three, because all of it is the same kind of thing —
//! settled once, and then left alone for weeks. Three pages meant three trips
//! out of the workbench to set a machine up, and a sidebar naming each of them;
//! folded together they are sections of one pane, read down in the order a
//! fresh install needs them: credentials first, because without them nothing a
//! session does with a Repo can be pushed, then the shared Rust build cache
//! every session builds into, then where the share viewer is hosted, then the
//! paths Verkstead may work inside at all, then the Agent Profiles and the Repos
//! a Conversation is settled against.
//!
//! The conversations pane rides along because it is the app's navigation rather
//! than the workbench's furniture: configuring a machine is something done
//! *while* work is going on, and a page that took the list away made the human
//! leave the settings to see whether anything had moved. It is the same
//! component the workbench draws, so the gear at its head, the way to start a
//! Conversation and the archived switch at its foot all come with it — and the
//! gear reads as open, this being the pane it opens.
//!
//! Which level a narrow window is showing is this page's, as it is the
//! workbench's: the walk is conversations → settings → detail, one pane at a
//! time, and each pane's way back out is drawn by its own head and hidden by the
//! frame wherever the pane it goes back to is already on screen. The way out of
//! the settings themselves is a navigation rather than a change of level,
//! because the settings are a page and leaving them is leaving it.
//!
//! What the details pane is showing has a path of its own under `/settings`, the
//! way a Conversation's panes have one under the Conversation — see
//! `openings.ts`. Entering the settings pushes and changing which detail is open
//! replaces, so Back from a details pane leaves the settings rather than walking
//! back through every one of them that was looked at.
//!
//! The two switches that are about this device and this server rather than
//! about any one Conversation come with the Repos they were on — whether the
//! phone is told about a Question Set, and whether a newer Verkstead has been
//! released. Both belong here for the same reason everything else does, and
//! neither is a card: the switch is one control and the banner asks for nothing.

import { Route, useLocation, useNavigate } from "@solidjs/router";
import { Match, Switch, createMemo, createSignal, type JSX } from "solid-js";

import { Panes, PaneSticky, type Pane } from "../Panes";
import { ProfileList, ProfilePane } from "../profiles/ProfileList";
import { Notifications } from "../push/Notifications";
import { RepoDetails, RepoList, RepoPane } from "../repos/RepoList";
import { UpdateNotice } from "../update/UpdateNotice";
import { Conversations } from "../workbench/Conversations";
import { PaneHead } from "../workbench/PaneHead";
import { pathOf } from "../workbench/openings";
import { BuildCacheCard, BuildCachePane } from "./BuildCache";
import { GithubCard, GithubPane } from "./Credentials";
import { PathsCard, PathsPane } from "./Paths";
import { ShareViewerCard, ShareViewerPane } from "./ShareViewer";
import {
  SETTINGS,
  WORDS,
  openingAt,
  opensProfile,
  opensRepo,
  pathTo,
  profileOpened,
  repoOpened,
  type Opening,
} from "./openings";
import styles from "./SettingsPage.module.css";

/// Every details pane of this page, as the routes that reach one.
///
/// Declared here rather than in `App.tsx` because they are this page's own
/// arithmetic: what a pane is reached at is [`pathTo`]'s answer, and a route
/// table written somewhere else is a second opinion about it — one that agreed
/// with this page for as long as somebody remembered to keep the two in step,
/// and then quietly stopped. That is what happened to the share viewer: card,
/// pane and path all in hand, and *No such page* at the end of it, because a
/// nested route with no matching child falls to the app's catch-all.
///
/// So the ones named by a word are written from [`WORDS`], which is what
/// [`openingAt`] reads a path against — a section added there arrives with the
/// route that reaches it. The two named by an id keep their own line: what
/// stands in that segment is an id or the word `new`, and no id the server
/// issues is `new`.
///
/// The leaves draw nothing. What they are is what the path says, and this page
/// reads that off the URL — they are here so the parent route matches, and so
/// that pressing a card does not take the middle pane down with it.
///
/// A plain function rather than a component: the router reads the routes out of
/// the JSX handed to it rather than out of anything a component renders, so this
/// is called where they are written.
export function panes(): JSX.Element {
  return (
    <>
      <Route path="/" />
      {WORDS.map((word) => (
        <Route path={`/${word}`} />
      ))}
      {/* The blank form rides in the same segment an id does, as
          `/settings/profiles/new`. */}
      <Route path="/profiles/:profile" />
      <Route path="/repos/:repo" />
    </>
  );
}

/// The settings page, whole.
export function SettingsPage(): JSX.Element {
  const navigate = useNavigate();
  const where = useLocation();

  /// What the details pane is showing, read off the path rather than held
  /// beside it, so there is one account of what is open.
  const opening = createMemo(() => openingAt(where.pathname));

  /// Which level a narrow window is showing. The settings themselves when the
  /// page is entered, and the details straight away where the path names one —
  /// which is a cold load of a details pane, a reload or a link somebody kept.
  ///
  /// Read once, at the moment the page is built, rather than followed: what
  /// moves it afterwards is the human pressing a card or a way back, and a page
  /// that recomputed it from the URL would carry them into the details pane
  /// every time one was opened on a wide window.
  const [pane, setPane] = createSignal<Pane>(
    opening() === null ? "middle" : "details",
  );

  /// Opening a details pane, which is a navigation to where that pane stands
  /// and a walk into it on a narrow window.
  ///
  /// It replaces rather than pushes: the details of the settings are places in a
  /// page rather than pages, so walking between them should not have to be
  /// walked back out of one at a time. What Back leaves is the settings.
  const select = (open: Opening) => {
    navigate(pathTo(open), { replace: true });
    setPane("details");
  };

  /// And a details pane spending itself, which is what a Profile saved or
  /// removed, or a Repo registered or taken off the registry, leaves behind: the
  /// pane was asked about something that is settled now, so the settings are what
  /// stands after it and the cards there are what say the work landed.
  ///
  /// Replacing for the reason opening one does, and over the entry opening one
  /// already wrote: the settings keep the single history entry they were entered
  /// on, whatever was opened and shut inside them.
  const shut = () => {
    navigate(SETTINGS, { replace: true });
    setPane("middle");
  };

  return (
    <Panes
      pane={pane()}
      middleLabel="Settings"
      conversations={
        <Conversations selected="" open={(id) => navigate(pathOf(id))} />
      }
      middle={
        <Settings
          opening={opening()}
          select={select}
          back={() => navigate("/")}
        />
      }
      details={
        <Details
          opening={opening()}
          back={() => setPane("middle")}
          done={shut}
        />
      }
    />
  );
}

/// The middle pane: everything the human configures, in the order a fresh
/// install needs it.
function Settings(props: {
  /// What the details pane is showing, which is what says which card is open.
  opening: Opening | null;
  /// And how to change it.
  select: (opening: Opening) => void;
  /// The way back out to the conversations, which is a navigation: the settings
  /// are a page, and leaving them is leaving it. Drawn always and hidden by the
  /// frame wherever the conversations are already on screen.
  back: () => void;
}): JSX.Element {
  return (
    <>
      {/* The switch on the head's line rather than in a section of its own:
          whether this device is told about a Question Set is one switch, and a
          switch is small enough to live in the space the title was leaving
          empty anyway. */}
      <PaneSticky>
        <PaneHead back={{ to: "Conversations", go: props.back }} title="Settings">
          <Notifications />
        </PaneHead>
      </PaneSticky>

      <div class={styles.settings}>
        {/* Under the head and above everything else: it is about the server the
            whole page came from rather than about anything on it, and it asks
            for nothing — so it is read on the way past, once. Drawn only when
            there is a release waiting; this is nothing at all the rest of the
            time. */}
        <UpdateNotice />

        <GithubCard
          open={props.opening === "github"}
          press={() => props.select("github")}
        />
        {/* Under the credentials and above the lists: it is the other thing
            Verkstead itself was told rather than anything a Conversation is
            settled against, and the one setting on this page about what a
            session runs inside. */}
        <BuildCacheCard
          open={props.opening === "build-cache"}
          press={() => props.select("build-cache")}
        />
        {/* And the last of the three things Verkstead itself was told: where the
            page that draws a published share is hosted. Under the other two
            because it is the one a machine works perfectly without — what it
            costs to leave alone is a worse read for whoever a share is sent
            to. */}
        <ShareViewerCard
          open={props.opening === "share-viewer"}
          press={() => props.select("share-viewer")}
        />
        {/* The last of what Verkstead itself was told, and the one thing under
            the lists rather than beside them: where Verkstead may work at all,
            and what a session is given beyond its worktree. It stands directly
            above the Repos because it is what they are registered from — a
            machine with no watched path has nothing to put on that list. */}
        <PathsCard
          open={props.opening === "paths"}
          press={() => props.select("paths")}
        />
        {/* Told which of its own things is open rather than the whole opening:
            where a Profile's pane stands is this page's arithmetic, and a
            section that knew the settings' paths would be a second opinion
            about them. */}
        <ProfileList
          opening={profileOpened(props.opening)}
          open={(id) => props.select(opensProfile(id))}
          add={() => props.select(opensProfile("new"))}
        />
        {/* Told which of its own things is open rather than the whole opening,
            for the reason the Profiles are: where a Repo's pane stands is this
            page's arithmetic. */}
        <RepoList
          opening={repoOpened(props.opening)}
          open={(id) => props.select(opensRepo(id))}
          add={() => props.select(opensRepo("new"))}
        />
      </div>
    </>
  );
}

/// The details pane: whatever the settings have open, and bare paper where
/// nothing is.
function Details(props: {
  opening: Opening | null;
  /// The way back out to the settings, which is a change of level rather than a
  /// navigation: what is open stays open, and the URL goes on saying so.
  back: () => void;
  /// And the way out of a pane that has spent itself, which is a navigation.
  done: () => void;
}): JSX.Element {
  /// Which Profile the pane is about, as something to key on: a new object each
  /// time it really changes, and the same one for as long as it does not.
  ///
  /// The pane stands inside a `keyed` Match over it, so opening another Profile
  /// tears the form down and builds it again from nothing. Without the key the
  /// component would be kept and only its prop changed, and whatever had been
  /// typed into the last Profile's form would go on standing in this one's.
  ///
  /// An object rather than the value itself, because an id of nothing but digits
  /// is no use as a condition: the first Profile the server ever issued would
  /// read as nothing being open at all.
  const profile = createMemo(() => {
    const one = profileOpened(props.opening);
    return one === null ? null : { which: one };
  });

  /// And which registered Repo, the same way and for the same reason. Never the
  /// form: registering one is a pane of its own below, so what is left here is
  /// an id.
  const repo = createMemo(() => {
    const one = repoOpened(props.opening);
    return typeof one === "number" ? { which: one } : null;
  });

  return (
    <Switch>
      <Match when={props.opening === "github"}>
        <GithubPane back={props.back} />
      </Match>
      <Match when={props.opening === "build-cache"}>
        <BuildCachePane back={props.back} />
      </Match>
      <Match when={props.opening === "share-viewer"}>
        <ShareViewerPane back={props.back} />
      </Match>
      <Match when={props.opening === "paths"}>
        <PathsPane back={props.back} />
      </Match>
      {/* The Repos' two panes are two components rather than one asked about a
          Repo that does not exist yet, the way the Profiles' one form is: what
          registers a Repo is a path typed, and what an opened one draws is
          everything the repository and the store say about it. */}
      <Match when={repoOpened(props.opening) === "new"}>
        <RepoPane back={props.back} done={props.done} />
      </Match>
      <Match when={repo()} keyed>
        {(open) => (
          <RepoDetails
            repo={open.which}
            back={props.back}
            done={props.done}
          />
        )}
      </Match>
      <Match when={profile()} keyed>
        {(open) => (
          <ProfilePane
            profile={open.which}
            back={props.back}
            done={props.done}
          />
        )}
      </Match>
    </Switch>
  );
}
