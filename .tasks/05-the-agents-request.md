# 05. The agent's request

## What to build

`verkstead transfer` naming a linked device that is not permitted, or permitted
but asleep, becomes a request rather than a refusal.

Today the server's `by_the_session` refuses an unticked device (telling the agent
to ask with `verkstead ask`) and a device whose preflight finds it unreachable.
Instead, for those two:

- the call is **accepted** and the turn ends, as an accepted transfer's does;
- **Verkstead's own Set goes up** with the ADR's agent rows — *The agent asked to
  move to nuc-win, which is not permitted.* with *Move to nuc-win* · *Keep it
  here*, or *…, which is not awake, and the work moves there by itself once it
  is.* with *Keep it here* alone — waited on as task 02's, woken as task 03's,
  and an unticked asleep one chained as task 04's;
- **Move to** ticks and moves, as task 01's;
- **Keep it here** carries the session on with a note saying the work stays here
  — the Carried Conversation's resume, as an addition's carrying-on is
  (ADR-0028; the resume module's `widened` and the carrying `Occasion`s, with a
  note of its own beside `skills::widened`).

**Still refused, in today's words**: a name that is nobody's or is two devices',
this device, a device that answers lacking the Repo, a companion or the harness,
and a Verkstead in no cluster.

**The words that tell agents** say the new route: the Guide's *Moving the work
to another device*, the prompt's devices-of-the-cluster sentence (stage 01's,
in `skills`), and any skill that tells an agent to ask for a tick first.

## Acceptance criteria

- [ ] `verkstead transfer` to an unticked linked device exits 0, the session
      ends at the turn's end, and the Set asked by Verkstead goes up; *Move to*
      ticks and moves the work, *Keep it here* carries the session on here with
      the note in its prompt.
- [ ] To a ticked device that is asleep, the Set has *Keep it here* alone, and
      the device answering moves the work.
- [ ] Unknown, ambiguous and lacking devices are refused in the words they are
      refused in today; the Guide and the prompt say the new route.
