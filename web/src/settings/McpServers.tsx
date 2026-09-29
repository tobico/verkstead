//! The MCP servers declared for this installation, on the settings page: a name
//! and a URL each, said once here and attached to a Conversation from the Attach
//! button where its Brief is written.
//!
//! A session runs with no MCP servers at all today, on purpose: a Built Root has
//! the account's own taken out of it for every harness, the way it has the
//! account's plugins and hooks. This section is the one deliberate way in — a
//! server nobody's home directory happened to hold, said in Verkstead's own
//! `config.yaml`, and given to a session only where its Conversation asked for
//! it. See ADR-0021, and **MCP server** in `CONTEXT.md`.
//!
//! **Declared once for the installation, attached per Conversation.** What is
//! here is the declaration and nothing else: a Conversation refers to one by
//! name, and nothing in this pane knows or cares which Conversations have.
//!
//! **HTTP only.** There is no command, no arguments and no transport to choose
//! on this form. A stdio server is a child process the agent starts inside its
//! own sandbox, which is a hole in the sandbox rather than a setting, and it was
//! turned down in the grilling this was settled in.
//!
//! **The name is the identity, and a server is never renamed.** It is what a
//! chip refers to and what the agent sees in front of the server's tool names,
//! so it is lowercase letters, digits and hyphens and unique among the
//! declarations. The form offers a name field on a *new* row alone: a declared
//! one draws its name as words, because renaming one would be every chip
//! pointing at it losing what it pointed at. Changing a name is removing the
//! declaration and making another. The URL is the half that is edited.
//!
//! Two halves in two panes, like every section beside it: a card in the middle
//! pane saying how many are declared, and the rows that rewrite them in the
//! details pane it opens, at `/settings/mcp-servers`. Both read the one settings
//! query the rest of the page reads, and the save goes through the one settings
//! endpoint — so the author, the token and the rest ride along as they stand.
//!
//! The declarations travel as an action rather than a value, the way the ignore
//! rules do and for the same reason: a save is *refused* over a name that is not
//! a name or one another declaration already has. So the rows are sent only once
//! somebody has touched one, and a save about an email address is one the server
//! cannot turn down over a name it never showed anybody.
//!
//! **A row nobody wrote anything in is not a declaration**, and comes off the
//! page as the save goes out — which is how one is deleted where Remove is not
//! pressed, and what keeps a row somebody added and left from turning down the
//! save it rode along with.
//!
//! A refusal is the whole request refused: nothing is written, and what comes
//! back names the row and the box it is about. So it is drawn at that row, with
//! everything the human typed left where they left it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Index, Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import type {
  McpServer,
  McpServersEdit,
  ServerField,
  ServerRefused,
  SettingsSaved,
  SettingsView,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldConfig } from "./held";
import styles from "./McpServers.module.css";

/// What the section is called, in the one place both panes read it from.
const TITLE = "MCP servers";

/// One row of the pane as it is being edited: a declaration, and whether it is
/// one the server already holds.
///
/// That second half is what decides whether the name is a field or words. It is
/// carried on the row rather than worked out by looking the name up in what was
/// read, because a row added and not yet saved has no name to look up — and a
/// name typed to match one already declared would otherwise stop being editable
/// under the hand that was typing it.
type Editing = McpServer & { fresh: boolean };

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// A count with the word it counts, so a line reads as English rather than as
/// `1 servers`.
function counted(many: number, one: string, more: string): string {
  return `${many} ${many === 1 ? one : more}`;
}

/// The declarations as they stand, as the card that opens the section.
///
/// What is on the card is what somebody scanning the page is after: how many are
/// declared, and what that comes to for a Conversation. The names are behind the
/// press — a card is read past rather than read.
export function McpServersCard(props: {
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
            class={styles.serversCard}
            open={props.open}
            press={props.press}
          >
            <h2>{TITLE}</h2>

            <p class={styles.standing}>
              <Show
                when={told().mcp_servers.length > 0}
                fallback={
                  <>
                    Nothing is declared, so a conversation has no server to
                    attach.
                  </>
                }
              >
                {counted(told().mcp_servers.length, "server", "servers")}{" "}
                declared, to attach to a conversation from its Attach button.
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the rows that rewrite them, which is the details pane the card opens.
///
/// One Save under the lot rather than a press per row: the server writes the
/// declarations as one list, so a row edited is the list sent, and a per-row
/// press would be sending the others along without saying so. There is no
/// Cancel — a details pane is left by opening something else or by the way back
/// a narrow window draws, and what was typed and never saved goes with it.
export function McpServersPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // The declarations as they are being edited, or `null` while nobody has
  // touched a row — the rows follow the server until then, the way every field
  // on this page does. `null` is also what says the save has nothing to say
  // about them, which is what keeps a save from another section one that cannot
  // be refused: see [`McpServersEdit`].
  const [rows, setRows] = createSignal<Editing[] | null>(null);

  // And what the last save was turned down over, by the row and the box it
  // named. Emptied as the next save goes out, because it describes what was
  // sent rather than what is on the page.
  const [refusals, setRefusals] = createSignal<ServerRefused[]>([]);

  const told = (): SettingsView | undefined => settings.data;

  /// Everything else in `config.yaml` this pane is not about, as it stands — see
  /// [`heldConfig`]. A save writes the whole file, so a section left out would
  /// be a section emptied.
  const held = () => heldConfig(told());

  /// The rows as they are drawn: what has been edited, or what is declared while
  /// nothing has been.
  const drawn = (): Editing[] =>
    rows() ?? (told()?.mcp_servers ?? []).map((server) => ({ ...server, fresh: false }));

  /// One box rewritten, which is what makes the next save a `Set` of the whole
  /// list — the server writes the declarations as one list, so a row edited is
  /// the list sent.
  const rewrite = (at: number, part: Partial<McpServer>) =>
    setRows(
      drawn().map((row, which) => (which === at ? { ...row, ...part } : row)),
    );

  /// A row added, empty, for the human to write a declaration into — and the one
  /// row on the page whose name is a field, this being the only moment a name is
  /// chosen.
  ///
  /// Its errors are dropped with it and with every other row's: a refusal names
  /// a row by where it stood, so a list that has gained or lost one is a list
  /// they no longer point into. Typing in a box leaves them, because the rows
  /// are still the rows the server was talking about.
  const add = () => {
    setRefusals([]);
    setRows([...drawn(), { name: "", url: "", fresh: true }]);
  };

  const remove = (at: number) => {
    setRefusals([]);
    setRows(drawn().filter((_, which) => which !== at));
  };

  /// Why the row standing at `at` was turned down, against one of its boxes —
  /// `null` where the save was not refused over that half of it.
  const trouble = (at: number, field: ServerField): string | null =>
    refusals().find((was) => was.server === at && was.field === field)?.why ??
    null;

  /// Whether a row is a declaration at all, which is whether anything was
  /// written in either box. Emptiness is decided the way the server decides it —
  /// by what is left after the spaces — so the two halves agree about which rows
  /// are declarations.
  const written = (row: Editing): boolean =>
    row.name.trim() !== "" || row.url.trim() !== "";

  /// The rows nobody wrote anything in, taken off the page as the save goes out
  /// — which is how a declaration is deleted without pressing Remove, and what
  /// stops a row somebody added and never filled in refusing the save it rode
  /// along with.
  ///
  /// Taken off the page rather than merely left out of the request, so that what
  /// is drawn and what was sent are one list: a refusal names a declaration by
  /// where it stood in what was sent, and a request that quietly left rows out
  /// would have those numbers pointing at the wrong boxes.
  const tidied = () => {
    const edited = rows();

    if (edited !== null) {
      setRows(edited.filter(written));
    }
  };

  /// What the save says about the declarations: nothing at all until somebody
  /// has touched a row, and the whole list in the order it is drawn once they
  /// have — the blank rows having been taken off it by [`tidied`] first.
  ///
  /// The row's own `fresh` flag does not travel. It is how this pane knows which
  /// name to draw as a field, and the server's business is the list as it is to
  /// stand.
  const edit = (): McpServersEdit => {
    const edited = rows();

    return edited === null
      ? "Keep"
      : { Set: { servers: edited.map(({ name, url }) => ({ name, url })) } };
  };

  const save = useMutation(() => ({
    mutationFn: () =>
      saveSettings({
        git_author: told()?.git_author ?? { name: "", email: "" },
        // Untouched. This pane has no business with the credentials, and a blank
        // token field read as *clear this* is exactly what `Keep` is here to
        // stop.
        github_token: "Keep",
        share_on_done: told()?.share_on_done ?? false,
        // And the ignore rules left exactly where they are: this pane has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody.
        ignored_comments: "Keep",
        // And the one thing this pane is about — see [`edit`].
        mcp_servers: edit(),
        ...held(),
      }),
    onSuccess: (saved: SettingsSaved) => {
      // A refusal is the whole save refused — nothing was written — so nothing
      // here is spent and nothing is read back: what the human typed stays in
      // front of them, with the errors drawn at the rows they name.
      if (saved.refused_servers.length > 0) {
        setRefusals(saved.refused_servers);
        return;
      }

      // The rows go back to following the server, which has just been told what
      // they said: what is drawn from here on is what was written down, and a
      // row that was new is a declaration now.
      setRows(null);
      setRefusals([]);

      // Taken from the answer rather than asked for again: what a save comes
      // back with *is* a fresh read of the two files, so a second read would
      // learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();

    // Whatever the last save was turned down over went with the press that is
    // being made now: the answer to this one says what is wrong with what is
    // being sent this time.
    setRefusals([]);

    // And a row nobody wrote anything in is not a declaration, so it goes before
    // the list is read off the page — see [`tidied`].
    tidied();

    save.mutate();
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
          <form class={styles.servers} onSubmit={commit}>
            {/* What the section is for, in the one line a pane on this page says
                its own in. Both halves of it: what a declaration is, and what
                has to happen before a session sees one. */}
            <Note>
              Declared here for the whole installation, over HTTP, and given to a
              session only where its conversation has attached one. A name is
              lowercase letters, digits and hyphens, and cannot be changed
              afterwards.
            </Note>

            <Show
              when={drawn().length > 0}
              fallback={<Empty>No servers are declared.</Empty>}
            >
              <ul class={styles.rows}>
                <Index each={drawn()}>
                  {(row, at) => (
                    <li class={styles.row}>
                      {/* The name is a field on a new row and words on a
                          declared one: a server is referred to by name, so
                          renaming one is deleting it and declaring another. */}
                      <Show
                        when={row().fresh}
                        fallback={
                          <p class={styles.name}>
                            <code>{row().name}</code>
                          </p>
                        }
                      >
                        <label for={`mcp-name-${at}`}>Name</label>
                        <input
                          id={`mcp-name-${at}`}
                          type="text"
                          autocapitalize="off"
                          autocorrect="off"
                          spellcheck={false}
                          placeholder="docs"
                          value={row().name}
                          onInput={(ev) =>
                            rewrite(at, { name: ev.currentTarget.value })
                          }
                        />
                      </Show>
                      <Show when={trouble(at, "Name")}>
                        {(why) => (
                          <ErrorLine class={styles.trouble}>{why()}</ErrorLine>
                        )}
                      </Show>

                      <label for={`mcp-url-${at}`}>URL</label>
                      <input
                        id={`mcp-url-${at}`}
                        type="url"
                        inputmode="url"
                        autocapitalize="off"
                        autocorrect="off"
                        spellcheck={false}
                        placeholder="https://mcp.example.com/docs"
                        value={row().url}
                        onInput={(ev) =>
                          rewrite(at, { url: ev.currentTarget.value })
                        }
                      />
                      <Show when={trouble(at, "Url")}>
                        {(why) => (
                          <ErrorLine class={styles.trouble}>{why()}</ErrorLine>
                        )}
                      </Show>

                      <QuietButton
                        class={styles.remove}
                        onClick={() => remove(at)}
                      >
                        Remove
                      </QuietButton>
                    </li>
                  )}
                </Index>
              </ul>
            </Show>

            <QuietButton class={styles.add} onClick={add}>
              Add a server
            </QuietButton>

            <div class={styles.buttons}>
              <button type="submit" disabled={save.isPending}>
                Save
              </button>
            </div>

            {/* A server that could not write the files, which is the one thing
                here that is an error rather than an answer. */}
            <Show when={save.isError}>
              <ErrorLine class={styles.failure}>
                The settings could not be saved: {save.error?.message}
              </ErrorLine>
            </Show>
          </form>
        </Match>
      </Choose>
    </>
  );
}
