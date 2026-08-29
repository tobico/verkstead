//! What the details pane has open, and the path it stands at.
//!
//! The pane shows one thing, and which thing it is, is in the URL: a details
//! pane has a path of its own, nested under the Conversation it belongs to.
//! One account of what is open rather than two — a selection held beside the
//! URL is lost the moment the page is navigated away from and back, and a link
//! to a pane is a link to nothing.
//!
//! Three shapes under a Conversation, because there are three kinds of thing
//! the pane draws:
//!
//! - `events/:id` — a Timeline Event with a full self.
//! - `backlog` — the backlog, there being one per Conversation.
//! - `roadmaps/:name` — a roadmap, by the directory name that is its identity.
//!
//! The `events/` segment is what keeps the ids and the word-named panes apart.
//! A bare `:event` segment would have read the same as `backlog` the moment
//! anything was named by a word, so the ids are put behind a segment of their
//! own and can never collide with one.
//!
//! A path naming something the loaded Conversation does not have leaves the
//! pane empty, which is what the pane is when nothing is open at all: the URL
//! is a record of what was picked rather than a promise that it is still there.

/// What the details pane is showing, as the card that opened it names itself.
///
/// An event's id, for the kinds of event that have a full self to open. And a
/// word for the two plan cards, which have none: each is read off the worktree
/// every time the conversation is, so the row that says where it landed fixes a
/// position rather than an identity. The backlog is the bare word, there being
/// one per conversation; a roadmap carries its own name after it, a worktree
/// being allowed any number of those.
///
/// One channel for all of them, so that opening any closes the rest — a details
/// pane shows one thing. A string rather than an object for the same reason:
/// what is open is compared against what a card would open, and two of the same
/// selection have to be the same value.
export type Opening = number | "backlog" | `roadmap:${string}`;

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

  if (opening === "backlog") {
    return `${under}/backlog`;
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

  if (what === "backlog" && which === undefined) {
    return "backlog";
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
