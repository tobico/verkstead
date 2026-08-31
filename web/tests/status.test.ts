//! What the status button says about a Conversation, before anything is drawn:
//! the one word for where the work stands, and the line under it saying what is
//! running.
//!
//! Worth a test of its own because it is a fold rather than a field. Every fact
//! behind the first line is already on the Conversation and they overlap by
//! design — a run that stopped without the human is waiting *and* blocked *and*
//! resumable — so what is being asked here is which of them wins, one row of
//! the order at a time. The page's own tests say where the words land; these
//! say what the words are.

import { describe, expect, it } from "vitest";

import type { AgentOutputEvent, ConversationView } from "../src/api/types";
import { agent, status } from "../src/workbench/StatusButton";

import building from "./fixtures/conversation-building.json" with {
  type: "json",
};

/// A Conversation in the middle of the ladder, with nothing about it waiting,
/// stopped, running or driven: the state every case below is one override away
/// from.
const QUIET: ConversationView = {
  ...(building as ConversationView),
  state: "Implementing",
  waiting: false,
  blocked_on: null,
  stopped_by_hand: false,
  ready_to_resume: false,
  waiting_on_checks: false,
  working: false,
  driven: false,
  resets: null,
  timeline: [],
};

/// That Conversation with something else true about it.
function like(over: Partial<ConversationView>): ConversationView {
  return { ...QUIET, ...over };
}

/// A session on its record, running or finished.
function ran(over: Partial<AgentOutputEvent>): AgentOutputEvent {
  return {
    id: 1,
    at: "2026-08-30T02:00:00Z",
    lines: 12,
    turns: 3,
    latest: "Working on it",
    running: true,
    idle: false,
    profile: "Work",
    model: "claude-fable-5",
    agent_type: "Claude",
    ...over,
  };
}

describe("the status word", () => {
  /// The top of the order, and the one the accent is spent on: something is
  /// waiting on the human, and they are the only one who can move it.
  it("says a Conversation waiting on the human, in the accent", () => {
    expect(status(like({ waiting: true }))).toEqual({
      word: "Waiting on you",
      state: "Implementing",
      attention: true,
    });
  });

  /// And it says it over everything under it, which is the point of it being
  /// first: a stop that happened without the human is all four of these at
  /// once, and *Waiting on you* is the one that says what to do about it.
  it("says it over a stop, a resume and a run", () => {
    const said = status(
      like({
        waiting: true,
        blocked_on: 12,
        ready_to_resume: true,
        working: true,
        driven: true,
      }),
    );

    expect(said.word).toBe("Waiting on you");
  });

  /// The other accented one: a stop that is not the human's own on a
  /// Conversation that is somehow not waiting on them.
  it("says Blocked on a stop that was not the human's, in the accent", () => {
    expect(status(like({ blocked_on: 12 }))).toEqual({
      word: "Blocked",
      state: "Implementing",
      attention: true,
    });
  });

  /// A stop they made themselves is the quiet half of the same fact. They were
  /// there; the word is worth reading and there is nothing to shout about.
  it("says Stopped on a stop the human pressed, and quietly", () => {
    expect(status(like({ blocked_on: 12, stopped_by_hand: true }))).toEqual({
      word: "Stopped",
      state: "Implementing",
      attention: false,
    });
  });

  /// And on a Conversation something ought to be driving with no stop on the
  /// record at all: a run that was never started, or a server that came back up
  /// without it.
  it("says Stopped where there is a resume to make and no stop recorded", () => {
    expect(status(like({ ready_to_resume: true })).word).toBe("Stopped");
  });

  /// A wrap-up down to its checks is neither stopped nor running: it waits on
  /// GitHub, which is nobody here.
  it("says a wrap-up is waiting on its checks", () => {
    expect(
      status(like({ state: "Wrapping", waiting_on_checks: true })),
    ).toEqual({
      word: "Waiting on checks",
      state: "Wrapping",
      attention: false,
    });
  });

  /// Under the stop, though — a wrap-up with a resume to make has stopped, and
  /// the checks are not what is holding it.
  it("says Stopped over the checks where there is a resume to make", () => {
    expect(
      status(
        like({
          state: "Wrapping",
          waiting_on_checks: true,
          ready_to_resume: true,
        }),
      ).word,
    ).toBe("Stopped");
  });

  it("says Running while a session is in the worktree", () => {
    expect(status(like({ working: true }))).toEqual({
      word: "Running",
      state: "Implementing",
      attention: false,
    });
  });

  /// A session that has gone quiet says nothing extra. It is still what the run
  /// is doing, and a second word for it would be the button reporting on the
  /// agent's typing.
  it("says Running whether or not the session is talking", () => {
    const quiet = like({
      working: true,
      timeline: [{ AgentOutput: ran({ idle: true }) }],
    });

    expect(status(quiet).word).toBe("Running");
  });

  /// And the moment between one step of a backlog and the next: nothing is
  /// running, and something of Verkstead's own is still holding it.
  it("says Driven where nothing runs and something still drives", () => {
    expect(status(like({ driven: true }))).toEqual({
      word: "Driven",
      state: "Implementing",
      attention: false,
    });
  });

  it("says Running over Driven, a driven run being the ordinary one", () => {
    expect(status(like({ working: true, driven: true })).word).toBe("Running");
  });

  /// The three states no status applies to: nothing is supposed to be driving
  /// one, and for two of them the word for where it got to *is* the state.
  it("says the bare state on a Draft, a Done and a Closed conversation", () => {
    for (const state of ["Draft", "Done", "Closed"] as const) {
      expect(
        status(like({ state, waiting: true, blocked_on: 12, working: true })),
      ).toEqual({ word: null, state, attention: false });
    }
  });

  /// And the same where a Conversation on the ladder is doing none of the six
  /// things above: a state with nothing to add to it is a state on its own
  /// rather than a word invented to sit beside it.
  it("says the bare state where nothing else holds", () => {
    expect(status(QUIET)).toEqual({
      word: null,
      state: "Implementing",
      attention: false,
    });
  });
});

describe("what the second line says", () => {
  /// The Profile and the model as the human would say them, with nothing
  /// between: one thing being named rather than two facts joined.
  it("names the running session's profile and model", () => {
    expect(
      agent(like({ working: true, timeline: [{ AgentOutput: ran({}) }] })),
    ).toBe("Work Fable 5");
  });

  /// The prettifying is the viewer's, so an id this build has never heard of
  /// still reaches the human — as itself.
  it("says a model it does not know as the id it travelled as", () => {
    expect(
      agent(
        like({
          working: true,
          timeline: [{ AgentOutput: ran({ model: "claude-opus-7" }) }],
        }),
      ),
    ).toBe("Work claude-opus-7");
  });

  /// The last running session, a Conversation running one at a time — a record
  /// is a column of finished ones with at most one live at the end of it.
  it("names the session that is running rather than one that has finished", () => {
    expect(
      agent(
        like({
          working: true,
          timeline: [
            { AgentOutput: ran({ id: 1, running: false, profile: "Old" }) },
            { AgentOutput: ran({ id: 2, profile: "Now" }) },
          ],
        }),
      ),
    ).toBe("Now Fable 5");
  });

  /// The one stop that says something a resume cannot: every other one waits
  /// for the same press, and this one waits for the same press and an account.
  it("says when the account comes back on a stop a window made", () => {
    expect(agent(like({ ready_to_resume: true, resets: "3pm" }))).toBe(
      "Out of window until 3pm",
    );
  });

  /// Every other moment with nothing registered, which includes the quiet
  /// between two steps of a backlog and every Draft, Done and Closed
  /// conversation there is.
  it("says nothing is running in every other quiet moment", () => {
    expect(agent(QUIET)).toBe("No agent running");
    expect(agent(like({ driven: true }))).toBe("No agent running");
    expect(agent(like({ state: "Done" }))).toBe("No agent running");

    // A session that has ended is not one that is running: the record keeps it
    // and the button has moved on.
    expect(
      agent(like({ timeline: [{ AgentOutput: ran({ running: false }) }] })),
    ).toBe("No agent running");
  });

  /// A session from before Verkstead wrote either half down. There is one
  /// running and nothing true to say about it, which is what this says.
  it("says a session is running where the record kept neither half", () => {
    expect(
      agent(
        like({
          working: true,
          timeline: [{ AgentOutput: ran({ profile: null, model: null }) }],
        }),
      ),
    ).toBe("Agent running");
  });

  it("says the half it has where the record kept one of them", () => {
    expect(
      agent(
        like({ working: true, timeline: [{ AgentOutput: ran({ model: null }) }] }),
      ),
    ).toBe("Work");
  });
});
