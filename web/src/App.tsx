//! The application: everything under the API routes the agents use.

import { Route, Router } from "@solidjs/router";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { onCleanup, onMount, type JSX } from "solid-js";

import styles from "./App.module.css";
import { Empty } from "./notices";
import { listenForNudges } from "./nudge";
import { SetPage } from "./set/SetPage";
import { SettingsPage } from "./settings/SettingsPage";
import { Workbench } from "./workbench/Workbench";

/// One client for the whole app, made once rather than per render: it is where
/// the cache lives, and a page that rebuilt it would have no cache at all.
///
/// Coming back reads afresh whatever has gone stale, which is half of what
/// stands in for the poll that used to (ADR-0009). The other half is the
/// catch-up in `nudge.ts`, on the same moment and wider: this setting re-reads
/// what a staleTime has let go, and the catch-up re-reads everything, because a
/// page that was away cannot know what it missed.
///
/// Said out loud though it is the default, because it used to be off, on the
/// reasoning that coming back to a tab is not new information about a Set. For
/// an installed app it is precisely that: the phone was away, nothing was
/// listening while it was, and the list the human is now looking at stopped
/// being true while they were gone.
const queries = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: true } },
});

export function App(): JSX.Element {
  // Held here rather than by a page, because the whole app is what a Nudge is
  // about: the stream outlives every navigation between the lists and a Set,
  // and a page that opened its own would drop it on the way to the next.
  onMount(() => onCleanup(listenForNudges(queries)));

  return (
    <QueryClientProvider client={queries}>
      <Router root={Shell}>
        {/* The workbench has the root: it is what Verkstead is for, and what a
            device with a window opens on. The Conversation in the URL is a
            record of which one is open rather than a document of its own — the
            same page draws both. */}
        <Route path="/" component={Workbench} />
        <Route path="/conversations/:id" component={Workbench} />
        {/* Everything the human configures, on one page: the GitHub token and
            the git author Verkstead was told, the Agent Profiles a session runs
            under, and the Repos a Conversation is started against. The Repos
            and the Profiles had routes of their own until they were folded in
            here; those paths are no such page now, rather than redirects to
            this one. */}
        <Route path="/settings" component={SettingsPage} />
        {/* One Set as a page of its own, which is what a push notification
            opens: a phone woken by one is being asked about that Set and
            nothing else, so it lands on the Set rather than on the workbench
            around it. The same Set is reached in the workbench as the details
            pane of the Timeline Event it landed on. */}
        <Route path="/sets/:id" component={SetPage} />
        <Route path="*" component={NoSuchPage} />
      </Router>
    </QueryClientProvider>
  );
}

/// What every page sits in. The column the stylesheets set its width on: the
/// measure every page but one is read at is `main`'s own in `styles/base.css`,
/// and the workbench's exception to it is this shell's, in `App.module.css`.
function Shell(props: { children?: JSX.Element }): JSX.Element {
  return <main class={styles.shell}>{props.children}</main>;
}

function NoSuchPage(): JSX.Element {
  return <Empty>No such page.</Empty>;
}
