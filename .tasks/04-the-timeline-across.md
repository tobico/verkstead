# 04. The Timeline and everything hanging off it

## What to build

The record's slice, serialised on the source, sent over the link and written on
the far end under new local ids — so the Timeline there reads as it read here.

What travels is everything about *this Conversation* and nothing about the
machine it was on. The store's own schema tour is the list to work down: the
Timeline Events themselves, the Question Sets and Responses asked from them
with their deferrals and deliveries and endings, the Captures, Transcripts,
session names, session Pairings and session endings that hang off a session's
Event, the Steers and the stops and escalations, the commits recorded, the pull
request and wrap-up bookkeeping, the unseen mark, the shares, and the
Attachments with their files. What does not travel is the device's own: Repos,
Agent Profiles, the Repo's Pairing memory, Members, joins, push subscriptions
and the Remote access banner.

Every id is remapped as it lands, because both ends issue their own and every
Verkstead has a Conversation 1. An Event id referenced by a Capture, a
Transcript, a session Pairing or a Set has to come out pointing at the Event it
actually landed as. A Set's own id moves too, and a Set left open has to be
answerable on the far end afterwards — its Response reaching whatever session
is waiting there, rather than a session on a machine the work has left.

Attachments are rows and bytes both. The rows travel with the slice and the
files travel beside them, landing in the far end's own attachments directory
under its own Conversation id, so a session there is given the paths its own
sandbox expects.

A slice is prose and binary and can be large — a Transcript is megabytes of a
session talking. Carry it under a bound of its own and refuse whole rather than
part way, the way a memory store and an account mirror are bounded, and say
which Conversation was too big to move.

## Acceptance criteria

- [ ] The Timeline on the far end reads as it did on the source: the same
      Events in the same order, each drawing the card it drew, with its
      Captures, Transcripts and session names behind it.
- [ ] A Question Set left open on the source is answerable on the far end, and
      its Answers reach a session running there.
- [ ] An Attachment opens on the far end, at a path that device's own sessions
      are given, with the file's bytes unchanged.
- [ ] Nothing of the source machine crosses: the far end's Repos, Agent
      Profiles, Members and Repo Pairing memory are exactly what they were.
