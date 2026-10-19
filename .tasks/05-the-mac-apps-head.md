# 05. The Mac app's head

## What to build

The conversations pane's head inside the desktop app on a Mac, where the
window has no title bar and the traffic lights are drawn over the top-left
corner of the page — exactly where a 3rem collapsed pane puts its logo. Today
the frame pads the head at the window's left edge clear of the lights by the
inset the controls module measures; a 3rem pane cannot pad by 5rem.

**The head drops below the lights, collapsed and expanded alike** (Set 1265
Q19, in the human's words): wherever the frame measures a left inset, the
conversations pane's head takes no left padding and instead stands one band
lower, under a strip of paper the lights sit in. One rule for both states, so
the logo does not jump between beside the lights and below them as the pane
expands. **In full screen it stands at the top**, where the lights withdraw:
the controls module already answers no inset there, so the rule that reads the
inset reads that too and nothing has to ask about full screen by name.

**The lights stay where the app puts them** (Set 1270 Q23). The app centres
them on the head's first row by a measurement the page pushes to it, and that
does not change: the strip the head drops below is the band the page already
measures for the app, so the head pads itself down by its own band and the
lights sit centred in the paper above it. Nothing in the desktop app changes.
The band travels only inside the app, so a browser, a phone and the Windows
and Linux apps — whose controls are at the right — pad by nothing and move
nothing, which is the guard every controls rule already has.

The right-hand inset is untouched: the details pane's head on Windows and
Linux keeps clear of the controls overlay exactly as it does today.

There is no Mac in the dev shell; the arithmetic is proven in the vitest suite
by handing the frame a left inset and reading what the head is padded by, and
the real thing is proven the way Mac work is proven here, with a throwaway
workflow on a macOS runner where the screenshot is the proof. Keep the module
docs of the controls module, the head module and the frame's stylesheet true
to a head that drops rather than shifts.

## Acceptance criteria

- [ ] In the Mac app the logo stands below the traffic lights, collapsed and
      expanded, and nothing in the head is under a light.
- [ ] In full screen on a Mac the head stands at the top of the pane, as it
      does today.
- [ ] On Windows, Linux and in every browser the head is exactly where it was,
      and the right-hand inset still keeps the details pane's head clear.
- [ ] The traffic lights are at the height the app places them today.
