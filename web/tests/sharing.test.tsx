//! A shared conversation, drawn from the file it travels in.
//!
//! `tests/fixtures/share.json` is a golden fixture like the workbench's: `cargo
//! test` asks the real endpoint and writes the file, so what these assertions
//! read is the record the server actually composes. Which is also what makes
//! this worth asserting on this side — the curation is the server's and is
//! proved over there, and what is proved here is that the page drawn from it
//! reads as a record and offers nothing to do about it.
//!
//! Nothing is stubbed, and that is the subject rather than a convenience. A
//! share opens off a disk with no server anywhere: `fetch` is left exactly as
//! the environment has it, so a component that reached for the network would
//! fail here the way it would fail in the recipient's browser.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import type { SharedConversation, TimelineEvent } from "../src/api/types";
import { Share } from "../src/share/Share";
import shared from "./fixtures/share.json" with { type: "json" };

const SHARED = shared as unknown as SharedConversation;

/// The same record with its timeline replaced, for the tests about one shape of
/// it.
function holding(timeline: TimelineEvent[]): SharedConversation {
  return {
    ...SHARED,
    conversation: { ...SHARED.conversation, timeline },
  };
}

/// The record this file's fixture carries, by kind.
function kinds(shared: SharedConversation): string[] {
  return shared.conversation.timeline.map(
    (event) => Object.keys(event)[0] ?? "",
  );
}

describe("a shared conversation", () => {
  it("draws the record it was given, titled for its branch", async () => {
    render(() => <Share shared={SHARED} />);

    // The timeline pane, named as the workbench names it.
    expect(await screen.findByLabelText("Timeline")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "usage-limits" })).toBeTruthy();

    // And what is on it: the brief, and every commit the branch carried.
    expect(screen.getByText("Brief")).toBeTruthy();
    expect(
      screen.getByText("feat: read the account's own limit error"),
    ).toBeTruthy();
  });

  /// The curation is the server's — see `crates/server/tests/sharing.rs` — and
  /// this is the half of it a reader sees: what the fixture carries is what the
  /// page has to draw, and the kinds that never board leave no trace.
  it("carries the asks and the commits, and nothing of the sessions", () => {
    const held = kinds(SHARED);

    expect(held).toContain("Brief");
    expect(held).toContain("QuestionSet");
    expect(held).toContain("Commit");

    for (const left of ["AgentOutput", "Handoff", "Notice", "PullRequest"]) {
      expect(held).not.toContain(left);
    }

    render(() => <Share shared={SHARED} />);

    // Nothing marks the gap. The record between the brief and the first commit
    // held a session's output and a handoff, and the page says neither.
    expect(screen.queryByText(/handoff/i)).toBeNull();
    expect(screen.queryByText(/output/i)).toBeNull();
  });

  /// The whole of what a share is: a record to read. Every control the
  /// workbench hangs off a conversation is about doing something to it, and
  /// there is nobody on the other end of this file to do it.
  ///
  /// Asked of a record that says every one of them applies. The server clears
  /// those fields on the way out — `crates/server/tests/sharing.rs` is where
  /// that is proved — and this is the other half of the same guarantee: the
  /// page would not draw them even if a record arrived saying it could.
  it("offers nothing to press but the cards themselves", async () => {
    const { container } = render(() => (
      <Share
        shared={{
          ...SHARED,
          conversation: {
            ...SHARED.conversation,
            state: "Draft",
            ready_to_grill: true,
            ready_to_resume: true,
            ready_to_stop: true,
            working: true,
          },
        }}
      />
    ));
    await screen.findByLabelText("Timeline");

    // The ⋯ that carries stop, steer and close, and the two blocks under the
    // record that say what happens next.
    expect(
      screen.queryByRole("button", { name: "Conversation actions" }),
    ).toBeNull();
    expect(screen.queryByText("Start work")).toBeNull();
    expect(screen.queryByText("Resume")).toBeNull();

    // And nothing anywhere on the page that is a row of a menu or a field to
    // fill in: a shared brief is the document it froze as, whatever state the
    // record says the conversation is in.
    expect(container.querySelector('[role="menuitem"]')).toBeNull();
    expect(container.querySelector("textarea")).toBeNull();
    expect(container.querySelector("input")).toBeNull();
  });

  /// There is no list to pick from and no way back to one: a share is one piece
  /// of work and nothing around it.
  it("has no conversations pane and no way back to one", async () => {
    render(() => <Share shared={SHARED} />);
    await screen.findByLabelText("Timeline");

    expect(screen.queryByLabelText("Conversations")).toBeNull();
    expect(screen.queryByText("← Conversations")).toBeNull();
  });

  /// Opening a share lands on the end of the record, which is where the work
  /// got to — the same landing the workbench makes when a conversation is
  /// opened.
  it("opens on the last thing that has a pane behind it", async () => {
    render(() => <Share shared={SHARED} />);

    const details = await screen.findByLabelText("Details");
    // The fixture ends on its companion's commit, and a commit's pane is the
    // one this build's share does not carry the diff for yet.
    await waitFor(() =>
      expect(details.querySelector("h1")?.textContent).toBe("Commit"),
    );
  });

  /// A brief opens as the document it is, out of the record the file carries —
  /// no fetch, because there is nowhere to fetch from.
  it("opens the brief as the document it froze as", async () => {
    const brief = SHARED.conversation.timeline.find(
      (event): event is Extract<TimelineEvent, { Brief: unknown }> =>
        "Brief" in event,
    );
    expect(brief).toBeTruthy();

    render(() => <Share shared={holding([brief!])} />);

    const details = await screen.findByLabelText("Details");
    await waitFor(() =>
      expect(details.querySelector("h1")?.textContent).toBe("Brief"),
    );
    expect(
      details.textContent?.includes("Pausing when an account runs out of window"),
    ).toBe(true);
  });

  /// The template before anything is written into it, and any file whose slot
  /// somebody has emptied: a page that says what it is holding rather than a
  /// blank one.
  it("says so where it is carrying no conversation", () => {
    render(() => <Share shared={null} />);

    expect(screen.getByText(/not carrying a conversation/)).toBeTruthy();
  });
});
