//! Mermaid itself, built to one script a share can carry.
//!
//! A share fetches nothing, and mermaid is the one thing on a Set's page the
//! browser draws for itself — so where the record carries a Diagram, the
//! library has to be *inside* the file. It cannot ride in the app's own chunk:
//! that chunk is in every share, and mermaid is megabytes, so a share of a
//! Conversation nobody drew a picture in would be twenty times the size of what
//! it is carrying.
//!
//! So it is built on its own (`vite.mermaid.config.ts`), and the server puts it
//! in the document only when a Set in the bundle says it has a Diagram — see
//! `crates/server/src/sharing.rs`. This is that build's whole entry: it hangs
//! the renderer where the share's own stub goes looking for it, which is the
//! seam between the two.

import mermaid from "mermaid";

import { GLOBAL } from "./mermaid";

(window as unknown as Record<string, unknown>)[GLOBAL] = mermaid;
