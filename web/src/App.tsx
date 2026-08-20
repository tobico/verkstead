//! The application: everything under the API routes the agents use.

import { Route, Router } from "@solidjs/router";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { onCleanup, onMount, type JSX } from "solid-js";

import { ArchiveList } from "./archive/ArchiveList";
import { listenForNudges } from "./nudge";
import { PendingList } from "./pending/PendingList";
import { ProfileList } from "./profiles/ProfileList";
import { RepoList } from "./repos/RepoList";
import { SetPage } from "./set/SetPage";
import { Workbench } from "./workbench/Workbench";

/// One client for the whole app, made once rather than per render: it is where
/// the cache lives, and a page that rebuilt it would have no cache at all.
///
/// Coming back reads everything afresh. The document becoming visible again is
/// what the setting means here — the PWA reopened, the phone unlocked, the tab
/// refocused — and it is the one signal an iOS PWA reliably fires on resume.
///
/// Said out loud though it is the default, because it used to be off, on the
/// reasoning that coming back to a tab is not new information about a Set. For
/// an installed app it is precisely that: the phone was away, the interval poll
/// was suspended with it, and the list the human is now looking at stopped
/// being true while they were gone. The extra fetch is what ADR-0005 buys with
/// it, and the poll stays underneath as the fallback.
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
        {/* The phone's answering flow, which the workbench does not touch. Both
            of these are transitional: they retire once Question Sets are reached
            through the Conversation they belong to. */}
        <Route path="/pending" component={PendingList} />
        <Route path="/archive" component={ArchiveList} />
        <Route path="/repos" component={RepoList} />
        {/* What a session runs under. Reached from the sidebar beside the
            repos, because both are things a conversation is settled against. */}
        <Route path="/profiles" component={ProfileList} />
        <Route path="/sets/:id" component={SetPage} />
        <Route path="*" component={NoSuchPage} />
      </Router>
    </QueryClientProvider>
  );
}

/// What every page sits in. The column the stylesheet sets its width on.
function Shell(props: { children?: JSX.Element }): JSX.Element {
  return <main>{props.children}</main>;
}

function NoSuchPage(): JSX.Element {
  return <p class="empty">No such page.</p>;
}
