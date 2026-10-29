# 03. Submit applies servers and ticks, and writes the record

## What to build

Submitting a Response applies the Set's pending servers and device ticks, in
the transaction that stores the Response, once. A pending server is attached
to the Conversation by name although the round is no longer drafting, this
being the second moment ADR-0028 adds beside ADR-0021's; a pending device is
permitted exactly as a tick under *May be transferred to* on the Steer would
be. Every question is asked before anything is stored: a server whose
declaration has gone since the pick refuses the Submit by name, with the
Answers kept on the sheet and the chip's × as the way out. Storing the Answers
and dropping the addition with a Notice was turned down.

The record moves from pending to applied, and the Timeline gains a line under
the Set's Event in the shape the Steer's announcement lines take, naming what
was added. A Steer Event with a frozen form is not what this is: nobody
steered.

The Response gains an `added` list beside `attachments`, filled on the way out
the same way, each entry naming the addition in words and where it is: for a
server its name and the prefix in front of its tools, for a device its name and
id. The waiting agent's `verkstead ask` prints it with the Answers, and
`verkstead answers` prints the same, so the one addition that restarts nothing,
a device tick, still reaches the agent that asked. The generated viewer types
follow the schema.

A Set locked unanswered, or closed, leaves its pendings unapplied. The settled
Set page draws applied additions as chips under the Answer, the way a frozen
Brief draws its servers, and unapplied ones struck through.

In this task nothing is restarted: a server attached here reaches the next
launch, which is the Deferred Ask's rule and the accepted intermediate state
until task 05. Companions are task 04's.

## Acceptance criteria

- [ ] Submit attaches each pending server and permits each pending device in the Response's transaction; a later launch is written with the server, and `verkstead transfer` to the device is allowed.
- [ ] The Response carries `added` beside `attachments`, filled on the way out; `verkstead ask` and `verkstead answers` both print it, naming each addition in words.
- [ ] A pending server no longer declared refuses the Submit by name, nothing is applied or stored, and the sheet keeps the Answers with the chip still removable.
- [ ] A line under the Set's Event on the Timeline names what was added, in the Steer's shape and to the copy rules.
- [ ] The settled Set page draws applied additions as chips and, on a Set locked or closed with pendings, the unapplied ones struck through.
