# 09. The slow session-account tests

## What to build

After task 08 every Windows leg still runs 8 to 13 minutes (run 40709146514:
8m22 to 12m23). About six of those minutes are setup and build, which every
leg pays. What sets the slowest legs on top of that is one test each from the
suites `.config/nextest.toml` holds serial in the `session-account` group.
Those tests come late on their leg (position 347 of 352), so one long one
becomes the leg's tail:

- `logins_windows::a_login_left_past_its_limit_is_killed`: 227 s (leg 7)
- `logins_windows::a_code_handed_to_the_login_leaves_the_login_in_the_account`: 192 s (leg 6)
- `logins_windows::the_last_device_closing_the_modal_kills_the_login`: 121 s (leg 8)
- `sandbox_windows::a_grok_login_saved_by_rename_is_the_accounts_afterwards`: 103 s (leg 1)
- `sandbox_windows::a_fresh_account_is_given_settings_that_skip_the_bypass_consent`: 78 s (leg 10)
- `sandbox_windows::a_grok_session_is_given_a_root_of_the_allowlist_and_nothing_else` and `…a_grok_root_without_memory_has_a_store_of_its_own`: 62–65 s
- in-crate `sandbox::granting::writing::tests::a_granted_path_is_read_as_the_account_and_a_refused_one_is_not`: 70 s

Find out where each one spends its time: a cold first logon as the session
account, a fixed wait, or a poll with a long interval. Then take that time out
test-side, as the human chose (Set 44, Q2). The fixture work stays in the
tests, and the database and production code stay as they are. Where the cost
cannot come out of the test, the order can. A slow serial test that starts
first on its leg instead of last runs beside the rest of the leg rather than
after it. Check whether nextest can be told that, through a test-group or
priority override in `.config/nextest.toml`.

To get per-test times, read the job log with `gh api --allow-escape-sequences
repos/tobico/verkstead/actions/jobs/<job>/logs`, strip `\x1b\[[0-9;]*m`, and
sort the `PASS [ N.NNNs]` lines.

Record the run that carries this commit as a ledger row in the Windows job's
comment, in the same columns as the rows before it.

## Acceptance criteria

- [ ] No session-account test takes more than 60 seconds on a leg, or the ledger says which one still does and why it cannot come down test-side
- [ ] Every Windows leg and the Linux job are green
- [ ] The ledger carries a row for the run that measured this commit
