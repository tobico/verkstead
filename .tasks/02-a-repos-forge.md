# 02. A Repo's Forge

## What to build

A Repo's Forge is read off its remote's host at the moment it is needed — never
stored, never picked on the Repo's row. It is matched against the `hosts` every
**enabled** Forge declares, plus the hosts the human has added to a Forge
through a declared `list` settings field (a self-hosted instance is a host
added, not a plugin written). GitHub's manifest declares that field.

A Repo whose remote matches no enabled Forge has no Forge:
- no wrap-up is started for it, and the Conversation lands Done at the signal
  with the Timeline saying why;
- `verkstead done` accepts its signal the way it does when GitHub is out of
  reach, with no open-pull-request refusal;
- the Share pane has no Publish;
- the Repo's row and the Conversation's card say *No forge for <host>* where a
  pull request pane would have been drawn.

The generated viewer types carry the Repo's Forge, or its absence, so the
viewer reads it rather than guessing from the URL. Check `docs/copy.md` and
CONTEXT.md for the words.

## Acceptance criteria

- [ ] A Repo whose remote is on a host no enabled Forge declares lands Done at
      the signal with nothing watching, the Timeline saying why, and its row
      and card say *No forge for <host>*.
- [ ] Adding that host to GitHub's extra-hosts setting makes the next
      Conversation on that Repo wrap up. Disabling the Forge makes it match
      none again.
- [ ] A Repo on github.com reads exactly as before.
