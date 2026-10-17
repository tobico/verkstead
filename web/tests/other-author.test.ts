//! What a press says over a companion under another author than the repo the
//! conversation is on (ADR-0025): the fact, and where the companion is on the
//! conversation already, taking it off as the way on.

import { describe, expect, it } from "vitest";

import type { OtherAuthor } from "../src/api/types";
import { adoptRefusal } from "../src/workbench/Adoption";
import { companionAddRefusal, repoSwitchRefusal } from "../src/workbench/Setup";
import { takeUpRefusal } from "../src/workbench/TakeUp";
import { grillRefusal } from "../src/workbench/Timeline";

const OTHER: OtherAuthor = {
  companion: "askance",
  companion_author: "Work",
  author: "Home",
};

describe("a companion of another author", () => {
  it("is taken off first where it is on the conversation already", () => {
    const said =
      "The askance repo is Work's and this conversation's repo is Home's. A conversation works as one author. Take the askance repo off first.";

    expect(grillRefusal({ OtherAuthor: OTHER })).toBe(said);
    expect(adoptRefusal({ OtherAuthor: OTHER })).toBe(said);
    expect(takeUpRefusal({ OtherAuthor: OTHER })).toBe(said);
    expect(repoSwitchRefusal({ OtherAuthor: OTHER })).toBe(
      "The askance repo is Work's and that repo is Home's. A conversation works as one author. Take the askance repo off first.",
    );
  });

  it("is the fact alone where the add was refused, nothing having been added", () => {
    expect(companionAddRefusal({ OtherAuthor: OTHER })).toBe(
      "The askance repo is Work's and this conversation's repo is Home's. A conversation works as one author.",
    );
  });
});
