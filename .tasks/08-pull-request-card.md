# 08. Pull request card

## What to build

The pull request timeline card opens its details pane by clicking anywhere on
the card, not just the title text. The *On GitHub* link comes off the card —
GitHub stays reachable through the details pane, which already shows the URL.

The card currently nests an anchor beside a title-button, so making the whole
card the press means restructuring it: one card, one target, in both places it
is drawn (pinned and on the record row). Selection styling and the
blocked-on-you fronting behaviour stay as they are.

## Acceptance criteria

- [ ] Clicking anywhere on the card selects the event and opens the details
      pane, in the pinned deck and on the record row alike
- [ ] No GitHub link on the card; the details pane still links to the pull
      request
- [ ] Keyboard and screen-reader access to the card survives the restructure
