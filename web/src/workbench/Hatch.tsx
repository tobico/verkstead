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
//! **A refused press is said in front of the human**, as the ordinary menu's
//! now is: the refusal's own sentence, in the same card over the page. It
//! matters more here than anywhere else it is drawn. Everywhere else a refusal
//! is a page drawn against a Conversation that has moved, and the re-read behind
//! the press is the correction; here there is no re-read to make and no row to
//! correct, because the reading is the thing that failed — so a press that went
//! quietly nowhere would leave the human on a page that will not load, with the
//! one way off it apparently doing nothing.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";

import { Menu } from "../Menu";
import {
  archiveConversation,
  closeAndArchiveConversation,
  listConversations,
} from "../api/client";
import type { ConversationArchived, ConversationClosed } from "../api/types";
import { useReading } from "../freshness";
import { ARCHIVE_REFUSAL, Action, CLOSE_REFUSAL, Refusal } from "./Actions";
import styles from "./Actions.module.css";
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

  /// The sentence a refused press is answered with, and `null` while there is
  /// nothing to answer. Held out here rather than in the menu's rows, because
  /// the rows are built when the menu opens and thrown away when it closes, and
  /// the card outlives the menu the press was made in: the press shuts it.
  const [refused, setRefused] = createSignal<string | null>(null);

  // The menu's own way to shut, held here because what closes it is the press
  // coming back rather than the press going out.
  let shut = (): void => {};

  /// What a press that did nothing comes to: the menu goes, handing the focus
  /// back to the ⋯ it was dropped from, and the sentence saying why opens over
  /// the page.
  const refuse = (sentence: string): void => {
    shut();
    setRefused(sentence);
  };

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

  /// What every press here does when it lands: take the list's word for it away,
  /// and leave.
  const leave = () => {
    void queries.invalidateQueries({ queryKey: ["conversation"] });
    void queries.invalidateQueries({ queryKey: ["conversations"] });
    props.back();
  };

  const closeAway = useMutation(() => ({
    mutationFn: (id: number) => closeAndArchiveConversation(id),
    onSuccess: (outcome: ConversationClosed) => {
      if (CLOSE_REFUSAL[outcome]) {
        refuse(CLOSE_REFUSAL[outcome]);
        return;
      }

      leave();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be closed: ${error.message}`),
  }));

  const archive = useMutation(() => ({
    mutationFn: (id: number) => archiveConversation(id),
    onSuccess: (outcome: ConversationArchived) => {
      if (ARCHIVE_REFUSAL[outcome]) {
        refuse(ARCHIVE_REFUSAL[outcome]);
        return;
      }

      leave();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be archived: ${error.message}`),
  }));

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
                  pressing="Closing…"
                  says="Permanently end the conversation, delete the worktree, and take it off the conversations list. The branch stays where it is."
                  working={closeAway.isPending}
                  press={() => closeAway.mutate(id())}
                />
              }
            >
              <Action
                class={styles.archive}
                label="Archive"
                pressing="Archiving…"
                says="Take it off the conversations list. Its record stays where it is."
                working={archive.isPending}
                press={() => archive.mutate(id())}
              />
            </Show>
          )}
        </Menu>
      </PaneHead>

      {/* Outside the header, because the press that opens it shuts the menu it
          was made in: what the human is looking at from here is one card over
          the page. */}
      <Refusal said={refused()} close={() => setRefused(null)} />
    </>
  );
}
