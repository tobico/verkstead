# 04. A login resumes the runs it unblocks

## What to build

A successful login — from the Notice or from the Profile card — resumes every
run on that Profile that is stopped as Signed out, through the harness's own
resume. Each resume builds a fresh root, which is what picks the new login up
(on Linux the old session's bind holds the old file).

## Acceptance criteria

- [ ] Two runs stopped as Signed out on one Profile both resume after one login
- [ ] Runs stopped for any other reason, or on another Profile, are untouched
- [ ] The resumed session is given the new login
