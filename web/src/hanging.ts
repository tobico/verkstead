//! Which side of what it came out of a dropped card hangs on.
//!
//! One rule for every dropdown on the page — the pickers' rows and the cards a
//! [`Menu`](./Menu.tsx) drops — because the two of them had come to answer it
//! differently: the pickers measured and went over a control near the foot of
//! the window, and the cards hung under it whatever was there, off the bottom
//! of the pane. A rule written twice is the drift `Menu` exists to stop.

/// Whether a card `asked` pixels tall should hang **over** an anchor standing
/// between `top` and `bottom` in the window, rather than under it.
///
/// Over only where it does not fit under, and then only where there is more
/// room over: the ordinary way round is the one to be in wherever being in it
/// costs nothing, and where neither side fits the card goes to whichever side
/// shows more of it. A card that went over whenever it did not fit under would
/// hang a tall one off the top of the window with the roomier side unused.
export function hangsOver(
  anchor: { top: number; bottom: number },
  asked: number,
  height: number = window.innerHeight,
): boolean {
  return anchor.bottom + asked > height && anchor.top > height - anchor.bottom;
}
