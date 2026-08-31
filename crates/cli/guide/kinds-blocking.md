- **Blocking** — `verkstead ask`. The session idles until the Response comes
  back, so the Answers are in front of it when it goes on.
- **Deferred** — `verkstead ask --deferred`. The Set is stored, the command
  returns at once, and the session carries on without it. The human answers in
  their own time, and their Answers are folded into the prompt of a later
  session of the same conversation — so nothing *this* session does will ever
  see them.

**Block only on Questions whose Answers affect the work about to be done.** That
is the whole rule. "Which of these two shapes should the config take?" blocks,
when the config is what is being written now. "Is the wording of this error
message right?" does not: it is worth asking, the work does not turn on it, and
the answer reaches whoever picks the work up next.
