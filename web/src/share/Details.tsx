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
//! The Question Sets are drawn from the sheets the bundle carries — the same
//! [`Sheet`] the workbench opens one with, asked for its read-only shape: the
//! answering surface goes, and what is left is the record however the Set stood
//! when the share was taken.
//!
//! The commits are the same arrangement one level along: the bundle carries the
//! pane the workbench would have fetched — the Message, and the whole diff with
//! its folds and its colours already in it — and `Opened` draws it, which is the
//! component the workbench's own commit pane is. A commit the repository had
//! already lost when the share was taken travels as what the store kept, and
//! says where its diff went rather than reading as a commit that changed
//! nothing.

import { Match, Show, Switch, type JSX } from "solid-js";

import type {
  BriefEvent,
  CommitEvent,
  ConversationView,
  QuestionSetEvent,
  SetView,
  SharedCommit,
  SteerEvent,
} from "../api/types";
import { Empty } from "../notices";
import { Sheet } from "../set/Sheet";
import { Brief } from "../workbench/Brief";
import { Opened } from "../workbench/Commit";
import { Document } from "../workbench/Document";
import { PaneHead } from "../workbench/PaneHead";
import type { Opening } from "../workbench/openings";

/// An Event a share opens, and which kind it turned out to be.
type Open =
  | { brief: BriefEvent }
  | { steer: SteerEvent }
  | { asked: QuestionSetEvent }
  | { commit: CommitEvent };

export function Details(props: {
  conversation: ConversationView;
  /// The sheets the share is carrying, one per Question Set on that Timeline.
  sets: SetView[];
  /// And the panes behind the commits, one per commit on it.
  commits: SharedCommit[];
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
  const opened = (): Open | undefined =>
    props.conversation.timeline
      .map((event): Open | undefined => {
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
                readOnly
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
          {/* The whole sheet, drawn by the component the workbench opens a
              Set with and asked for its read-only shape: the Preface, the
              Diff it was asked over, every Question with its Options and what
              was decided, and the Postscript. With its own pane header and its
              table of contents, exactly as the workbench draws one. */}
          <Match when={setIn(open())}>
            {(asked) => (
              <Show
                when={sheet(props.sets, asked().set_id)}
                fallback={
                  <Missing heading="Question set" back={props.back}>
                    This share is not carrying the sheet this set was asked as.
                  </Missing>
                }
              >
                {(set) => <Sheet set={set()} back={props.back} readOnly />}
              </Show>
            )}
          </Match>
          {/* The commit's own pane, drawn by the component the workbench opens
              one with: what it was called and how much it moved, the Message it
              wrote about itself, and the whole diff folded per file. Read out
              of the file rather than fetched — the export read it out of git,
              and there is nothing here to read anything with. */}
          <Match when={commitIn(open())}>
            {(commit) => (
              <Show
                when={carried(props.commits, commit().id)}
                fallback={
                  <Missing heading="Commit" back={props.back}>
                    This share is not carrying the pane this commit opens.
                  </Missing>
                }
              >
                {(shared) => (
                  <Opened
                    commit={commit()}
                    pane={shared().pane}
                    back={props.back}
                    instead={
                      shared().held ? (
                        <Empty>This commit changed no files.</Empty>
                      ) : (
                        <Empty>
                          The repository no longer had this commit when the
                          share was taken, so its diff could not travel with it.
                        </Empty>
                      )
                    }
                  />
                )}
              </Show>
            )}
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

/// The sheet the share carries for one Set, and nothing where it carries none.
///
/// A miss should be unreachable — every Question Set that boards is rendered
/// into the bundle beside it — so what it means is a share written by something
/// that disagrees with this build, and the pane says so rather than drawing an
/// empty document.
function sheet(sets: SetView[], id: number): SetView | undefined {
  return sets.find((set) => set.id === id);
}

/// The pane the share carries for one commit, and nothing where it carries
/// none.
///
/// A miss should be unreachable for the reason a missing sheet is: every commit
/// that boards is rendered into the bundle beside it. A commit git had lost is
/// not this — it is carried, with the flag beside it saying the diff is not.
function carried(commits: SharedCommit[], id: number): SharedCommit | undefined {
  return commits.find((commit) => commit.id === id);
}

/// The id an opened Event is reached by, whichever kind it turned out to be.
function which(open: Open): number {
  if ("brief" in open) {
    return open.brief.id;
  }
  if ("steer" in open) {
    return open.steer.id;
  }
  return "asked" in open ? open.asked.id : open.commit.id;
}

/// And each kind on its own, for the pane that draws it.
function briefIn(open: Open): BriefEvent | undefined {
  return "brief" in open ? open.brief : undefined;
}

function steerIn(open: Open): SteerEvent | undefined {
  return "steer" in open ? open.steer : undefined;
}

function setIn(open: Open): QuestionSetEvent | undefined {
  return "asked" in open ? open.asked : undefined;
}

function commitIn(open: Open): CommitEvent | undefined {
  return "commit" in open ? open.commit : undefined;
}
