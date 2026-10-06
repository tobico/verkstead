# 02. A rejected code starts again

## What to build

When `claude auth login` refuses the code, the modal says so in plain words and
starts a fresh login with a new URL, without the human closing anything. First
find out what the real claude 2.1.283 does with a wrong code — exits with an
error, or asks again — and handle that shape; write down which it was beside
the code that reads it.

## Acceptance criteria

- [ ] The real claude's behaviour on a wrong code is checked and recorded
- [ ] With a fake `claude` that rejects a code, the modal shows the refusal and
      a new URL, and a correct code then succeeds
- [ ] Still one login process per Profile throughout the retry
