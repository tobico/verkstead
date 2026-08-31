//! What the details pane has open, and the path it stands at.
//!
//! The pane shows one thing, and which thing it is, is in the URL: a details
//! pane has a path of its own, nested under the Conversation it belongs to.
//! One account of what is open rather than two — a selection held beside the
//! URL is lost the moment the page is navigated away from and back, and a link
//! to a pane is a link to nothing.
//!
//! Four shapes under a Conversation, because there are four kinds of thing the
//! pane draws:
//!
//! - `events/:id` — a Timeline Event with a full self.
//! - `backlog` — the backlog, there being one per Conversation.
//! - `roadmaps/:name` — a roadmap, by the directory name that is its identity.
//! - `share` — sharing the Conversation, there being one of that per
//!   Conversation as well.
//!
//! The `events/` segment is what keeps the ids and the word-named panes apart.
//! A bare `:event` segment would have read the same as `backlog` the moment
//! anything was named by a word, so the ids are put behind a segment of their
//! own and can never collide with one.
//!
//! A path naming something the loaded Conversation does not have leaves the
//! pane empty, which is what the pane is when nothing is open at all: the URL
//! is a record of what was picked rather than a promise that it is still there.
//!
//! And which Event opens which pane is here too — see [`openingOf`] — because it
//! is the same question the paths are about, asked of an Event rather than of a
//! path: what the details pane can be showing. The Timeline's own cards have no
//! need of it, each of them being drawn for a kind it already knows; what does
//! is picking the end of a record, where the kind is whatever the last Event
//! turned out to be.

import type { TimelineEvent } from "../api/types";

/// What the details pane is showing, as the card that opened it names itself.
///
/// An event's id, for the kinds of event that have a full self to open. And a
/// word for the two plan cards, which have none: each is read off the worktree
/// every time the conversation is, so the row that says where it landed fixes a
/// position rather than an identity. The backlog is the bare word, there being
/// one per conversation; a roadmap carries its own name after it, a worktree
/// being allowed any number of those.
///
/// And a word for the Share pane, which no card opens at all: it is the icon
/// button on the Timeline's header that opens it, and there is one share of a
/// Conversation the way there is one backlog of it.
///
/// One channel for all of them, so that opening any closes the rest — a details
/// pane shows one thing. A string rather than an object for the same reason:
/// what is open is compared against what a card would open, and two of the same
/// selection have to be the same value.
export type Opening = number | "backlog" | "share" | `roadmap:${string}`;

/// What opens the named roadmap, by the directory name that is its identity.
export function opensRoadmap(name: string): Opening {
  return `roadmap:${name}`;
}

/// And which roadmap an opening names, or `null` where it names none.
export function roadmapOpened(opening: Opening | null): string | null {
  return typeof opening === "string" && opening.startsWith("roadmap:")
    ? opening.slice("roadmap:".length)
    : null;
}

/// Where a Conversation stands, which is what every one of its details panes is
/// nested under.
export function pathOf(conversation: string | number): string {
  return `/conversations/${encodeURIComponent(String(conversation))}`;
}

/// And where one of its details panes stands.
export function pathTo(
  conversation: string | number,
  opening: Opening,
): string {
  const under = pathOf(conversation);

  if (opening === "backlog" || opening === "share") {
    return `${under}/${opening}`;
  }

  const roadmap = roadmapOpened(opening);
  if (roadmap !== null) {
    return `${under}/roadmaps/${encodeURIComponent(roadmap)}`;
  }

  return `${under}/events/${opening}`;
}

/// What a path says is open, or `null` where it names no details pane at all —
/// the Conversation's own path, the bare workbench, or any other page.
///
/// Read off the path rather than off the router's params, because two of these
/// routes carry no parameter of their own: `backlog` is the segment itself, and
/// what tells a Conversation from a Conversation with its backlog open is what
/// stands after the id.
export function openingAt(pathname: string): Opening | null {
  const segments = pathname.split("/").filter((segment) => segment !== "");
  if (segments[0] !== "conversations") {
    return null;
  }

  // What stands after the Conversation's id, which is a segment and at most one
  // thing named by it. Anything longer is no path of ours, whatever it starts
  // with.
  const [what, which, ...rest] = segments.slice(2);
  if (rest.length > 0) {
    return null;
  }

  if ((what === "backlog" || what === "share") && which === undefined) {
    return what;
  }

  if (which === undefined) {
    return null;
  }

  if (what === "roadmaps") {
    return opensRoadmap(decodeURIComponent(which));
  }

  // Digits and nothing else, because an id is what the server issued. A segment
  // that is not one names no Event, and neither does an id the Conversation has
  // not got: both leave the pane empty, which is the same answer as a stale
  // selection's.
  if (what === "events" && /^\d+$/.test(which)) {
    return Number(which);
  }

  return null;
}

/// What a Timeline Event opens in the details pane, or `null` where it opens
/// nothing.
///
/// Which kinds have a pane behind them, asked of an Event whose kind is not
/// known yet — the Timeline draws a card per kind and needs no such question,
/// and picking the end of a record is nothing but it.
///
/// Three of them answer for themselves rather than by their kind: the Brief
/// opens only once it has frozen — while it is being written the card is a field
/// with the conversation's setup under it — a steer opens only where it carried
/// a document, and the backlog only where there is still one to read. A move, a
/// manual task and a steer into wrapping up have nothing to show at all, and
/// each is drawn as a line rather than as a card for that reason.
///
/// The two lists open by their word rather than by the row's id, being read off
/// the worktree rather than off the record. A row that landed several roadmaps
/// draws a card apiece, and the last of them is the last card of that row.
export function openingOf(event: TimelineEvent): Opening | null {
  if ("Brief" in event) {
    return event.Brief.frozen ? event.Brief.id : null;
  }

  if ("Steer" in event) {
    return event.Steer.html === null ? null : event.Steer.id;
  }

  if ("TaskList" in event) {
    return event.TaskList.list === null ? null : "backlog";
  }

  if ("StageList" in event) {
    const last = event.StageList.roadmaps.at(-1);
    return last === undefined ? null : opensRoadmap(last.name);
  }

  if ("AgentOutput" in event) return event.AgentOutput.id;
  if ("QuestionSet" in event) return event.QuestionSet.id;
  if ("UnreadableSet" in event) return event.UnreadableSet.id;
  if ("Handoff" in event) return event.Handoff.id;
  if ("Commit" in event) return event.Commit.id;
  if ("Notice" in event) return event.Notice.id;
  if ("PullRequest" in event) return event.PullRequest.id;

  return null;
}

/// The end of a record: the last thing on a Timeline that has a pane behind it,
/// or `null` where none of it has one.
///
/// What opening a Conversation lands on. The last *openable* Event rather than
/// the last Event, because a record very often ends on a move — every step of
/// the ladder writes one — and landing on a row with nothing behind it would
/// leave the pane bare at the exact moment the human asked to be shown where
/// the work got to.
///
/// A record with nothing openable on it at all — a Draft with only the Brief
/// being written — selects nothing, and the pane stays bare paper.
export function lastOpening(timeline: readonly TimelineEvent[]): Opening | null {
  for (let at = timeline.length - 1; at >= 0; at -= 1) {
    const opening = openingOf(timeline[at]!);
    if (opening !== null) {
      return opening;
    }
  }

  return null;
}
