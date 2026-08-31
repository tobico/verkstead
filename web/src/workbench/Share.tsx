//! Sharing a Conversation, in a pane of its own: where the last share went, and
//! every way there is to hand the record to somebody.
//!
//! It was four rows of the actions menu, which is a menu about the Conversation
//! as a whole — resume it, stop it, steer it, close it. Sharing is none of
//! those: it does nothing to the work, it is three presses rather than one, and
//! the fact it is mostly opened for is a link that a menu row can only say half
//! of. So it comes out of the menu, and what is left there is what changes the
//! Conversation.
//!
//! Opened by the share icon on the Timeline's header — see `Timeline.tsx` —
//! which is a details pane like every other, at a path of its own so it survives
//! a reload and can be linked to. The one details pane nothing on the record
//! opens: a share belongs to the Conversation rather than to any moment on it,
//! the way the backlog does.
//!
//! What it draws is the record's own [`ShareView`], which is the Conversation
//! this pane was handed: nothing is fetched here, and nothing is held beside the
//! read — a publish invalidates the Conversation, and where the share went is a
//! field on it.
//!
//! **The viewer's link and the gist are both drawn, because they are for two
//! different things.** The viewer's is what a reader is sent — the gist's id in
//! the hosted page's fragment, which draws the record as a conversation rather
//! than as source — and it is the one with a copy button, that being what the
//! human came here for. The gist is where the file actually is, and it is the
//! only place a share can be deleted: nothing in Verkstead deletes one, so the
//! way to GitHub is the way to that.
//!
//! And under them the three presses the menu lost, in the order they cost
//! something: the download, which costs nothing and reaches nobody; the publish,
//! which puts the file in a gist; and the share to the pull requests, which
//! publishes it and says so in front of whoever is reviewing the work. The last
//! is drawn only where the record holds a pull request, exactly as its row was.
//!
//! Both presses that reach GitHub answer in a **toast** rather than in the pane,
//! and that is unchanged from the menu: what GitHub refused is something to go
//! and do rather than a pane out of date, and a publish that worked hands back a
//! link to reach for.

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";

import { publishShare, sharePath, shareToPullRequests } from "../api/client";
import type {
  CommentedOn,
  ConversationView,
  MissedOut,
  ShareCommented,
  SharePublished,
} from "../api/types";
import { toast } from "../Toasts";
import { utcStamp } from "../set/when";
import { PaneSticky } from "../Panes";
import { Note } from "../notices";
import { PaneHead } from "./PaneHead";
import styles from "./Share.module.css";
import { onAPullRequest } from "./Steer";

/// What a publish came back with, as the toast it is said in.
///
/// The press here that reaches outside this machine: publishing writes to GitHub
/// as the token on the settings page, and two of the three ways it can be
/// refused are that token. So each is a sentence and a way to the page it is
/// fixed on, and the one that worked is a sentence and the link it just made.
///
/// **A toast rather than a line in the pane.** An outcome is a moment and this
/// pane is a drawing of the record — the link the publish just made is on it the
/// moment the re-read lands — and what a toast adds is something to reach for
/// while it is still the thing the human is thinking about. See `Toasts.tsx`.
export function published(outcome: SharePublished): JSX.Element {
  if (outcome === "NoToken") {
    return (
      <>
        Verkstead has no GitHub token to publish as.{" "}
        <A href="/settings/github">Put one in on the settings page.</A>
      </>
    );
  }

  if (outcome === "NoGistScope") {
    return (
      <>
        The saved GitHub token may not write gists.{" "}
        <A href="/settings/github">
          Re-issue it with the gist scope and save it again.
        </A>
      </>
    );
  }

  if ("Refused" in outcome) {
    return <>GitHub would not take it: {outcome.Refused.why}</>;
  }

  // Through the share viewer, which is what the server composed it as — see
  // `link` in `crates/server/src/sharing.rs`. The gist itself is what was
  // published; a link that draws it as a conversation is what is worth handing
  // to somebody, so that is what this opens and what the human copies.
  return (
    <>
      The share is published.{" "}
      <a href={outcome.Published.share.url} target="_blank" rel="noreferrer">
        Open it.
      </a>
    </>
  );
}

/// And what became of the one-click share, in a toast of its own.
///
/// Three shapes and one sentence each. A share that was never published says
/// what the publish would have said — it is the same write to GitHub under the
/// same token, and the settings page is where two of the three are fixed. A
/// conversation on no pull request is a pane drawn against one that has since
/// moved. And a share that went says where it went, naming whatever missed out
/// beside what worked: the file is up either way, and a human told which pull
/// request it never reached can paste the link there themselves.
export function commented(outcome: ShareCommented): JSX.Element {
  if (outcome === "NoPullRequest") {
    return <>This conversation is on no pull request.</>;
  }

  if ("NotPublished" in outcome) {
    // In the publish's own words rather than said again here. What it holds is
    // always a refusal — a publish that worked is the other shape of this — so
    // what comes back is the sentence and the way to fix it.
    return published(outcome.NotPublished.why);
  }

  const { on, missed } = outcome.Commented;
  const said: string[] = [];

  if (on.length > 0) {
    said.push(`Commented on ${on.map(named).join(", ")}.`);
  }

  for (const miss of missed) {
    said.push(`Nothing could be said on ${named(miss)}: ${miss.why}`);
  }

  return <>{said.join(" ")}</>;
}

/// What one pull request is called in that sentence: its number, and the
/// repository it is in where that is not the conversation's own.
///
/// The same rule its card draws by — an unlabeled number means the repo the
/// work is in — because a conversation ends on one pull request per repository
/// it was worked in, and `#7` means something else in each of them.
function named(pull: CommentedOn | MissedOut): string {
  return pull.repo ? `#${pull.number} in ${pull.repo}` : `#${pull.number}`;
}

export function Share(props: {
  conversation: ConversationView;
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  /// What every press here leaves behind: a pane drawn against a conversation
  /// that has moved. A publish replaces where the share went, which is the half
  /// of this pane that is a drawing of the record.
  const reread = () => {
    void queries.invalidateQueries({ queryKey: ["conversation"] });
    void queries.invalidateQueries({ queryKey: ["conversations"] });
  };

  const publish = useMutation(() => ({
    mutationFn: (id: number) => publishShare(id),
    onSuccess: (outcome: SharePublished) => {
      toast(() => published(outcome));
      reread();
    },
    onError: (error: Error) => {
      // The transport rather than the answer: a request that never landed has
      // no named outcome, so it is said in GitHub's place.
      toast(() => published({ Refused: { why: error.message } }));
    },
  }));

  const comment = useMutation(() => ({
    mutationFn: (id: number) => shareToPullRequests(id),
    onSuccess: (outcome: ShareCommented) => {
      toast(() => commented(outcome));
      reread();
    },
    onError: (error: Error) => {
      // The transport rather than the answer, exactly as the publish's is: a
      // request that never landed has no named outcome, and the publish is what
      // it would have failed at.
      toast(() =>
        commented({ NotPublished: { why: { Refused: { why: error.message } } } }),
      );
    },
  }));

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Share" />
      </PaneSticky>

      {/* Where the last share went, on a Conversation somebody has published
          one of — and a plain sentence where nobody has. The link is the share
          viewer's, composed by the server off the gist it recorded, so a share
          taken before there was a viewer still opens as a conversation. See
          `link` in `crates/server/src/sharing.rs`. */}
      <Show
        when={props.conversation.shared}
        fallback={
          <Note>
            This conversation has not been published. Publishing puts a copy of
            the record where a link reaches it.
          </Note>
        }
      >
        {(shared) => (
          <div class={styles.published}>
            <p class={styles.heading}>Published share</p>

            <p class={styles.link}>
              <a href={shared().url} target="_blank" rel="noreferrer">
                {shared().url}
              </a>
              <Copy of={shared().url} />
            </p>

            <p class={styles.when}>Taken {utcStamp(shared().at)}.</p>

            {/* And the file itself, which is the other half of what a published
                share is. Verkstead deletes no gist and offers no way to — so
                the human who wants this share gone is sent to the one place it
                can be done. */}
            <p class={styles.gist}>
              <a href={shared().gist} target="_blank" rel="noreferrer">
                {shared().gist}
              </a>
            </p>
            <p class={styles.deleting}>
              The gist is where the file is, and the only place a share can be
              deleted.
            </p>
          </div>
        )}
      </Show>

      {/* And every way there is to hand the record over, in the order they
          cost something. */}
      <div class={styles.doing}>
        {/* A file to take away, which is the one of the three that reaches
            nobody and costs nothing. A link rather than a button that fetches,
            because that is what a browser is for: the server answers as an
            attachment and names the file, so the whole of this is where it
            points — and a right-click offers *Save link as* like every other
            download the human has ever made. */}
        <a
          class={styles.download}
          href={sharePath(props.conversation.id)}
          download=""
        >
          Download
        </a>

        {/* And the same file put where a link reaches it: a secret gist,
            published as the token on the settings page. Beside the download
            rather than instead of it — one is a file to attach and the other is
            a link to paste, and which of the two a colleague wants is not this
            pane's to decide. */}
        <button
          type="button"
          class={styles.publish}
          disabled={publish.isPending}
          onClick={() => publish.mutate(props.conversation.id)}
        >
          {publish.isPending
            ? "Publishing…"
            : props.conversation.shared
              ? "Publish again"
              : "Publish"}
        </button>

        {/* And the whole of it in one press, on a conversation whose work is on
            a pull request: the same publish, and a comment carrying the link
            and what is in the file on every pull request it holds.

            Offered only where there is a pull request to say it on, which is
            what the pinned cards already say: a conversation with none has
            nowhere for this press to go. */}
        <Show when={onAPullRequest(props.conversation)}>
          <button
            type="button"
            class={styles.comment}
            disabled={comment.isPending}
            onClick={() => comment.mutate(props.conversation.id)}
          >
            {comment.isPending ? "Sharing…" : "Share to pull request"}
          </button>
        </Show>
      </div>

      <Note>
        Publishing takes a fresh snapshot every time. What was already sent goes
        on standing where it was.
      </Note>
    </>
  );
}

/// The copy button beside the viewer's link: the whole of what most visits to
/// this pane are for.
///
/// It says it copied and then stops saying it, because a clipboard write is
/// silent — nothing on the screen changes, and a press that looks like it did
/// nothing gets pressed again. A word for two seconds is the whole of the
/// feedback; there is nothing to undo and nothing to report.
///
/// A clipboard the browser refuses — an insecure origin, a permission denied —
/// leaves the link itself, which is selectable text beside the button. So a
/// failure says nothing rather than opening a notice about a convenience.
function Copy(props: { of: string }): JSX.Element {
  const [copied, setCopied] = createSignal(false);

  const copy = () => {
    void navigator.clipboard
      ?.writeText(props.of)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      })
      .catch(() => {});
  };

  return (
    <button type="button" class={styles.copy} onClick={copy}>
      {copied() ? "Copied" : "Copy"}
    </button>
  );
}
