//! A shared Conversation, drawn as the workbench draws one — minus the half of
//! the workbench a reader has no use for.
//!
//! Two panes rather than three: the record on one side and whatever it has open
//! on the other. There is no conversations pane because there are no
//! conversations — a share is one piece of work, and a list with one row in it
//! that goes nowhere would be furniture. The frame is the same frame all the
//! same (`Panes.tsx`), so a phone walks through the two levels exactly as it
//! walks through the workbench's three.
//!
//! Everything else is the workbench's own components, handed the record the file
//! carries. That is the point of the whole arrangement: what a colleague reads
//! is what the human sees, rather than a second rendering that would drift.
//!
//! Nothing here fetches, and nothing here can. What is drawn is what boarded
//! (`crates/render/src/sharing.rs`) — a record with every field a control hangs
//! off already saying nothing — and the Timeline is asked for its read-only
//! shape on top of that, which takes off the menu, the block that says what
//! happens next, and the way back to a list that does not exist.

import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { Show, createEffect, createSignal, type JSX } from "solid-js";

import { Panes, type Pane } from "../Panes";
import type { SharedConversation } from "../api/types";
import { Empty } from "../notices";
import { Timeline } from "../workbench/Timeline";
import { titled } from "../workbench/naming";
import { lastOpening, type Opening } from "../workbench/openings";
import { Details } from "./Details";

/// A client for the cards that ask for one.
///
/// The Timeline's Brief card holds a mutation — it is a field while a
/// Conversation drafts — and a mutation wants a client whether or not anything
/// ever fires one. Nothing in a share does: every Brief boards frozen, so the
/// card is a rendering, and this exists so that the component the workbench and
/// the share both draw needs no second version of itself.
const queries = new QueryClient();

export function Share(props: {
  /// What the file was carrying, or `null` where it was carrying nothing —
  /// the untouched template, or a slot that would not parse.
  shared: SharedConversation | null;
}): JSX.Element {
  return (
    <QueryClientProvider client={queries}>
      <Show
        when={props.shared}
        fallback={
          <Empty>
            This file is not carrying a conversation. Ask whoever sent it for
            another.
          </Empty>
        }
      >
        {(shared) => <Record shared={shared()} />}
      </Show>
    </QueryClientProvider>
  );
}

/// The record itself, once there is one.
function Record(props: { shared: SharedConversation }): JSX.Element {
  const conversation = () => props.shared.conversation;

  /// Which level a narrow window is showing, and what the record has open.
  ///
  /// Held here rather than read off a URL, which is what the workbench does: a
  /// share is one page in one file, opened from a disk as often as from a
  /// server, and a router pushing paths into a `file://` history would be
  /// spelling out a place nobody can come back to.
  const [pane, setPane] = createSignal<Pane>("middle");

  /// Opened at the end of the record, exactly as opening a Conversation in the
  /// workbench is: the last thing on the Timeline with a pane behind it, so a
  /// reader arrives at where the work got to rather than at the beginning of
  /// it.
  const [selected, setSelected] = createSignal<Opening | null>(
    lastOpening(conversation().timeline),
  );

  // And the tab says which Conversation this is. A share sits in a downloads
  // folder among other files and other tabs, and *Verkstead* would name every
  // one of them the same thing.
  createEffect(() => {
    document.title = `${titled(conversation())} — Verkstead`;
  });

  return (
    <Panes
      pane={pane()}
      middleLabel="Timeline"
      middle={
        <Timeline
          conversation={conversation()}
          details={() => setPane("details")}
          selected={selected()}
          select={setSelected}
          readOnly
        />
      }
      details={
        <Details
          conversation={conversation()}
          sets={props.shared.sets}
          event={selected()}
          back={() => setPane("middle")}
        />
      }
    />
  );
}
