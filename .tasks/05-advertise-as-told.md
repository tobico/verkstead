# 05. Advertise as told

## What to build

An Annex advertises its Keeper's addresses at the forward port rather than its
own interfaces (ADR-0024; CONTEXT.md, **Annex Forward**), so a third device
records it at an address it can reach — and, every successful dial
overwriting a member's addresses with what it advertises, keeps recording it
there.

- **A peer route on the Keeper**, behind the member gate, answering the
  Keeper's own advertised addresses at its forward port: a bare IP becomes
  `ip:8424` (`[ip]:8424` for v6) and the tailnet name `name:8424`, which the
  dialler already reads as an address with a port. Only a device keeping an
  Annex answers it.
- **The Annex's Device Reading asks it at each reading**, of the Keeper named
  in `kept_by`, dialled pinned at the Keeper's member addresses, and
  advertises the answer **instead of** its own interfaces and tailnet name.
  This is a new arm of the private address chooser in `device/reading.rs`,
  beside the fixed list and the interface read — `Reading::advertising` is
  only a suites' constructor. An answer is held the way the tailnet half is
  held, so a reading is not a dial each time.
- **When the Keeper cannot be reached**, the Annex advertises the last answer
  it had, and nothing before the first. (Decided at planning.)
- The identity endpoint, the join body, announcements and the Devices *this
  device* row all read the Reading, so they all follow.

Task 01 already keeps an Annex off mDNS; this is the other half of *its
address is its Keeper's*.

## Acceptance criteria

- [ ] After the Keeper's Join, B's identity endpoint lists A's addresses at
      8424 and none of its own.
- [ ] Three servers: C, told about B by A, records B at A's address on 8424,
      dials it there through the forward, and the handshake is B's
      certificate; B is still at that address on C after C's identity dial
      refreshes its row.
- [ ] With A stopped after one answer, B still advertises that answer; a B
      that has never reached A advertises no addresses.
