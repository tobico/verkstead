# 03. Dividers between groups

## What to build

**Every border in the layout tree becomes a divider that drags.** A split node
has one between each pair of its children, and dragging it moves the shares on
either side of it: the group that grows takes exactly what the group that
shrinks gives, and nothing outside that split moves at all.

**The frame's own divider is the pattern.** It is a separator rather than a
button, because what it changes is a value rather than an action — the share of
the parent the group before it is worth — so it carries that value, it is
reachable from the keyboard, and the arrow keys nudge it by the same travel a
drag gives it. The difference here is that Code's splits go both ways: a
side-by-side split's divider moves left and right, and a stacked one's moves up
and down, so the orientation, the keys and the axis all follow the split's
direction. The frame's widths are the frame's own and are not the thing to
import — what is being copied is how a divider behaves.

**Clamped to a floor per group**, and the floor is a length rather than a
share, for the reason the frame's minimums are lengths: what makes a group too
narrow is what stands in it, and a bar of tabs over an editor is the same size
on every window. So what a floor is worth as a share is arithmetic against the
split it is measured in, and a divider dragged past it stops.

**Shares, so a resize keeps them.** The window changing shape moves no share:
the groups are drawn at the percentages they were left at, re-clamped against
the floors at the new size, which is the whole reason a divider settles a
percentage rather than a column.

## Acceptance criteria

- [ ] A drag moves the shares either side of one divider and leaves every other
      group where it was.
- [ ] The arrow keys nudge a focused divider along its own axis, and a stacked
      split's divider answers up and down rather than left and right.
- [ ] A drag past a group's floor stops at it.
- [ ] Resizing the window keeps the shares the dividers were left at.
