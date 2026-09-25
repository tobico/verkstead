# 03. The picker, drawn twice

## What to build

**A `Process` control in the setup row along the bottom edge of the composer's
box**, between the Repo trigger and the role pickers: a dimmed label over its
value like every other option in that row, labelled *Process*, and reading the
Process the Conversation is. One row in the list for now — Develop — and that
list is the one place a later stage adds to, a Process being offered only once its
stage has landed, as an agent type is offered only once it can launch the real
thing.

**Split the way the row's other controls are split**, because the setup is asked
in two places: the control and what it looks like exported from the shared
module, and what a pick *does* belonging to whoever draws it.

- On **a saved Draft's composer** it saves itself the moment it is touched, the
  way the Pairing pickers do, and says under itself whatever the endpoint refused
  — in the same shape a refused Repo switch is said in. Once a worktree exists the
  control reads settled and disabled, exactly as the Repo picker does, the press
  being refused by the server either way.
- On **the compose page** it writes into what the device is holding, beside the
  other fields, so a reload or a closed tab loses nothing and nothing reaches the
  server until a press. A page holding nothing but an untouched Process is still
  a page worth nothing, so the held state's *blank* and *empty* readings gain it
  with the rest.

**Develop is where it starts, and it is not remembered per Repo.** The Pairings
are remembered because they are the same answer most of the time; a Process is the
one thing about a Conversation likeliest to differ from the last. So the compose
page's control stands on Develop for every repo, and the create replay gains one
more step that runs **only where the human touched the control** — exactly as a
role picker left on the prefill sends nothing, which is what leaves the server's
own reading standing.

**On a Draft that took up a pull request the control is drawn disabled, reading
Review.** That is what the Conversation is; Review cannot be picked until its own
stage lands; and a row saying nothing about a Conversation that is a Review would
be the row missing the thing that tells it apart. The same over a pull request
loaded into the compose page, whose Process is as settled as its repo is.

## Acceptance criteria

- [ ] The compose page's setup row reads Repo, Process, then the role pickers,
      with Process on Develop — and a page whose control was never touched
      creates a Conversation with no Process request in its replay, while one
      where it was touched sends exactly one.
- [ ] A saved Draft's pick saves through the endpoint and the pane reads it back;
      the same control on a Conversation whose work has started is disabled, and a
      press that reaches the server anyway is refused with the sentence drawn
      under the control.
- [ ] A Draft holding a pull request draws the control disabled reading Review, on
      the composer and on the compose page alike.
