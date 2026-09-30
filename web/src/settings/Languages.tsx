//! Which languages a session gets build support for, on the settings page: one
//! box per **descriptor** the server loaded, and — on the one whose store an
//! sccache bounds — how big that store may grow.
//!
//! **The page is drawn from the descriptors rather than from a list it holds.**
//! A language is data: the server loads the descriptors embedded in its binary,
//! merges what `config.yaml` says over them, and hands the lot over with a
//! label each. So a language an installer wrote a descriptor for has a box here
//! without a line of this file knowing its name, and there is nothing here that
//! would have to be edited to add one.
//!
//! **The file only.** There is no descriptor editor: the two keys of an entry
//! this page writes are the switch and the size, and an installer who wants a
//! variable, a manifest or a label edits `config.yaml`.
//!
//! Which is why a language whose entry in that file would not load is drawn
//! with its controls off and a sentence saying why: the two keys they write go
//! into that entry, and the server writes it back exactly as it was typed. What
//! the sentence says is the reason — the variable, where a variable is what was
//! refused — and whether the language is running on the descriptor Verkstead
//! ships or off altogether. A language is never dropped from the list over it:
//! it is still there, and which of the two happened is the thing to know.
//!
//! One of two things on this page that are about a **Sandbox** rather than about
//! who Verkstead is — the Sandbox binds section is the other — and the only one
//! of the two that is on with nothing configured. Every path that section adds
//! is a hole somebody typed on purpose; what a language opens is Verkstead's own
//! directory, holding nothing but build output, so it can be opened for a human
//! who never asked and the only control over it here is the one that *closes*
//! it.
//!
//! It is on with nothing configured, which is the whole shape of the feature: a
//! human should not have a slower machine for never having found this section.
//! So a checkbox says where its language stands rather than whether anybody has
//! touched it, and the size field shows the default as a placeholder rather than
//! as a value somebody chose.
//!
//! Two halves in two panes, which is what the settings page is: a card in the
//! middle pane naming the languages that have build support on, and the controls
//! that change them in the details pane it opens, at `/settings/languages`. What
//! the human cannot fix from the browser — no sccache where the server can see
//! one — is on the card as well as in the pane, the way the credentials'
//! warnings are: whoever needs to read it is precisely whoever is not editing.
//!
//! Both halves read the one query, which is the one the credentials above them
//! read: one payload holds both files, and a read apiece would be two opinions
//! about what is saved.
//!
//! Two ways to save, because there are two kinds of control. A checkbox saves
//! itself the moment it is ticked — a box that needed a second press to mean
//! anything is not a box. A size is typed, so it saves on a press of its own;
//! nothing is committed while somebody is still halfway through writing `30`.
//!
//! Which is why **a tick sends the sizes the server last gave it** rather than
//! what the fields hold. One request writes the whole file, so a tick has to say
//! something about every size, and saying what is in a box would commit a number
//! nobody pressed Save on — the `5` of a `50` somebody was halfway through and
//! then thought better of the whole thing and unticked. What was typed stays
//! typed: the tick did not save it, so the field goes on holding it and its own
//! Save is still what commits it. The Cleanup pane's two durations are the same
//! rule for the same reason.
//!
//! A size hangs off the checkbox it belongs to, which is the page's one pattern
//! for configuration that only means something while something else is on:
//! indented under that box, and disabled while it is off — see [`Nested`]. It is
//! disabled for a second reason as well, and the reason is the warning above it:
//! the size is sccache's own, so a server with no sccache has nothing to read it.
//!
//! **And a language with no field for its size still sends one.** `size` is a
//! key of every entry whoever wrote it, and only the language whose store an
//! sccache bounds has anything here reading it — so a save built out of the
//! fields drawn would write `config.yaml` with every other language's size gone,
//! a key nobody was ever shown being emptied by a press about something else.
//! What every save sends is what the server last gave it, per language, for both
//! keys — see [`heldLanguages`].
//!
//! Everything goes through the one settings endpoint, which writes both files:
//! the author rides along as it stands and the token is left alone, so saving a
//! store size cannot lose either.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch as Choose,
  createSignal,
  type JSX,
} from "solid-js";

import { CardButton } from "../CardButton";
import { Check, Nested } from "../Check";
import { PaneSticky } from "../Panes";
import { loadSettings, saveSettings } from "../api/client";
import type {
  LanguageEdit,
  LanguageView,
  SettingsSaved,
  SettingsView,
  UnreadEntry,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import {
  heldAtOnce,
  heldCleanup,
  heldInstructions,
  heldLanguages,
  heldPaths,
} from "./held";
import styles from "./Languages.module.css";

/// What the section is called, wherever it names itself: the card's heading and
/// the pane's head.
const TITLE = "Language support";

/// The settings as they stand, read once for the two panes that draw them.
///
/// The same read the credentials are drawn from, by the same key: one payload
/// holds both files, and two queries over it would be two opinions about what is
/// saved.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// What is said where a session's compiling is not cached, which the server has
/// already decided.
///
/// Drawn on the card and in the pane alike, because it is the same sentence in
/// both: said where somebody would otherwise wonder why nothing got faster. Not
/// an error — the downloads are still shared, and the builds still work.
///
/// **And it is something to go and do**, which it was not always. `cached`
/// carried a third answer for a while: a Windows server was a machine where no
/// session could reach a compile server at all, because the AppContainer a
/// session ran in was refused the loopback the client talks to its server over,
/// and telling somebody to install sccache there would have changed nothing. A
/// session on that platform runs as a local account of Verkstead's own now and
/// reaches the loopback like anything else, so the answer went with the reason
/// for it and what is left everywhere is an instruction.
function uncompiled(): JSX.Element {
  return (
    <p class={styles.warning}>
      No sccache is installed where the server can see it, so dependency{" "}
      <em>compiles</em> are not cached — only the downloads. Install sccache on
      the server to cache the compiling too.
    </p>
  );
}

/// Whether the warning above is worth drawing at all.
///
/// Only for a language that is switched on, because the rest of it is only true
/// then: switched off, nothing of that language is cached at all, and a line
/// saying the downloads still are would be wrong exactly where somebody has just
/// turned it off.
///
/// The setup card's own warning is gated on more than this: it is drawn only
/// for a Repo that builds Rust, because that is a note above a press rather
/// than a page about the machine — see `ConversationView::compiles_uncached`,
/// which the server works out with the switch already in hand.
function warned(told: SettingsView | undefined): boolean {
  return (told?.languages ?? []).some(
    (language) =>
      language.enabled &&
      language.compiling !== null &&
      language.compiling !== "Cached",
  );
}

/// Whether the size hanging off a checkbox means anything.
///
/// The checkbox being on, an sccache being there to read it, and the entry this
/// would be written into being one the server could read: the size is sccache's
/// own word, so a server without one has nowhere to put it, and an entry that
/// would not load is one nothing can be written into. All three are the group's
/// *off*, and the group is drawn greyed rather than taken away — a field that
/// vanished would say the setting had, and it has not.
function sizeable(language: LanguageView): boolean {
  return (
    language.enabled &&
    language.compiling === "Cached" &&
    language.unread === null
  );
}

/// What is said under a language whose entry in `config.yaml` was not used.
///
/// The reason first, because it is what there is to fix, and it is the server's
/// own sentence: what goes wrong in a file somebody hand-wrote is open-ended,
/// and a viewer holding the vocabulary would be a release that could not add a
/// reason. Then what the language is running on meanwhile, which is the other
/// thing to know and the one this page can word for itself — the difference
/// between a cache working exactly as it did before that entry was written and
/// a language that is off until somebody goes and looks.
///
/// Drawn where `uncompiled` is and greying the controls beside it, for the
/// reason that one does not: the two keys those controls write go into this
/// entry, and a box that sprang back the moment it was ticked would be a worse
/// answer than a box that says why it cannot be.
function unreadable(entry: UnreadEntry): JSX.Element {
  return (
    <p class={styles.warning}>
      Its entry in <code>config.yaml</code> {entry.why}.{" "}
      <Choose>
        <Match when={entry.running_on === "BuiltIn"}>
          Verkstead is using the descriptor it ships, which is what this
          language had before that entry was written.
        </Match>
        <Match when={entry.running_on === "Nothing"}>
          Verkstead ships no descriptor of that name, so this language is off
          until the entry is fixed.
        </Match>
      </Choose>
    </p>
  );
}

/// And which languages those are, in the words the card names them by.
///
/// On the card as well as in the pane, the way the sccache warning is and for
/// the same reason: whoever needs to read it is precisely whoever is not
/// editing. What it cannot say there is the reason — that is a sentence each,
/// and the card is a line.
function unread(told: SettingsView | undefined): string[] {
  return (told?.languages ?? [])
    .filter((language) => language.unread !== null)
    .map((language) => language.label);
}

/// What has build support switched on, in the words the card names them by.
///
/// The labels the server gave, which for a language an installer wrote is the
/// one in their file. A list rather than a sentence, because that is what the
/// card says: the languages a session can build, and nothing under the heading
/// where there are none.
function standing(told: SettingsView | undefined): string[] {
  return (told?.languages ?? [])
    .filter((language) => language.enabled)
    .map((language) => language.label);
}

/// What has build support, as the card that opens it.
///
/// What is on the card is what somebody scanning the page is after: which
/// languages a session builds with help, and the one thing about that which
/// wants doing somewhere else. The controls are in the pane.
export function LanguagesCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <Choose>
      <Match when={settings.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          Could not read the settings: {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data}>
        {(told) => (
          <CardButton
            as="article"
            class={styles.languagesCard}
            open={props.open}
            press={props.press}
          >
            <h2>{TITLE}</h2>

            <Show when={warned(told())}>{uncompiled()}</Show>

            <Show when={unread(told()).length > 0}>
              <p class={styles.warning}>
                Verkstead could not read what <code>config.yaml</code> says
                about {unread(told()).join(", ")}.
              </p>
            </Show>

            <Show when={standing(told()).length > 0}>
              <p class={styles.standing}>{standing(told()).join(", ")}</p>
            </Show>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the controls that change them, which is the details pane the card opens.
///
/// There is no Save over the whole of it and no Cancel: a checkbox is its own
/// press and a size has one of its own, and a details pane is left by opening
/// something else or by the way back a narrow window draws.
export function LanguagesPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed in a size field, by the language it belongs to — a
  // language with nothing typed is one whose field follows the server, as the
  // author fields do.
  const [typed, setTyped] = createSignal<Record<string, string>>({});

  const told = (): SettingsView | undefined => settings.data;

  /// What a field holds: what was typed, else the size somebody configured,
  /// else nothing at all — because an unconfigured size is drawn as the
  /// placeholder underneath rather than as text in the box.
  const size = (language: LanguageView) =>
    typed()[language.name] ?? (language.size_configured ? language.size : "");

  /// What a save is asked to do: every language as it is to stand, and the one
  /// whose size was just pressed Save on, where one was.
  ///
  /// The second half is what says whether that field should let go of what was
  /// typed and follow the server again. A tick's save carries the sizes the
  /// server holds, so it commits nothing anybody typed and leaves the boxes
  /// alone — the same shape, and the same reason, as the Cleanup pane's.
  type Asked = { edit: LanguageEdit[]; committed: string | null };

  const save = useMutation(() => ({
    mutationFn: ({ edit }: Asked) => {
      const author = told()?.git_author ?? { name: "", email: "" };

      return saveSettings({
        git_author: author,
        // Untouched. This form has no business with the credentials, and a
        // blank token field read as *clear this* is exactly what `Keep` is
        // here to stop.
        github_token: "Keep",
        languages: edit,
        // And what becomes of an archived Conversation, likewise — see
        // [`heldCleanup`].
        cleanup: heldCleanup(told()),
        // And how much Verkstead runs at once, likewise — see [`heldAtOnce`].
        at_once: heldAtOnce(told()),
        // And so is how a conflict is resolved, which is the section under it
        // on the page.
        conflict_resolution: told()?.conflict_resolution ?? "Merge",
        // And the switch on the GitHub section, likewise.
        share_on_done: told()?.share_on_done ?? false,
        // And the paths as they stand, which is the same reason again: a save
        // says what the file holds afterwards, so a list left out is a list
        // emptied — see [`heldPaths`].
        ...heldPaths(told()),
        // And the text every session is given, likewise — see
        // [`heldInstructions`].
        ...heldInstructions(told()),
        // And the ignore rules left exactly where they are. Alone among the
        // settings they travel as an action rather than a value: this form has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody — see
        // [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
        // And the declared MCP servers left where they are, for the reason
        // beside it: they are the other setting that travels as an action, and a
        // section that spoke for them could have its own save refused over a
        // name somebody hand-edited into the file weeks ago — see
        // [`McpServersEdit`].
        mcp_servers: "Keep",
      });
    },
    onSuccess: (saved: SettingsSaved, asked: Asked) => {
      // What was typed goes, because the answer is now what the field follows —
      // and only for the language this save committed. A tick commits none, so
      // what somebody is halfway through writing is still theirs.
      if (asked.committed !== null) {
        const language = asked.committed;
        setTyped((held) => {
          const { [language]: _committed, ...rest } = held;
          return rest;
        });
      }

      // The save's answer *is* a fresh read of both files, so a second read
      // would learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// Every language as a save puts it back, with one entry's key replaced —
  /// which is what either press sends. The sizes are the *server's*, so a press
  /// that is not about a size commits none of them: see [`heldLanguages`], and
  /// this module's header for why a tick does not send what the field holds.
  const writing = (name: string, key: Partial<LanguageEdit>): LanguageEdit[] =>
    heldLanguages(told()).languages.map((language) =>
      language.name === name ? { ...language, ...key } : language,
    );

  /// A box ticked, which saves itself.
  const flip = (language: LanguageView, enabled: boolean) =>
    save.mutate({
      edit: writing(language.name, { enabled }),
      committed: null,
    });

  /// And a size pressed, which sends that language's field.
  const commit = (ev: SubmitEvent, language: LanguageView) => {
    ev.preventDefault();
    save.mutate({
      edit: writing(language.name, { size: size(language) }),
      committed: language.name,
    });
  };

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title={TITLE} />
      </PaneSticky>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Could not read the settings: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={told()}>
          {(set) => (
            <div class={styles.languages}>
              <For each={set().languages}>
                {(language) => (
                  <>
                    <Check
                      label={language.label}
                      on={language.enabled}
                      // And off for a language whose entry would not load: the
                      // switch is a key of that entry, and there is nothing
                      // readable there to write it into — see [`unreadable`],
                      // which is the sentence underneath saying so.
                      disabled={save.isPending || language.unread !== null}
                      flip={(enabled) => flip(language, enabled)}
                    />

                    {/* The size is sccache's, so the group is off where there
                        is no sccache to read it as well as where the box is
                        unticked. A language whose store nothing bounds has no
                        group at all — there is no size to draw. */}
                    <Show when={language.compiling}>
                      <Nested on={sizeable(language)}>
                        <form
                          class={styles.sizing}
                          onSubmit={(ev) => commit(ev, language)}
                        >
                          <label for={`language-size-${language.name}`}>
                            How large the compiled half may grow
                          </label>
                          <div class={styles.field}>
                            <input
                              id={`language-size-${language.name}`}
                              type="text"
                              autocapitalize="off"
                              autocorrect="off"
                              spellcheck={false}
                              // The default, so an empty box reads as the size
                              // nobody has chosen rather than as no size at all.
                              placeholder={language.size}
                              value={size(language)}
                              onInput={(ev) =>
                                setTyped((held) => ({
                                  ...held,
                                  [language.name]: ev.currentTarget.value,
                                }))
                              }
                            />
                            <button type="submit" disabled={save.isPending}>
                              Save
                            </button>
                          </div>
                        </form>
                      </Nested>
                    </Show>

                    {/* And why nothing above it can be changed, where the
                        entry those two keys go into would not load. */}
                    <Show when={language.unread}>
                      {(entry) => unreadable(entry())}
                    </Show>
                  </>
                )}
              </For>

              <Show when={warned(set())}>{uncompiled()}</Show>

              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </>
  );
}
