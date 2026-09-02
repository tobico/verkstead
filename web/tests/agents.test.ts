//! How one session's account, model and backend read out together — the one
//! reading every place that says who runs a session shares.
//!
//! Worth a test of its own because it is a composition rather than a field.
//! Three rules decide what a pairing reads as — the backend's name, whether the
//! model's own name has already said it, and whether the account's name is worth
//! saying at all — and each of them is a way of being quietly wrong: a backend
//! named twice, a backend dropped from a model nobody has heard of, an account's
//! name taken off a run it did not make. The pages' own tests say where the
//! words land; these say what the words are.

import { describe, expect, it } from "vitest";

import { briefly, reading } from "../src/agents";
import type { ProfileEntry } from "../src/api/types";

/// One saved profile of a backend, which is all this cares about them: a name
/// and an agent type.
const profile = (
  name: string,
  agent_type: ProfileEntry["account"]["agent_type"],
): ProfileEntry => ({
  id: 1,
  name,
  account:
    agent_type === "Claude"
      ? { agent_type, claude_dir: "/srv/dir", config_file: "/srv/file" }
      : { agent_type, home: "/srv/home" },
  models: [],
  broken: null,
});

/// A list with two Claude Code accounts on it, which is what makes the account's
/// own name worth saying.
const TWO = [profile("Work", "Claude"), profile("Home", "Claude")];

describe("the reading of who runs a session", () => {
  it("says the backend, the model and the account", () => {
    expect(
      reading({ agent: "Claude", model: "claude-fable-5", profile: "Work" }, TWO),
    ).toBe("Claude Code Fable 5 — Work");
  });

  /// The backend goes where the model's own name has already said it: "Grok 4.6"
  /// is a Grok Build model and says so, and "Grok Build Grok 4.6" would be the
  /// brand twice.
  ///
  /// Both spellings, because a model does not always lead with the brand: the
  /// whole of its name is searched, so one named after its backend at either end
  /// reads once.
  it("drops the backend from a model that has already named it", () => {
    expect(
      reading({ agent: "Grok", model: "grok-4.6", profile: "Build" }, [
        profile("Build", "Grok"),
      ]),
    ).toBe("Grok 4.6");
    expect(
      reading({ agent: "Codex", model: "gpt-5-codex", profile: "Work" }, [
        profile("Work", "Codex"),
      ]),
    ).toBe("GPT-5 Codex");
  });

  /// And stays where the model's name is nobody's brand, which is most of them.
  it("keeps the backend beside a model named after nothing", () => {
    expect(
      reading(
        { agent: "OpenCode", model: "minimax/minimax-m2.1", profile: "Work" },
        [profile("Work", "OpenCode")],
      ),
    ).toBe("OpenCode Minimax M2.1");
  });

  /// A profile picked before a model was paired beside it is half a choice: the
  /// backend and the account are the half there is, and no model is invented for
  /// it. The name is said whatever the list holds — with no model there is
  /// nothing else to tell one account from another.
  it("says the backend and the account where there is no model", () => {
    expect(reading({ agent: "Claude", model: null, profile: "Work" }, [
      profile("Work", "Claude"),
    ])).toBe("Claude Code — Work");
  });

  /// An id this build has never heard of keeps the backend beside it, rather
  /// than collapsing on a brand word that happens to be in the id: the list goes
  /// stale the week another model ships, and a reading that quietly stopped
  /// naming the backend would be that staleness reaching the page.
  it("says the backend and the raw id of a model it does not know", () => {
    expect(
      reading({ agent: "Claude", model: "claude-opus-9", profile: "Work" }, TWO),
    ).toBe("Claude Code claude-opus-9 — Work");
  });

  /// The account's name is dropped where its backend has one account saved:
  /// there is nothing for the name to tell apart.
  it("drops the account's name where its backend has one", () => {
    expect(
      reading({ agent: "Claude", model: "claude-fable-5", profile: "Work" }, [
        profile("Work", "Claude"),
      ]),
    ).toBe("Claude Code Fable 5");
  });

  /// Counted over the backend's own accounts rather than over the list: a second
  /// account on another backend tells nothing apart here.
  it("counts a backend's own accounts and no others", () => {
    expect(
      reading({ agent: "Claude", model: "claude-fable-5", profile: "Work" }, [
        profile("Work", "Claude"),
        profile("Build", "Grok"),
      ]),
    ).toBe("Claude Code Fable 5");
  });

  /// A name that no longer matches any saved account keeps its own: dropping it
  /// would read as the account that happens to be left, which is a run
  /// attributed to somebody who did not make it.
  it("keeps a name the saved accounts no longer hold", () => {
    expect(
      reading({ agent: "Claude", model: "claude-fable-5", profile: "Gone" }, [
        profile("Work", "Claude"),
      ]),
    ).toBe("Claude Code Fable 5 — Gone");
  });

  /// And so does a reading made before the list has been read at all. Saying the
  /// name is never wrong; dropping it can be.
  it("says the name while the saved accounts are still being read", () => {
    expect(
      reading(
        { agent: "Claude", model: "claude-fable-5", profile: "Work" },
        undefined,
      ),
    ).toBe("Claude Code Fable 5 — Work");
  });

  /// A record from before the backend was written down reads as the model and
  /// the account, with no backend guessed for it — and one with nothing but a
  /// name reads as the name rather than as an em dash with a name after it.
  it("leaves out a backend that was never recorded", () => {
    expect(
      reading({ agent: null, model: "claude-fable-5", profile: "Work" }, TWO),
    ).toBe("Fable 5 — Work");
    expect(reading({ agent: null, model: null, profile: "Work" }, TWO)).toBe(
      "Work",
    );
  });
});

/// The shorter reading the setup row's closed trigger draws, where the harness's
/// mark beside the words is what still says Claude from Codex.
describe("the reading with the backend left off", () => {
  it("says the model and the account", () => {
    expect(
      briefly({ agent: "Claude", model: "claude-fable-5", profile: "Work" }, TWO),
    ).toBe("Fable 5 — Work");
  });

  /// The same question about the account's name, asked the same way: a backend
  /// with one saved account needs no name after its model.
  it("drops the account's name where its backend has one", () => {
    expect(
      briefly({ agent: "Claude", model: "claude-fable-5", profile: "Work" }, [
        profile("Work", "Claude"),
      ]),
    ).toBe("Fable 5");
  });

  /// And the backend is said after all where there is no model to say instead:
  /// a Profile picked before a model was paired beside it has nothing else to
  /// be read by.
  it("falls back to the backend where the pairing has no model", () => {
    expect(briefly({ agent: "Claude", model: null, profile: "Work" }, TWO)).toBe(
      "Claude Code — Work",
    );
  });

  /// An id the build cannot read is still the honest thing to show: it degrades
  /// to itself here exactly as it does in the whole reading.
  it("keeps the raw id of a model it does not know", () => {
    expect(
      briefly({ agent: "Claude", model: "claude-opus-9", profile: "Work" }, TWO),
    ).toBe("claude-opus-9 — Work");
  });
});
