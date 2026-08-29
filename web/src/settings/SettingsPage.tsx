//! Everything the human configures, on one page: the GitHub token and the git
//! author Verkstead itself was told, the shared Rust build cache its sessions
//! build into, the Agent Profiles a session runs under, and the Repos a
//! Conversation can be started against.
//!
//! One page rather than three, because all of it is the same kind of thing —
//! settled once, and then left alone for weeks. Three pages meant three trips
//! out of the workbench to set a machine up, and a sidebar naming each of them;
//! folded together they are sections of one page, read down in the order a
//! fresh install needs them: credentials first, because without them nothing a
//! session does with a Repo can be pushed.
//!
//! The two switches that are about this device and this server rather than
//! about any one Conversation come with the Repos they were on — whether the
//! phone is told about a Question Set, and whether a newer Verkstead has been
//! released. Both belong here for the same reason everything else does.

import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import { ProfileList } from "../profiles/ProfileList";
import { Notifications } from "../push/Notifications";
import { RepoList } from "../repos/RepoList";
import { UpdateNotice } from "../update/UpdateNotice";
import { BuildCache } from "./BuildCache";
import { Credentials } from "./Credentials";
import styles from "./SettingsPage.module.css";

/// The settings page, whole.
export function SettingsPage(): JSX.Element {
  return (
    <div class={styles.listPage}>
      {/* The way back to the workbench, in the slot every page keeps for where
          else there is to go. */}
      <A class={styles.back} href="/">
        ← Workbench
      </A>
      {/* On the title's line rather than in a section of its own: whether this
          device is told about a Question Set is one switch, and a switch is
          small enough to live in the space the heading was leaving empty
          anyway. */}
      <div class={styles.pageHead}>
        <h1>Settings</h1>
        <Notifications />
      </div>
      {/* Under the heading and above everything else: it is about the server
          the whole page came from rather than about anything on it, and it asks
          for nothing — so it is read on the way past, once. Drawn only when
          there is a release waiting; this is nothing at all the rest of the
          time. */}
      <UpdateNotice />

      <Credentials />
      {/* Under the credentials and above the lists: it is the other thing
          Verkstead itself was told, rather than anything a Conversation is
          settled against — and it is the one setting here about what a session
          runs inside. */}
      <BuildCache />
      <ProfileList />
      <RepoList />
    </div>
  );
}
