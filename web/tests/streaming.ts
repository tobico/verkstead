//! The server's Nudge stream, stood in for: jsdom has no `EventSource`, and a
//! test would want its own of one anyway — there is no other way to put a Nudge
//! on the wire or to sever the connection carrying it (ADR-0009).
//!
//! Here rather than in the one file that drives it, because two do: the Nudge's
//! own tests, which ask which reads each kind causes, and the workbench's, where
//! the Code pane's tree follows the disk on a `files` Nudge rather than on a
//! query being invalidated.

import { vi } from "vitest";

/// A stand-in for the browser's `EventSource`.
export class Streaming {
  /// Every stream the page has opened, newest last.
  static opened: Streaming[] = [];

  private readonly listeners = new Map<string, Array<(event: Event) => void>>();
  closed = false;

  constructor(readonly url: string) {
    Streaming.opened.push(this);
  }

  addEventListener(name: string, listener: (event: Event) => void): void {
    this.listeners.set(name, [...(this.listeners.get(name) ?? []), listener]);
  }

  close(): void {
    this.closed = true;
  }

  /// What the browser does when the connection is established — on the first
  /// one and on every reconnect after it, which is the whole of how a page
  /// finds out it was away.
  opens(): void {
    this.fire("open");
  }

  /// One Nudge, as the server writes it: a named event whose data says what
  /// moved. `said` is passed through untouched, so a test may put something
  /// down the wire that no page could read.
  nudges(said: unknown): void {
    const data = typeof said === "string" ? said : JSON.stringify(said);

    for (const listener of this.listeners.get("nudge") ?? []) {
      listener(new MessageEvent("nudge", { data }));
    }
  }

  private fire(name: string): void {
    for (const listener of this.listeners.get(name) ?? []) {
      listener(new Event(name));
    }
  }
}

/// Put it where the browser's would be, with nothing opened yet.
///
/// Taken away again by `vi.unstubAllGlobals()`, which every suite that calls
/// this already runs after each test.
export function streaming(): void {
  Streaming.opened = [];
  vi.stubGlobal("EventSource", Streaming);
}

/// The stream the page opened, newest first — which is the one it is listening
/// on. There is one at a time and not one per page: the connection is given
/// back whenever the page is hidden and taken again when it is looked at, so a
/// page that has been away has opened more than one over its life.
export function stream(): Streaming {
  const opened = Streaming.opened.at(-1);

  if (!opened) {
    throw new Error("the page opened no stream");
  }

  return opened;
}
