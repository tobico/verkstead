//! What `import("mermaid")` means inside a share: the library the document is
//! already carrying, rather than one to fetch.
//!
//! The share build aliases the package to this module (`vite.share.config.ts`),
//! and the reason is size. That build folds every dynamic import into its one
//! chunk — there is nothing to fetch a second one with — so an `import("mermaid")`
//! left as itself would put the whole renderer in every share ever made.
//! Aliased, what goes in the chunk is this file, and mermaid rides separately in
//! the one share that has a Diagram to draw: see `mermaid-library.ts` and the
//! slot the server writes it into.
//!
//! A thenable rather than a `Promise`, and that is the whole of the care here:
//! the chunk's modules are evaluated when the script runs, and a `Promise` built
//! then would look for the library before the document had finished handing it
//! over. This looks when it is awaited, which is the moment a Set with a Diagram
//! on it is drawn.

import type { Renderer } from "../set/diagrams";

/// Where the library hangs when a share is carrying one. Said on both sides of
/// the seam, as the record's own slot is.
export const GLOBAL = "verksteadMermaid";

/// The renderer this document has, refusing where it has none.
///
/// Refusing rather than answering with nothing, because a bundle that does not
/// arrive is a case the drawing already handles: every Diagram stays the source
/// block the markdown renderer wrote, which is a readable page rather than a
/// broken one. It should be unreachable — a Set with no Diagram never asks — so
/// this is what a build that got the slot wrong degrades to.
const carried: PromiseLike<Renderer> = {
  then(resolve, reject) {
    const renderer = (
      window as unknown as Record<string, Renderer | undefined>
    )[GLOBAL];

    return renderer === undefined
      ? Promise.reject<never>(
          new Error("this share is not carrying the diagram renderer"),
        ).then(resolve, reject)
      : Promise.resolve(renderer).then(resolve, reject);
  },
};

export default carried;
