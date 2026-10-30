# 04. Verkstead's own Question Set

## Goal

Where the platform wanted has a device that is linked but not ticked, or
ticked but asleep, Verkstead asks rather than deciding — with a Question Set
of its own, the first it has ever authored. The Set names Verkstead as the
asker, says what wants the platform and why the device cannot be used yet, and
carries *Move to nuc-win* against *Run it here* where the device is not
permitted, and *Run it here* alone where it is permitted but asleep — the open
Set being the wait, which no answer closes. A ticked device that answers before
the human does moves the work and locks the Set with a line saying so. And `verkstead transfer` on such a device
is accepted as a request: the turn ends, the same Set goes up with *Keep it
here* as its second Option, *Move to* ticks and moves, and *Keep it here*
resumes the session with a note that the work stays. Demonstrable: a Windows
task on a Conversation with no Windows device ticked puts the Set, the phone
is told, and either press does what it says.

## Decisions in force

- **A Question Set, not a stop and not a card**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md), *Verkstead asks
  with a Question Set of its own*): an open Set already draws *Blocked on you*,
  reaches a phone, and has the Attach menu's tick.
- **The words are the ADR's table**, by `docs/copy.md`: a verb on the button,
  a full sentence naming the device above it, the notification naming the
  branch and the device.
- **A waking device settles the Set itself** — the move runs and the Set is
  locked, its line saying the device answered — because the tick already was
  the consent.
- **The agent's call becomes a request** rather than a refusal, on an unticked
  or sleeping linked device; a device off the cluster is still refused by name.
  *Keep it here* is the Carried Conversation's resume with a note, as an
  addition's carrying-on is (ADR-0028).
- **No device of the platform linked at all is a Notice**, and the work runs
  here, said once.
- **A Stage waiting on this Set takes a place**, as waiting on the human does.

## Proposed tasks (provisional)

1. **A Set with Verkstead as its asker** — stored and hung on the Timeline as a
   session's is, marked as Verkstead's, drawn with its asker named, and
   settled by a Response handler that runs the picked Option.
   - The Timeline card reads as asked by Verkstead; the phone is told.
2. **The gate** — stage 03's fallback Notice becomes this Set for the two
   askable cases; the no-device case stays a Notice.
   - *Run it here* launches where it is; *Move to* ticks and moves.
3. **The wake** — a sleeping device answering while its Set is open moves the
   work and locks the Set, saying so.
4. **The agent's request** — `verkstead transfer` accepts an unticked or
   sleeping device, ends the turn, puts the Set; *Keep it here* resumes the
   session with the note; the Guide and stage 01's prompt sentence say so.
   - A refused device is still refused in the words it is refused in today.

## Re-verify at start

- A Set is still stored and hung on the Timeline by `store::ask` in
  `crates/store/src/conversations.rs`, with no column for who asked; how it was
  asked is `crates/store/src/deferrals.rs`; locking is `lock_set` in
  `crates/store/src/lib.rs`.
- The agent's call is still `by_the_session` in
  `crates/server/src/transfers.rs`, refusing an unticked device and an
  unreachable one, with `AskedBy` in `crates/store/src/transfers.rs`.
- The resume with a note is still `resume::widened` and the Carried
  Conversation machinery in `crates/server/src/resume.rs`.
- Reachability is still `store::Member::reachable`, flipped by the dial in
  `crates/store/src/members.rs`; what wakes on a member answering is where the
  wake has to hook.
- Stage 03 landed, for the fallback this replaces.
