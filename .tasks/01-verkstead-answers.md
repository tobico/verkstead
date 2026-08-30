# 01. `verkstead answers`

## What to build

A second thing an agent can do with a Question Set: fetch the Response to one
it stored earlier, by id. The blocking ask hands a session its Answers by never
returning until they are there; a session that ended its turn instead has to be
able to come back for them, and this is how.

**One id in, one Response out.** `verkstead answers <id>` prints the Response as
YAML on stdout and exits 0, byte for byte what a blocking `verkstead ask` would
have printed for the same Set — so an agent parses the two the same way and the
Guide has one Response shape to describe. Nothing else is ever written to
stdout, which is the CLI's standing contract.

**Refused while there is nothing to fetch.** A Set nobody has answered yet is
not a wait to be opened here — this command polls once and comes back — so it
fails with a non-zero exit saying the Set is unanswered. A Set the human locked
unanswered fails saying no Response is coming, the same distinction the CLI's
wait already draws. An id that names no Set of this Conversation fails by name.
The server has the endpoint for all of this already; what the command adds is a
poll that does not hold and the wording of the three refusals.

**A fetch is a delivery.** The Answers reach the session that asked or a later
session's prompt, never both: a successful fetch records the Set as folded, so
the folding rule passes over a Set whose Answers the asking session has already
read. Recorded from the server, in the request that hands the Response over,
rather than by anything the CLI says afterwards. A refused fetch records
nothing.

**The scope is the Conversation's**, as every agent endpoint's is: the id comes
off the base URL the sandbox was given, so a Set belonging to another
Conversation names nothing here.

The Guide's own text is stage-02 task 02's; what this task owes it is the
command and its `--help`, which the Guide quotes verbatim.

## Acceptance criteria

- [ ] A stored, answered Set prints as Response YAML on stdout and exits 0,
      parseable exactly as the blocking ask's output is.
- [ ] An unanswered Set, a Set locked unanswered, and an id belonging to no Set
      of this Conversation each fail with a non-zero exit and a message saying
      which of the three it was.
- [ ] A successful fetch records the Set as folded, so its Answers do not also
      arrive under the next session's prompt; a refused fetch leaves the
      folding record alone.
