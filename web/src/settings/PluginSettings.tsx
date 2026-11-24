//! **A Plugin's settings pane** (ADR-0030, Set 1769, Q43), opened from its row
//! on the Plugins pane: the fields its manifest declares, each drawn by kind.
//!
//! - **text** is a line, and **size** a line the server reads as a size, the
//!   way the Sandbox page's build cache size is read: `30G`.
//! - **list** is a box of lines, one value per line.
//! - **secret** is stored where the GitHub token is, and never shown again.
//!   What comes back is only whether one is set, so a set one is drawn as
//!   **Replace** and **Clear**, as the Git section draws a token.
//!
//! The values are the cluster's: a save on one device is what every other
//! one's pane reads. **Only the server decides what a value may be**, so a
//! value it refuses comes back with the reason, drawn at its field, and a
//! save refused over any field writes none of them.
//!
//! The fields are drawn whether or not the plugin loaded here, under its
//! trouble, so a setting it needs can be put right from this pane.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  type JSX,
} from "solid-js";

import { PaneSticky } from "../Panes";
import { loadPluginSettings, savePluginSettings } from "../api/client";
import type {
  PluginField,
  PluginFieldEdit,
  PluginFieldRefused,
  PluginSettingsSaved,
  PluginSettingsView,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import styles from "./PluginSettings.module.css";

/// What a field's box holds, as typed: a list's lines joined by newlines, and
/// a secret's empty until something is typed into it.
function drawn(field: PluginField): string {
  switch (field.value.kind) {
    case "Text":
    case "Size":
      return field.value.value;
    case "List":
      return field.value.values.join("\n");
    case "Secret":
      return "";
  }
}

/// What a save sends for `field` with `typed` in its box, or nothing for a
/// secret nobody typed, which is kept.
function edit(field: PluginField, typed: string): PluginFieldEdit | null {
  switch (field.value.kind) {
    case "Text":
      return { key: field.key, value: { kind: "Text", value: typed } };
    case "Size":
      return { key: field.key, value: { kind: "Size", value: typed } };
    case "List":
      return {
        key: field.key,
        value: { kind: "List", values: typed.split("\n") },
      };
    case "Secret":
      return typed.trim() === ""
        ? null
        : { key: field.key, value: { kind: "Secret", value: typed } };
  }
}

export function PluginSettingsPane(props: {
  /// The plugin's uuid.
  plugin: string;
  /// The way back, which is the Plugins pane.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const key = () => ["plugins", props.plugin, "settings"];

  const settings = useReading(() => ({
    queryKey: key(),
    queryFn: () => loadPluginSettings(props.plugin),
    freshness: { reconcile: "key" },
  }));

  // What is typed into each box, by key, set from the server's answer when it
  // first arrives and after each save.
  const [typed, setTyped] = createSignal<Record<string, string>>({});
  // The secrets being replaced, by key: a set one is drawn as its presses
  // until **Replace** opens its box.
  const [replacing, setReplacing] = createSignal<Record<string, boolean>>({});
  const [refused, setRefused] = createSignal<PluginFieldRefused[]>([]);
  const [saved, setSaved] = createSignal(false);

  const fill = (view: PluginSettingsView): void => {
    setTyped(
      Object.fromEntries(view.fields.map((field) => [field.key, drawn(field)])),
    );
    setReplacing({});
  };

  let filled = false;
  createEffect(() => {
    const view = settings.data;
    if (view !== undefined && !filled) {
      filled = true;
      fill(view);
    }
  });

  const save = useMutation(() => ({
    mutationFn: (fields: PluginFieldEdit[]) =>
      savePluginSettings(props.plugin, fields),
    onSuccess: (answer: PluginSettingsSaved) => {
      setRefused(answer.refused);
      setSaved(answer.refused.length === 0);
      if (answer.refused.length === 0) {
        queries.setQueryData(key(), answer.settings);
        fill(answer.settings);
      }
    },
    onError: () => setSaved(false),
  }));

  const commit = (view: PluginSettingsView, ev: SubmitEvent): void => {
    ev.preventDefault();
    const fields = view.fields
      .map((field) => edit(field, typed()[field.key] ?? ""))
      .filter((sent): sent is PluginFieldEdit => sent !== null);
    save.mutate(fields);
  };

  const type = (key: string, value: string): void => {
    setTyped({ ...typed(), [key]: value });
    setSaved(false);
  };

  const why = (key: string): string | null =>
    refused().find((refusal) => refusal.key === key)?.why ?? null;

  /// One field's box, by its kind.
  const box = (field: PluginField): JSX.Element => {
    const id = `plugin-setting-${field.key}`;
    const value = () => typed()[field.key] ?? "";
    const write = (ev: { currentTarget: { value: string } }) =>
      type(field.key, ev.currentTarget.value);

    switch (field.value.kind) {
      case "Text":
        return <input id={id} type="text" value={value()} onInput={write} />;
      case "Size":
        return (
          <input
            id={id}
            type="text"
            autocapitalize="off"
            spellcheck={false}
            placeholder="30G"
            value={value()}
            onInput={write}
          />
        );
      case "List":
        return <textarea id={id} rows={4} value={value()} onInput={write} />;
      case "Secret": {
        const set = field.value.set;
        return (
          <>
            <Note>Stored, and never shown again.</Note>
            <Show
              when={!set || replacing()[field.key]}
              fallback={
                <div class={styles.presses}>
                  <button
                    type="button"
                    class="secondary"
                    onClick={() =>
                      setReplacing({ ...replacing(), [field.key]: true })
                    }
                  >
                    Replace
                  </button>
                  <button
                    type="button"
                    class="secondary"
                    disabled={save.isPending}
                    onClick={() =>
                      save.mutate([
                        { key: field.key, value: { kind: "SecretCleared" } },
                      ])
                    }
                  >
                    Clear
                  </button>
                </div>
              }
            >
              <input
                id={id}
                type="password"
                autocapitalize="off"
                autocorrect="off"
                autocomplete="off"
                spellcheck={false}
                value={value()}
                onInput={write}
              />
            </Show>
          </>
        );
      }
    }
  };

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Plugins", go: props.back }}
          title={settings.data?.name ?? "Plugin settings"}
        />
      </PaneSticky>

      <Switch>
        <Match when={settings.isPending}>
          <Empty>Reading its settings…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Its settings couldn't be read. {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={settings.data}>
          {(view) => (
            <form class={styles.form} onSubmit={(ev) => commit(view(), ev)}>
              <Show when={view().trouble}>
                {(trouble) => <p class={styles.trouble}>{trouble()}</p>}
              </Show>

              <Show
                when={view().fields.length > 0}
                fallback={<Empty>This plugin declares no settings.</Empty>}
              >
                <For each={view().fields}>
                  {(field) => (
                    <div class={styles.field} data-setting={field.key}>
                      <label for={`plugin-setting-${field.key}`}>
                        {field.label}
                      </label>
                      <Show when={field.description}>
                        {(description) => <Note>{description()}</Note>}
                      </Show>
                      <Show when={field.value.kind === "List"}>
                        <Note>One per line.</Note>
                      </Show>
                      {box(field)}
                      <Show when={why(field.key)}>
                        {(reason) => <ErrorLine>{reason()}</ErrorLine>}
                      </Show>
                    </div>
                  )}
                </For>

                <div class={styles.presses}>
                  <button type="submit" disabled={save.isPending}>
                    Save
                  </button>
                  <Show when={saved()}>
                    <Note>Saved.</Note>
                  </Show>
                </div>

                <Show when={save.error}>
                  {(error) => <ErrorLine>{error().message}</ErrorLine>}
                </Show>
              </Show>
            </form>
          )}
        </Match>
      </Switch>
    </>
  );
}
