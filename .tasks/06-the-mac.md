# 06. The Mac

## What to build

The login design holds on a Mac only if a session cannot reach the Keychain,
so that agy falls to its file store there as it does under bwrap. Verkstead's
Mac sandbox is a deny-by-default seatbelt policy that today allows every mach
lookup, with a note that narrowing it is worth doing once. This is that once:
refuse the Keychain's services — `com.apple.securityd` and the
`com.apple.SecurityServer` family, measured rather than assumed — while leaving
what the harnesses need to resolve names and reach the network.

On cat.local: a session under an agy Profile started signed out shows the
sign-in screen rather than a Keychain prompt or a locked-keyring error; Log in
on the card signs the Profile in, the token landing in the account's file as on
Linux; a session then runs, asks, is judged idle and resumes under it. The
other harnesses' sessions on the Mac are unchanged by the narrowing.

## Acceptance criteria

- [ ] The seatbelt policy refuses the Keychain's mach services and nothing else new, and every existing Mac suite still passes.
- [ ] On cat.local an agy Profile signs in by the card, the token is a file in the account, and a session resumes under it.
- [ ] A Claude, Codex, Grok Build and OpenCode session each still launch on the Mac under the narrowed policy.
