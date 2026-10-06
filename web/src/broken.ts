//! Why a Profile cannot be run under as things stand, in words — and the one
//! place they are written.
//!
//! Three sites say it and none of them may say it differently: the card in the
//! Profiles section, the row in a pairing picker, and the refusal a press that
//! would have started a session comes back with. What the server sends is the
//! finding and the facts around it — see `Broken` and `ProfileTrouble` — and
//! what is here is the sentence.
//!
//! **Three of the findings are about a mirror**, which is a Profile whose
//! account is on another machine: its home not answering, its account holding
//! no login there is anything to mirror, and the harness it runs not being on
//! *this* box. Each is something to go and put right somewhere, which is why
//! every one of them names the somewhere — the machine, or this one.
//!
//! **And one is about an account at home here: it has signed out.** A press
//! found it with no login, and the card's Log in button is the fix.
//!
//! **And the harness one is the onboarding probe's own sentence**, because it
//! is the onboarding probe's own finding: the dependencies step says whether a
//! harness is on this machine, and a second vocabulary for one fact would be
//! two answers to it. See [`harnessAbsent`], which the wizard's accounts step
//! draws from as well.

import { AGENT_NAME, type AgentType } from "./agents";
import type { Broken, ProfileEntry, ProfileTrouble } from "./api/types";

/// What the onboarding probe says of a harness that is not on this machine.
///
/// The wizard's accounts step puts its own *install it in the step above* after
/// this, having a step above to point at; a profile row and a refused start say
/// it and stop. The sentence itself is one sentence either way.
export function harnessAbsent(agent: AgentType): string {
  return harnessAbsentOn(agent, "this machine");
}

/// And the same finding about a machine that is not this one, named.
///
/// The transfer preflight's: what it reads is another device's own answer about
/// its own `PATH`, and a second wording for *that harness is not there* would be
/// two sentences for one fact — the only difference between them being which
/// machine is being talked about, which is the argument.
export function harnessAbsentOn(agent: AgentType, where: string): string {
  return `${AGENT_NAME[agent]} is not on ${where}.`;
}

/// And what a machine that has stopped answering is called, which is the word
/// the sidebar and the Devices section both wear for the same finding: one dial
/// worked down that machine's addresses and reached none of them.
export const UNREACHABLE = "unreachable";

/// What a row with no login file at home says where it is *not* broken — which
/// is on the device the account is on.
///
/// A login kept in the macOS Keychain, and an account nobody has logged in to
/// yet, are both perfectly runnable here: what they cannot do is be lent to
/// another machine, there being no file to mirror. So the row says that and
/// nothing stronger, and the same account read as a mirror elsewhere is broken
/// there — see `Broken.NoLoginAtHome`.
export const NOT_USABLE_AWAY =
  "No login file in this account, so it cannot be used from another device. A login here is what gives it one.";

/// What is wrong with one profile, said in full.
///
/// `null` is a row with nothing in the way of running a session under it, which
/// is the ordinary case.
export function brokenReading(profile: ProfileEntry): string | null {
  return profile.broken === null
    ? null
    : troubleReading({
        broken: profile.broken,
        agent_type: profile.account.agent_type,
        device: profile.device?.name ?? null,
      });
}

/// The same sentence out of what a refused press carries, which is the same
/// three facts a row holds: the finding, the harness, and the machine the
/// account is at home on.
export function troubleReading(said: ProfileTrouble): string {
  const where = said.device ?? "the device it is at home on";

  switch (said.broken) {
    case "DirMissing":
      return "Its claude directory is gone.";
    case "ConfigMissing":
      return "Its config file is gone.";
    case "HomeMissing":
      return "The home it kept its account under is gone.";
    case "HomeUnreachable":
      return `${where} is ${UNREACHABLE}, so its account cannot be fetched and no session here can be built out of it.`;
    case "NoLoginAtHome":
      return `Its account on ${where} holds no login file, so there is nothing to mirror here. Log in on that machine.`;
    case "HarnessMissing":
      return harnessAbsent(said.agent_type);
    case "SignedOut":
      return `Its ${AGENT_NAME[said.agent_type]} account is signed out. Log in to run a session under it.`;
  }
}

/// And the same finding in as few words as a picker row has for it.
///
/// **Refused rather than hidden**: a row that cannot be run stays in every
/// picker, because a row saying why is something to go and put right and a row
/// quietly missing is a human looking for a Profile they know they saved. So
/// the row has to carry the why, and a picker has room for a word rather than
/// for the sentence above.
export function brokenBriefly(broken: Broken): string {
  switch (broken) {
    case "DirMissing":
    case "ConfigMissing":
    case "HomeMissing":
      return "account gone";
    case "HomeUnreachable":
      return UNREACHABLE;
    case "NoLoginAtHome":
      return "no login at home";
    case "HarnessMissing":
      return "not on this machine";
    case "SignedOut":
      return "signed out";
  }
}
