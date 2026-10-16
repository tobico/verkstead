//! A Memo, opened: the whole of it under its title, and under that who left it
//! and when, with the one act on it beside.
//!
//! The handoff's pane — see `Document.tsx` — with a foot the handoff has no use
//! for. The card a memo is opened from is its kind and its title and nothing
//! else, so what the record used to stamp on the card is said here instead: the
//! session that left it, the day it did, and, where it has been taken back, by
//! whom and why. The number it is retired by is drawn nowhere — it is the
//! session's handle, and the prompt is what names it to the session.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, type JSX } from "solid-js";

import { restoreMemo, retireMemo } from "../api/client";
import type { ConversationView, MemoEvent, MemoPressed } from "../api/types";
import { ErrorLine } from "../notices";
import { QuietButton } from "../QuietButton";
import { keyOf, useDevice } from "../reaching";
import { plainDate, utcStamp } from "../set/when";
import { Document } from "./Document";
import styles from "./Memo.module.css";
import { pathTo } from "./openings";

/// What a memo is called on its card and over its pane: the title the session
/// gave it, or — on one left before there were titles — the first line of what
/// it wrote, the card cutting either to one line.
export function memoTitle(memo: MemoEvent): string {
  if (memo.title !== null) return memo.title;

  const rendered = document.createElement("template");
  rendered.innerHTML = memo.html;

  return (
    (rendered.content.textContent ?? "")
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line !== "") ?? ""
  );
}

/// The details pane of a Memo.
///
/// Retire on one that stands and Restore on one that does not, at the foot
/// beside the line saying who left it. One press either way, asking nothing
/// first because the other undoes it, and the pane redraws from what the server
/// answered rather than from what the press hoped — a Retire on one a session
/// got to first comes back with that session's name on it.
export function MemoPane(props: {
  conversation: number;
  memo: MemoEvent;
  /// Open another Event's pane the way pressing its card does — the session
  /// that left this one, here.
  open: (event: number) => void;
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const device = useDevice();

  const press = useMutation(() => ({
    mutationFn: (retiring: boolean) =>
      (retiring ? retireMemo : restoreMemo)(
        device(),
        props.conversation,
        props.memo.id,
      ),
    onSuccess: (pressed: MemoPressed) => {
      if (pressed === "NoSuchMemo") return;

      // The memo it answered with, put straight into the Conversation the
      // panes are drawn from — under the key they read it by — so the card and
      // this pane turn over without waiting on the Nudge's re-read.
      queries.setQueryData<ConversationView>(
        keyOf(device(), "conversation", String(props.conversation)),
        (view) =>
          view && {
            ...view,
            timeline: view.timeline.map((event) =>
              "Memo" in event && event.Memo.id === pressed.Memo.id
                ? { Memo: pressed.Memo }
                : event,
            ),
          },
      );
    },
  }));

  return (
    <Document
      heading={memoTitle(props.memo)}
      html={props.memo.html}
      empty="The session wrote nothing down."
      back={props.back}
    >
      <footer class={styles.foot}>
        <div class={styles.lines}>
          <p>
            Left by{" "}
            {/* A link, so it reads as somewhere to go and can be opened
                elsewhere; pressed here, it is the card press. */}
            <a
              href={pathTo(props.conversation, props.memo.left_in, device())}
              onClick={(event) => {
                if (event.metaKey || event.ctrlKey || event.shiftKey) return;
                event.preventDefault();
                props.open(props.memo.left_in);
              }}
            >
              {props.memo.left_by}
            </a>{" "}
            on{" "}
            <time datetime={props.memo.at} title={utcStamp(props.memo.at)}>
              {plainDate(props.memo.at)}
            </time>
          </p>
          <Show when={props.memo.retired}>
            {(retired) => (
              <p class={styles.retiredBy}>
                Retired by {retired().by}
                <Show when={retired().reason}>
                  {(reason) => <>: {reason()}</>}
                </Show>
              </p>
            )}
          </Show>
        </div>
        <QuietButton
          onClick={() => {
            if (!press.isPending) press.mutate(props.memo.retired === null);
          }}
        >
          {props.memo.retired ? "Restore" : "Retire"}
        </QuietButton>
      </footer>
      <Show when={press.isError}>
        <ErrorLine>
          The memo could not be {props.memo.retired ? "restored" : "retired"}:{" "}
          {press.error?.message}
        </ErrorLine>
      </Show>
    </Document>
  );
}
