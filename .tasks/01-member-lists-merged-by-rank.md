# 01. Member lists in memory, merged by rank

## What to build

The sidebar stops being this device's work and becomes the cluster's: every
member's Conversations merged with this device's own by **Rank**, each row
saying which device owns it, and every press on a row addressed to that device.

**The hub keeps each member's list in memory** (ADR-0020, *The opened device
relays*). It is that member's own `/api/ui/conversations`, read over the
**Relay**, and it is refreshed rather than fetched per load: fanning out on
every read was rejected, because it costs a round trip per device per refresh
and a member that is down would stall the sidebar.

- Read when the Nudge stream to that member is taken up, which is where the
  server already holds a task per member and announces `Everything` under it.
- Re-read whenever a Nudge announced under that member is one the sidebar
  re-reads on locally — the viewer's own table is the list of them, and today
  that is `conversations`, `conversation`, `set` and `everything`. A member
  printing a line of transcript is not one of them.
- A member that answers nothing holds whatever it last said. A member never
  reached holds nothing and contributes no rows.

**`/api/ui/conversations` answers the merge**, ordered by rank across the lot.
Stage 05's suffix is what makes that safe: every rank names the device that
issued it, so the merged order is total without a tiebreaker and reads the same
from every device. The hub cannot merge rows whose keys it has not got, so the
rank rides out on the row.

**And it answers this device's own rows alone over the Peer Listener.** The
viewer's namespace is one router mounted twice, so a member reading this
endpoint would otherwise get a merge of merges, and two hubs would each claim
the other's rows as their own. The Nudge stream already makes exactly this
distinction and the way it knows is the extension the peer router puts beside
every request — `crate::peer::workbench::OverTheLink`.

**What a row says about its device**, which is two questions and so two levels:

    /// The device a row belongs to, or `None` where there is no cluster and
    /// nothing to say — a lone device draws no device on any row.
    pub device: Option<RowDevice>,

    pub struct RowDevice {
        /// The Device Id, or `None` for this device's own rows, which is the
        /// shape `reaching.ts` already keys queries and composes paths by.
        pub id: Option<String>,
        pub name: String,
        pub os: String,
        /// False is the row drawn dimmed, from the last list held.
        pub reachable: bool,
    }

The name and the OS word are the hub's own — it holds them on every member row
and answers them for itself — so nothing in the viewer has to join a row against
another reading to draw it. Whether the block is there at all is the server's
call rather than the page's, the membership being what decides it.

**A row is addressed by its device everywhere in the sidebar.** Ids are each
device's own and collide by construction, so a bare id is no longer a row:

- the list's reconcile key, the drag's held order, and the `data-id` a drag
  reads back off the DOM;
- which row is selected, which is a Conversation *and* a device now;
- the pressed-row overlay, which stage 04 already gave a `device` to and which
  still asks for this device's own on every row;
- opening a row, which goes to that device's own URL —
  `/devices/{device}/conversations/{id}`, and the local shape unchanged for
  this device's own;
- and the card's right-click menu, which acts on the Conversation the card
  stands for and must act on it *on its own device*. A menu that closed this
  device's Conversation 4 because a member's row said 4 is the bug this whole
  paragraph is about.

CONTEXT.md gains **Merged List**: what it is, that it is served from the hub's
memory and refreshed on Nudges rather than fanned out per load, that it is
ordered by Rank alone, and that a member that stops answering keeps its rows.

Drawing the device on the row is task 02's; this task draws the rows and the
repo line exactly as they read today. The archived switch still governs this
device's own rows alone, and a drag still saves through stage 05's route — tasks
03 and 04.

## Acceptance criteria

- [ ] A Conversation created on B is on A's sidebar within a Nudge, in rank
      order among A's own rows, and pressing it opens it at B's device URL.
- [ ] Two Conversations the two devices each numbered `1` draw as two rows, and
      each press — opening it, and each row of its menu — reaches the right one.
- [ ] B switched off leaves B's rows on A's list, from the last list held,
      carrying the flag that says the device is not answering.
- [ ] A asked for `/api/ui/conversations` over the Peer Listener answers A's own
      rows alone, with no device on them.
- [ ] CONTEXT.md defines the Merged List in the project's own vocabulary.
