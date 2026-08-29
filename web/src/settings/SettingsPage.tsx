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
//! session does with a Repo can be pushed.
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

import { useLocation, useNavigate } from "@solidjs/router";
import { Show, createMemo, createSignal, type JSX } from "solid-js";

import { Panes, type Pane } from "../Panes";
import { ProfileList } from "../profiles/ProfileList";
import { Notifications } from "../push/Notifications";
import { RepoList } from "../repos/RepoList";
import { UpdateNotice } from "../update/UpdateNotice";
import { Conversations } from "../workbench/Conversations";
import { PaneHead } from "../workbench/PaneHead";
import { pathOf } from "../workbench/openings";
import { GithubCard, GithubPane } from "./Credentials";
import { openingAt, pathTo, type Opening } from "./openings";
import styles from "./SettingsPage.module.css";

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
        <Details opening={opening()} back={() => setPane("middle")} />
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
      <PaneHead back={{ to: "Conversations", go: props.back }} title="Settings">
        <Notifications />
      </PaneHead>

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
        <ProfileList />
        <RepoList />
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
}): JSX.Element {
  return (
    <Show when={props.opening === "github"}>
      <GithubPane back={props.back} />
    </Show>
  );
}
