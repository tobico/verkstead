# 08. The shortcuts

## What to build

**Four keystrokes, and they are the four a browser lets through.**
Ctrl+PageDown and Ctrl+PageUp move to the next and previous tab of the active
group, wrapping at each end. Ctrl+\ splits the active group to the right, the
same split its bar's icon and its tab menu make. Ctrl+` opens a terminal in the
active group, the same shell **New terminal** opens. Ctrl+W and Ctrl+Tab are
the browser's own and are deliberately not taken — a page that swallowed either
would be a pane fighting the window around it.

They hang where Ctrl+S already hangs: on the document, for as long as Code is
mounted, which is the whole of their reach. So a press arrives whichever half
of the pane the hands were on, and what it acts on is the **active group**
rather than whatever happens to have focus.

**A terminal's own keys stay the terminal's.** A grid with focus is a shell
being typed into, and the three keystrokes the terminal window already claims
for itself are the whole of what it takes from one: everything else goes up the
socket. Ctrl+C interrupts, Ctrl+D ends, Ctrl+L clears — none of them is
touched, and nothing added here may start swallowing them.

## Acceptance criteria

- [ ] Ctrl+PageDown moves to the next tab of the active group and wraps at the
      end; Ctrl+PageUp goes the other way.
- [ ] Ctrl+\ splits the active group to the right, and Ctrl+` opens a terminal
      in it.
- [ ] Ctrl+C in a focused terminal still interrupts, and nothing added here
      swallows a key the grid already had.
- [ ] Ctrl+W and Ctrl+Tab reach the browser untouched.
