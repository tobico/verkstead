# Device identity and the peer listener

A Verkstead knows what it is and can be asked. At first start it invents a
device id and a self-signed certificate beside the Workbench Key in the Data
Directory, re-issuing the certificate as its expiry comes near; it listens on a
TLS peer listener of its own, every interface, port 8423, that presents that
certificate and asks the caller for one without insisting on it; an un-gated
identity endpoint on that listener answers with the device's id, name, OS and
addresses; and the **Remote access** pane gains a **Devices** section whose list
holds this device alone — name with an OS icon, *this device*, its addresses, no
Unlink.

This is the ground every later stage of cluster mode stands on: linking is two
devices pinning each other's certificate, discovery is finding an identity
endpoint to ask, and the relay is a member's call over this listener. Nothing
here links anything — there is no member to hold yet, so the gate over every
route but the identity endpoint refuses everyone, and a re-issued certificate has
nobody to announce itself to and says so. Demonstrable end to end: start two
servers, curl one's identity from the other's machine, open Remote access on each
and see a WSL read as *Linux (WSL)*.

Roadmap stage: [01: Device identity and the peer listener](docs/roadmaps/cluster-mode/01-device-identity-and-the-peer-listener.md)

## Tasks

- [ ] 01: Identity on disk — [details](01-identity-on-disk.md)
- [ ] 02: The peer listener — [details](02-the-peer-listener.md)
- [ ] 03: The member gate — [details](03-the-member-gate.md)
- [ ] 04: The machine reading — [details](04-the-machine-reading.md)
- [ ] 05: The option and the VM test — [details](05-the-option-and-the-vm-test.md)
- [ ] 06: The Devices section — [details](06-the-devices-section.md)
- [ ] 07: The renewal — [details](07-the-renewal.md)
