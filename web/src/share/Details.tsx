//! The details pane of a share: the full self of whatever the record has open.
//!
//! The workbench's own pane and this one draw the same panes from the same
//! components, and they differ in one thing: where the material comes from. The
//! workbench fetches — a Set, a commit's diff, a session's Capture are each a
//! request made when somebody opens one — and a share fetches nothing, because
//! there is nothing to fetch from. So this is the switch written against what a
//! share carries rather than against every kind a Timeline can hold.
//!
//! Which is a shorter list, and it is the record's rather than this file's: the
//! kinds that do not board a share never reach here, so what is left is the two
//! documents that arrive rendered — the Brief, and whatever a steer sent a
//! session off with — and the two that a share carries payloads for, the
//! Question Sets and the commits.
//!
//! Those two are stubs for now. The record boards them and the cards open, and
//! what the pane draws is a sentence saying the material is not in this build's
//! share; the tasks after this one put the sheet and the diff in the bundle and
//! draw them here with the live components.

import { Match, Show, Switch, type JSX } from "solid-js";

import type {
  BriefEvent,
  CommitEvent,
  ConversationView,
  QuestionSetEvent,
  SteerEvent,
} from "../api/types";
import { Empty } from "../notices";
import { Brief } from "../workbench/Brief";
import { Document } from "../workbench/Document";
import { PaneHead } from "../workbench/PaneHead";
import type { Opening } from "../workbench/openings";

/// An Event a share opens, and which kind it turned out to be.
type Opened =
  | { brief: BriefEvent }
  | { steer: SteerEvent }
  | { asked: QuestionSetEvent }
  | { commit: CommitEvent };

export function Details(props: {
  conversation: ConversationView;
  /// What the record has open, if anything.
  event: Opening | null;
  /// And the way back to it, which is what a narrow window walks out through.
  back: () => void;
}): JSX.Element {
  /// The Event the pane is showing, where the record holds one under that id.
  ///
  /// An id naming nothing leaves the pane empty, which is what it is when
  /// nothing is open at all — the same answer the workbench gives, and for the
  /// same reason: what is open is a record of what was picked rather than a
  /// promise that it is still there.
  const opened = (): Opened | undefined =>
    props.conversation.timeline
      .map((event): Opened | undefined => {
        if ("Brief" in event) {
          return { brief: event.Brief };
        }
        // Only where it carries a document. A steer into wrapping up says
        // nothing but the state, and the record draws one of those as a line.
        if ("Steer" in event && event.Steer.html !== null) {
          return { steer: event.Steer };
        }
        if ("QuestionSet" in event) {
          return { asked: event.QuestionSet };
        }
        if ("Commit" in event) {
          return { commit: event.Commit };
        }
        return undefined;
      })
      .find((open) => open !== undefined && which(open) === props.event);

  return (
    <Show when={opened()}>
      {(open) => (
        <Switch>
          <Match when={briefIn(open())}>
            {(brief) => (
              <Brief
                conversation={props.conversation}
                brief={brief()}
                back={props.back}
              />
            )}
          </Match>
          {/* What a steer sent a session off with, read the way every other
              document the human writes is read — and named after the target,
              an instruction being one session's whole job and a follow-up's
              brief being what a conversation was opened on. */}
          <Match when={steerIn(open())}>
            {(steer) => (
              <Document
                heading={
                  steer().target === "FollowUp" ? "Follow-up" : "Instruction"
                }
                html={steer().html ?? ""}
                empty="Nothing was asked for."
                back={props.back}
              />
            )}
          </Match>
          <Match when={setIn(open())}>
            <Missing heading="Question set" back={props.back}>
              This build's shares carry the record of a question set rather than
              the document it was asked as.
            </Missing>
          </Match>
          <Match when={commitIn(open())}>
            <Missing heading="Commit" back={props.back}>
              This build's shares carry what a commit changed rather than its
              diff.
            </Missing>
          </Match>
        </Switch>
      )}
    </Show>
  );
}

/// A pane whose material this build's share does not carry: the header it would
/// have had, and a sentence saying so where the document would be.
///
/// Said rather than left blank, and said in terms of the share rather than of
/// the record: nothing went missing — the moment is on the Timeline and its card
/// says what happened — so what the reader is owed is which of the two they are
/// holding.
function Missing(props: {
  heading: string;
  back: () => void;
  children: JSX.Element;
}): JSX.Element {
  return (
    <>
      <PaneHead back={{ to: "Timeline", go: props.back }} title={props.heading} />
      <Empty>{props.children}</Empty>
    </>
  );
}

/// The id an opened Event is reached by, whichever kind it turned out to be.
function which(open: Opened): number {
  if ("brief" in open) {
    return open.brief.id;
  }
  if ("steer" in open) {
    return open.steer.id;
  }
  return "asked" in open ? open.asked.id : open.commit.id;
}

/// And each kind on its own, for the pane that draws it.
function briefIn(open: Opened): BriefEvent | undefined {
  return "brief" in open ? open.brief : undefined;
}

function steerIn(open: Opened): SteerEvent | undefined {
  return "steer" in open ? open.steer : undefined;
}

function setIn(open: Opened): QuestionSetEvent | undefined {
  return "asked" in open ? open.asked : undefined;
}

function commitIn(open: Opened): CommitEvent | undefined {
  return "commit" in open ? open.commit : undefined;
}
