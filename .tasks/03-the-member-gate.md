# 03. The member gate

## What to build

The certificate the peer listener's handshake accepted is handed through to the
routes, and middleware over the gated ones consults the member list
(ADR-0020, *The peer listener, and mutual TLS*). In this stage there is no
membership to consult — stage 02 is what creates a member — so the gate refuses
every caller, and what this task proves is that the *shape* is right: the
certificate reaches a route, the un-gated surface is exactly what it should be,
and everything else is refused for not being a member rather than for not
existing.

**Three routes stand outside the gate, and they are the whole of the un-gated
surface:**

- the **identity endpoint**, which asks for no certificate at all;
- the **join post**, which comes from a non-member by definition and whose
  certificate is pinned into the pending request it creates;
- the **dial-back** answering a join, matched against the certificate that
  pending request holds rather than against the member list.

The last two are stage 02's. This stage has the first and refuses the other two
along with everything else, there being no join to hold yet — so a caller that
reaches for either meets the gate's refusal, and the un-gated list has one entry
in it.

Everything else on this listener is a member's or is refused. The refusal says
what it is — a caller this device holds no membership for — rather than reading as
a missing path, so that stage 02's join has something to distinguish. Worth
looking at how the Workbench Key's gate words its own refusal and why it carries
no `WWW-Authenticate`: the same reasoning applies here, where the credential is a
certificate rather than anything the caller can be asked for after the fact.

The member list is whatever this stage can honestly give it — an empty one, read
from nowhere. Do not build stage 02's membership store for it: what the gate needs
is one question it can ask, and the answer today is always no.

## Acceptance criteria

- [ ] A call with no client certificate reaches the identity endpoint and nothing
      else on the peer listener.
- [ ] A call with an unknown client certificate reaches the identity endpoint too,
      and is refused by every gated route.
- [ ] The certificate the handshake accepted is readable by a route, so a later
      stage can pin it into a pending join.
- [ ] The refusal says the caller is not a member of this device's cluster, and
      is told apart from the answer a path that does not exist gets.
- [ ] The workbench listener's own gate is untouched: a request to the workbench
      is decided by the Workbench Key exactly as before.
