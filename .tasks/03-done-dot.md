# 03. The Done dot

## What to build

Verkstead pushes a notification when a wrap-up carries a conversation to Done,
but nothing in the UI reflects it. A Done conversation that sent that push gets
the sidebar's indicator dot until it is first viewed.

Nothing in the product records what the human has looked at, so this is a new
kind of fact, and it lives on the server so every device agrees:

- The wrap-up path that sends the Done push also stamps the conversation
  **unseen**, in the same breath.
- Opening the conversation in the workbench clears the stamp through an
  explicit call from the browser — a deliberate write, not a side effect of a
  read.
- While the stamp stands, the sidebar row draws the same accent disc that
  waiting-on-you draws: one mark meaning *look here*. The row's read-aloud
  label says why in words.

Scope is exactly the push path: a steer that moves a conversation to Done sends
no push and stamps nothing.

## Acceptance criteria

- [ ] A conversation the wrap-up carries to Done shows the accent disc on its
      sidebar row until it is opened, and on every device alike
- [ ] Opening it clears the dot everywhere; it never comes back for the same
      Done
- [ ] A steer to Done produces no dot
- [ ] The waiting-on-you disc is unaffected where both could apply
