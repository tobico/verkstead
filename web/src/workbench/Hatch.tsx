//! The way out of a Conversation whose page will not load.
//!
//! Everything the workbench offers about a Conversation as a whole hangs off one
//! read — `GET /api/ui/conversations/{id}`, which the pane and the sidebar's
//! right-click both draw their rows from. That read has half a dozen ways to
//! fail: a corrupt Timeline row, a Pairing that will not resolve, a worktree
//! look that throws, the store itself. When it does, the pane used to draw one
//! line of error and nothing else — so the Conversation the human most wanted to
//! be rid of was the one with no way to end it.
//!
//! The presses themselves were never the problem. Close, close-and-archive and
//! archive are POSTs that need nothing but the Conversation's id; it is only the
//! *menu* that needs the whole reading. So this is that menu drawn without it: a
//! degraded header carrying the same ⋯, with the one row that gets the human out.
//!
//! **One row, not four.** Which one is read off the sidebar's list — the query
//! the page already has in hand — and the reading only decides the label. A
//! Closed Conversation is offered **Archive**; anything else, and anything this
//! cannot tell, is offered **Close and archive**, which covers both: close
//! refuses nothing but a Conversation that is gone, and an already-closed one
//! still archives. Guessing *not closed* is therefore always safe, and guessing
//! *closed* would not be — Archive on a Conversation that is not closed answers
//! `NotClosed` and goes nowhere.
//!
//! **A press that lands leaves the page**, unlike the ordinary menu's, which
//! only reads the world again and stays where it is. There is nothing here to
//! stay for: the page could not be read before the press and will not be read
//! after it, so what a re-read would draw is the same error over a Conversation
//! that is now off the list. On a narrow window that is the way back to the
//! conversations, and on a wide one the empty pane — one navigation, because
//! they are the same one.
//!
//! **And it leaves at the press rather than at the answer.** Both rows here are
//! the ordinary menu's eager ones — see `eager.ts` — so the row goes off the
//! sidebar and the page goes home the moment the human presses, and the close
//! runs behind them. There is no page left to roll back to: what a failure
//! takes back is the row, which quietly reappears in the list, and what says
//! why is a toast.
//!
//! Which matters more here than anywhere else it is done. Everywhere else a
//! refusal is a page drawn against a Conversation that has moved, and the
//! re-read behind the press is the correction; here there is no re-read to make
//! and no row to correct, because the reading is the thing that failed — so a
//! press that went quietly nowhere would leave the human on a page that will
//! not load, with the one way off it apparently doing nothing.

import { useQueryClient } from "@tanstack/solid-query";
import { Show, type JSX } from "solid-js";

import { Menu } from "../Menu";
import {
  archiveConversation,
  closeAndArchiveConversation,
  listConversations,
} from "../api/client";
import type { ConversationArchived, ConversationClosed } from "../api/types";
import { useReading } from "../freshness";
import { ARCHIVE_REFUSAL, Action, CLOSE_REFUSAL } from "./Actions";
import styles from "./Actions.module.css";
import { eagerly, type Said } from "./eager";
import { PaneHead } from "./PaneHead";

/// The degraded header and the hatch under its ⋯, for the Conversation the pane
/// could not read.
export function Hatch(props: {
  /// The Conversation, as the URL named it — which is all there is: the reading
  /// that would have carried anything else is the one that failed.
  id: string;

  /// The way back out to the list, which is both what the header's back button
  /// does on a narrow window and where a press that landed leaves the human.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  // The menu's own way to shut, held here because the press is what shuts it.
  let shut = (): void => {};

  /// The sidebar's own list, read under the key the sidebar reads it under — so
  /// this is the answer already in hand rather than a second fetch of it.
  ///
  /// Nothing here fails if it is absent. What it decides is which of the two
  /// rows to draw, and *not in hand* falls the same way *not closed* does.
  const conversations = useReading(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,

    // Merged as the sidebar's is: the list is re-read constantly, and this is
    // the same cache entry.
    freshness: { reconcile: "id" } as const,
  }));

  const row = () =>
    conversations.data?.find((entry) => String(entry.id) === props.id);

  /// The Conversation's id as the presses want it. The path is what the human's
  /// URL held: one that names no Conversation is answered by the server the way
  /// one that names a Conversation that has gone is.
  const id = () => Number(props.id);

  /// Reading back what the press left stale, which is what lets go of the row
  /// it took off the list — see `eager.ts`. The Conversation's own read is
  /// among them although it is the read that failed: a press that closed it may
  /// be exactly what makes it readable again, and either way the answer to it
  /// is not this page's to keep.
  const reread = (): Promise<unknown> =>
    Promise.all([
      queries.invalidateQueries({ queryKey: ["conversation"] }),
      queries.invalidateQueries({ queryKey: ["conversations"] }),
    ]);

  /// What both presses here do: the menu goes, the row goes off the sidebar,
  /// the page goes home, and the request runs behind all three. A failure puts
  /// the row back and says why in a toast — see the module's own note above.
  const leaving = <Outcome,>(press: {
    says: Said;
    post: () => Promise<Outcome>;
    refusal: (outcome: Outcome) => string;
    fell: (error: Error) => string;
  }): void => {
    shut();
    eagerly({ conversation: id(), reread, ...press });
    props.back();
  };

  const closeAway = () =>
    leaving({
      says: { closed: true, archived: true },
      post: () => closeAndArchiveConversation(id()),
      refusal: (outcome: ConversationClosed) => CLOSE_REFUSAL[outcome],
      fell: (error: Error) =>
        `The conversation could not be closed: ${error.message}`,
    });

  const archive = () =>
    leaving({
      says: { archived: true },
      post: () => archiveConversation(id()),
      refusal: (outcome: ConversationArchived) => ARCHIVE_REFUSAL[outcome],
      fell: (error: Error) =>
        `The conversation could not be archived: ${error.message}`,
    });

  return (
    <>
      <PaneHead
        // The way back out of a level a narrow window has no other way out of.
        // Drawn always and hidden where all three panes stand at once, as every
        // pane's is — and the whole reason the hatch is on the pane rather than
        // on the sidebar's right-click, which a phone has no way to open at all.
        back={{ to: "Conversations", go: props.back }}
        // The branch, where the sidebar knows it, so the human can see this is
        // the Conversation they meant. Where it does not, the id it was asked
        // for: a header with no name at all would be one more thing that will
        // not load.
        title={row()?.branch ?? `Conversation ${props.id}`}
      >
        <Menu
          class={styles.conversationActions!}
          label="Conversation actions"
          name="Conversation actions"
          closer={(close) => (shut = close)}
          mark
        >
          {() => (
            <Show
              when={row()?.state === "Closed"}
              fallback={
                <Action
                  class={styles.closeAndArchive}
                  label="Close and archive"
                  says="Permanently end the conversation, delete the worktree, and take it off the conversations list. The branch stays where it is."
                  press={closeAway}
                />
              }
            >
              <Action
                class={styles.archive}
                label="Archive"
                says="Take it off the conversations list. Its record stays where it is."
                press={archive}
              />
            </Show>
          )}
        </Menu>
      </PaneHead>
    </>
  );
}
