- **Idling** — `verkstead ask`. The Set is stored, the command returns at once
  with the `id` it was stored under, and **the turn ends there**. Nothing on
  this end waits. When the human answers, Verkstead types a line into this
  terminal saying so, and `verkstead answers <id>` is what fetches the Answers
  — see **Running the ask** below.
- **Deferred** — `verkstead ask --deferred`. The Set is stored the same way, and
  the session carries straight on rather than ending its turn. Nothing will ever
  nudge about it: the human answers in their own time, and their Answers are
  folded into the prompt of a later session of the same conversation — so
  nothing *this* session does will ever see them.

**Idle only on Questions whose Answers affect the work about to be done.** That
is the whole rule. "Which of these two shapes should the config take?" is worth
ending a turn for, when the config is what is being written now. "Is the wording
of this error message right?" is not: it is worth asking, the work does not turn
on it, and the answer reaches whoever picks the work up next.
