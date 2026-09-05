//! Dragging files over a box, which a browser does with a `DataTransfer` and
//! jsdom does not do at all — see `src/Attaching.tsx` for what reads one.
//!
//! Written here rather than at each of the places a drop is asked about, for
//! the reason the listbox's own driving is written down: what a drag carries is
//! four events and an object no test environment builds, and a test spelling
//! that out is a test asserting against its own scaffolding.
//!
//! What is faked is exactly what a browser hands over: `types`, which is how a
//! page tells a drag carrying files from one carrying a selection of text, and
//! `items`, which is the only place a browser says which of the things dropped
//! was a folder. A drop carrying no folder is offered `files` as well, that
//! being the whole of what a page needs from one.

import { fireEvent } from "@solidjs/testing-library";

/// What a drag is carrying: the files in it, and the names of any folders that
/// were dragged along with them.
export type Carried = { files?: File[]; folders?: string[] };

/// One drag's `DataTransfer`, as far as anything reading one can tell.
///
/// A folder arrives as an entry that says it is a directory and a `File` that
/// looks like any other, which is the shape the browser really hands over: what
/// says the two apart is `webkitGetAsEntry`, and nothing else does.
export function carrying({ files = [], folders = [] }: Carried): DataTransfer {
  const entry = (isDirectory: boolean) => ({ isDirectory });

  const items = [
    ...files.map((file) => ({
      kind: "file",
      getAsFile: () => file,
      webkitGetAsEntry: () => entry(false),
    })),
    ...folders.map((name) => ({
      kind: "file",
      getAsFile: () => new File([], name),
      webkitGetAsEntry: () => entry(true),
    })),
  ];

  return {
    types: ["Files"],
    items,
    files,
    dropEffect: "none",
  } as unknown as DataTransfer;
}

/// And a drag carrying something that is not a file at all — a selection of
/// text, a link — which is a drag every box ignores.
export function carryingNothing(): DataTransfer {
  return {
    types: ["text/plain"],
    items: [],
    files: [],
    dropEffect: "none",
  } as unknown as DataTransfer;
}

/// One event of a drag, over the element it is over.
///
/// Built by hand rather than through `fireEvent.drop`: jsdom has no `DragEvent`
/// and no `DataTransfer`, so the transfer is defined onto a plain event — which
/// is all a handler ever reads off one.
export function drag(
  over: Element,
  kind: "dragenter" | "dragover" | "dragleave" | "drop",
  carried: DataTransfer,
): Event {
  const event = new Event(kind, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: carried });

  fireEvent(over, event);

  return event;
}

/// A whole drop: in over the box, and let go there.
export function dropOn(over: Element, carried: DataTransfer): void {
  drag(over, "dragenter", carried);
  drag(over, "dragover", carried);
  drag(over, "drop", carried);
}
