# 01. A pipe beside the socket

## What to build

On Windows the server opens a **named pipe** beside the TCP listener it already
has, and serves the same router over it. Everything a request can ask for over
the socket it can ask for over the pipe: it is one router, one database and one
Conversation-scoped namespace, reached two ways.

**The listener is a listener.** `axum::serve` takes anything implementing its
own public `Listener` trait — an `Io` that reads and writes, an `Addr`, an
`accept` and a `local_addr` — and it is implemented for a TCP listener and, on
Unix, for a Unix one. So the pipe half is a type of Verkstead's own implementing
that trait over `tokio::net::windows::named_pipe`, handed to a second
`axum::serve`. Nothing here needs hyper's connection API driven by hand.

A named pipe is not a socket, and the shape of accepting one is the thing to get
right: a pipe *instance* is created, then waited on for a client, and it *is*
the connection once one arrives. So the listener always holds an instance
created and waiting, and `accept` waits on that one, creates the next before
handing the connected one back, and never leaves the name with no instance
behind it — a client that dialled in that window would be refused for no reason.
An instance that will not create is the accept-error case the trait's own
documentation describes: log it and go round again rather than ending the
server.

**The name comes off the Data Directory**, so two Verksteads on one machine
running against two Data Directories open two pipes and neither disturbs the
other. The first instance is created as the *first* instance — the flag Windows
has for exactly this — so a second server pointed at one Data Directory is
refused by the pipe rather than quietly shadowing the first, which is the same
answer the TCP bind already gives for a taken address. The name is derived
rather than configured: nothing outside reads it except through what a session
is handed, which is task 03's.

**The security descriptor is an argument.** The pipe is created with a
descriptor the caller supplies, and what this stage supplies grants whoever runs
the server and nothing wider. The argument takes an extra identity beside that
one, and nothing passes it yet: it is the seam stage 03 fills with the
container's, and the whole reason the descriptor is decided here rather than
left to the platform's default. Written against the same `windows-sys` the
console and the junctions are — the security half of Win32 is another feature on
the same dependency, and that manifest's comment says what each feature is for
and has to go on saying it.

**Where it starts.** `run_on` resolves the Data Directory and ends on a single
`axum::serve(...).await`; the pipe wants the Data Directory, so it is made after
that and served beside the socket, with either one ending being the server
ending. There is no graceful shutdown to fit into — the process stopping is the
whole of stopping — and adding one is not this task's. The startup line already
says the address and the Data Directory; it says the pipe too, in a spelling a
human could paste.

Linux and macOS get nothing: no pipe, and no Unix-socket twin, because nothing
on those platforms needs one.

## Acceptance criteria

- [ ] On Windows, a request made over the pipe returns what the same request
      over TCP returns — including one under a Conversation's own base.
- [ ] Two servers on two Data Directories open two pipes and each answers only
      its own; a second server on one Data Directory is refused rather than
      taking the name over.
- [ ] The pipe's descriptor grants the account the server runs as and nothing
      wider, read back off the pipe itself — a runner has one account, so this
      is asked of the descriptor rather than by connecting as somebody else —
      and it takes a further identity as an argument that nothing passes yet.
- [ ] Linux and macOS builds compile and behave exactly as before, and the
      startup line on Windows names the pipe.
