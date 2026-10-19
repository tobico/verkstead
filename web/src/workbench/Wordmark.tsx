//! The head of whatever page Verkstead is entered at: the mark, the name, and
//! the gear on to the settings — or, in the sidebar's collapsed strip, the
//! mark alone.
//!
//! It lived in the conversations sidebar, that being the only page it was ever
//! drawn on. There are two now — the sidebar, and the compose page standing
//! without one while there is nothing to list (see `zero.ts`) — so it is here
//! rather than written a second time over there.
//!
//! Not the sticky block it sits in: what a pane keeps against its top edge is
//! the pane's own business, so each of the two hands this to a `PaneSticky` of
//! its own.

import { faGear } from "@fortawesome/free-solid-svg-icons";
import { useLocation, useNavigate } from "@solidjs/router";
import type { JSX } from "solid-js";

import { IconButton } from "../IconButton";
import { PaneHead } from "./PaneHead";
import styles from "./Wordmark.module.css";

/// The mark rather than a title: this is where Verkstead is entered, and what
/// stands under it says what it is a list of.
///
/// The icon is served from `assets/`, which vite copies to the site root
/// untouched, and it is the same artwork the favicon is, at the size this draws
/// it.
///
/// No alt text on it, because the word it stands beside is the alt text: a
/// screen reader that read both would say the name twice.
///
/// Collapsed in the sidebar's strip, the logo is all of it: the word and the
/// gear keep their room, so the row stands as tall as it does expanded and the
/// list under it starts where it always starts, and are not drawn — the gear's
/// open fill on the settings with them — though the heading still reads
/// *Verkstead* and Tab still stops on the gear. See `Wordmark.module.css`.
///
/// No way back either, this being the level every other pane is entered from —
/// which is what makes it a title the head is handed a class for rather than one
/// of the head's own: see the `heading` prop in `PaneHead.tsx`.
export function Wordmark(): JSX.Element {
  return (
    <PaneHead
      heading={styles.wordmark}
      title={
        <>
          <img src="/icons/icon-192.png" alt="" />
          <span class={styles.word}>Verkstead</span>
        </>
      }
    >
      <Settings />
    </PaneHead>
  );
}

/// The way to the rest of Verkstead, which is one page: the Repos and the Agent
/// Profiles a Conversation is settled against, and what Verkstead itself has
/// been told. What is waiting on the human is not there — a Question Set is
/// reached through the Conversation it was asked from, which is the list this
/// sits over.
///
/// At the head of the pane, where the ⋯ that held it was and where a link at
/// the foot of the list was before that. That foot is under the conversations,
/// and the conversations are the one part of the sidebar with no end: a long
/// enough list and the way out to the settings was somewhere the human had to
/// scroll to find.
///
/// An [`IconButton`](../IconButton.tsx) rather than a menu of one row, because
/// a menu of one row is a press with a press in front of it — and because this
/// is the same kind of thing the cards below it are: something in this pane
/// that is selected and opened into the pane beside it. So it is drawn as open
/// while the settings are what is being read, which is what the open card in
/// the list says about itself, in the same fill.
///
/// A gear, which is what a settings icon is everywhere, and the label is the
/// whole of what a screen reader gets: the shape says nothing when it is read
/// aloud.
function Settings(): JSX.Element {
  const navigate = useNavigate();
  const where = useLocation();

  /// Open while the settings are what the human is looking at, whichever of
  /// their panes they are in: everything the settings open into is a path
  /// under this one.
  const open = (): boolean =>
    where.pathname === "/settings" || where.pathname.startsWith("/settings/");

  return (
    <IconButton
      class={styles.settings}
      of={faGear}
      label="Settings"
      open={open()}
      press={() => navigate("/settings")}
    />
  );
}
