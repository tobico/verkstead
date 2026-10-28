# 01. The uuid everywhere

## What to build

Every Agent Profile is keyed by a **uuid**, and every reference to a Profile
names it by that uuid. The local integer id goes. ADR-0028 turned down keeping
it as the row key with the uuid beside it, so it must not survive as a second
key.

Profiles are still the `profiles` table with its mirror rows in this task. The
move onto stage 01's Replicated record facility is tasks 03 and 04. This task
changes only what a Profile is *named by*.

**At the switch** (a migration, run once):

- Each of this device's own Profiles gets a freshly minted uuid. It also keeps
  **where it was minted and the id it had there**: this device's Device Id and
  the integer id the row had. That pair is the bridge task 02 matches mirrors
  by. Keep it on the row from now on; task 03 carries it into the record.
- Each mirror row gets a **provisional uuid** of this device's minting. That
  means its Pairings are already uuids, and task 02 rewrites it to the home's
  real one. The mirror keeps its home device and the id it had at home.

**Every reference converts**, and the brief's list plus what re-grounding found
is the whole of it:

- the Conversation's three Pairing columns (grilling, implementation, review)
  and `stopped_signed_out`, which says whose account signed out
- `repo_pairings`, the Repo's memory of its last Pairing
- `pending_steers` and the **recorded steers** (`steers.profile_id`, which the
  Timeline draws by reading the Profile's current name). The human chose to
  convert the recorded steers rather than let them draw nothing (Set 1476).
- `profile_models` and `profile_homes`
- the transfer slices' remapping of steers and pending steers
- the account mirror directory, which becomes `accounts/<uuid>` under the Data
  Directory
- the login home, which becomes `homes/login-<uuid>`
- **the Windows login boundary**, which today is the negative Profile id. A
  uuid cannot be negated, so it needs another key that can never collide with
  a Conversation's boundary id or the Compile Server's zero. Read the
  boundaries module and pick one.
- the logins and signouts maps held in memory
- the peer `{profile}` routes (the account fetch, the login write-back, and
  the memory sync's two routes), now addressed by the uuid the answering
  device holds the Profile under
- `ProfileAcross`, which becomes the uuid alone, and `here()` in the peer
  transfers code, which resolves it
- `Nudge::Login`, the Log-in Notice, `ProfileChoice` and `ProfileEntry.id`
- the viewer: the pairing value's `Number(...)` parse, the settings route's
  `profileOpened`, the card's and the pane's `number` types, and `LoggingIn`

**The mirror refresher keeps working** within the mirror model. It reads each
member's `/api/ui/profiles`, whose `id` is now the member's uuid. A mirror made
from that read **takes the member's uuid as its own**, so one Profile is one
uuid on both ends, and the mirror also keeps its home device. Only a mirror
held from before the switch has a provisional uuid of this device's own. The
account and memory fetches address the home by the uuid. A member read for the first time after the
switch finds no match for a mirror held under the old integer, so for now its
mirrors are re-made and their Pairings dropped. Task 02 is what keeps them.

Both ends on this release resolve one Profile to the same uuid: a Transfer
names a device's own Profile by its own uuid, and the far end's mirror of it is
held under that uuid. The Preflight does not compare Profiles today. They are
resolved on arrival, and that stays as it is.

## Acceptance criteria

- [ ] After the switch, a Conversation's three Pairings, a Repo's pairing
      memory, a pending steer and a recorded steer all draw the same Profiles
      they drew before, and a signed-out stop's Log in opens the same Profile.
- [ ] A session under a Profile of this device's own launches, and a login
      (including on Windows, through the boundary) runs and is taken back.
- [ ] A Transfer between two Verksteads on this release names the Profile by
      uuid alone, and the far end resolves it to its mirror of that Profile.
- [ ] A session on B under a mirror of A's Profile still fetches the account
      and the memory from A, addressed by uuid, and writes the login back.
- [ ] The viewer picks, edits, removes and logs in to Profiles by uuid.
