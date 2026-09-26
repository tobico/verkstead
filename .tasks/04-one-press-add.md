# 04. One-press Add

## What to build

**Add** on a discovered row links the two devices with nothing typed.

The press names the row by its **Device Id** rather than by an address, because
discovery found a list of them: mDNS resolves one, and the identity a probe read
carries every address that device advertises. So the server dials the list it
holds, in the order it found it, exactly as a dial to a **Member** works down
that member's addresses — and stage 02's typed box keeps the single address it
has always had, what somebody types being only the first address ever known.

What a press leaves is what the typed **Add** leaves: a **Join** posted, the
question held on the far end for its ten minutes, and a pending row here reading
*waiting for confirmation on* that device with this device's own fingerprint
under it and a Cancel. The discovered row goes with the press, because a device
with a Join pending is one task 02 leaves out — so the press moves a row from one
list to the other rather than leaving two rows about one device.

**And a row can be stale by the time it is pressed.** The device may have gone
off the LAN or left the tailnet between the browse finding it and somebody
pressing it, so the press is refused in the words a dial that reached nobody
puts it in, naming the device, rather than as a bare failure.

## Acceptance criteria

- [ ] A press on a discovered row leaves the pending row stage 02 draws, and the discovered row is gone from the list.
- [ ] A device whose first address no longer answers is joined at the next address discovery found for it.
- [ ] A press on a row whose device has gone is refused in words naming the device, and the next read of the list is without it.
- [ ] Allowing the Join on the far end makes the device a Member on both, drawn as one rather than as discovered.
