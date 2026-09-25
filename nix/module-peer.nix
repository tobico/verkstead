# The module's `peerListen` and `openFirewall` options, evaluated rather than
# booted.
#
# The VM test next door brings a Verkstead up on the port these name and reads
# its identity endpoint back over TLS, which is what proves the option reaches
# the running server and that the rule it opens is in the live ruleset. What it
# cannot show is the other half of a switch: one machine has the firewall open
# or shut, never both, and a second machine booted to prove a rule *absent*
# would be minutes of VM for an answer the evaluation already holds.
#
# So this asks what the module *says* — the defaults, the port the rule is made
# out of, and what an `openFirewall = false` host is left with — the way
# `module-shell.nix` beside it asks after a line of passwd.
{
  lib,
  runCommand,
  # The flake's own module, which closes over the flake's package, and the
  # evaluator to put a configuration through — neither is anything `pkgs` holds.
  module,
  nixosSystem,
  system,
}:

let
  # A configuration holding `chosen` and otherwise the least the module will
  # evaluate with, which is `enable` and nothing else at all.
  evaluated =
    chosen:
    (nixosSystem {
      inherit system;
      modules = [
        module
        {
          services.verkstead = {
            enable = true;
          }
          // chosen;
        }
      ];
    }).config;

  # What the unit runs, as `systemctl cat` would print it.
  #
  # The context is discarded deliberately: the package's store path is inside
  # that string, so a complaint quoting it would make this check *build* the
  # server before it could print — minutes of compiling to say a flag came out
  # wrong. Nothing here runs the command; it is only read.
  command =
    chosen:
    builtins.unsafeDiscardStringContext (evaluated chosen)
      .systemd.services.verkstead.serviceConfig.ExecStart;

  # And the ports this host would answer on.
  ports = chosen: (evaluated chosen).networking.firewall.allowedTCPPorts;

  # One complaint per flag that did not come out on the command line, spelled
  # with the same escaping the module built it with — so that the day the
  # quoting changes, what is looked for changes with it rather than quietly
  # stopping matching.
  passes =
    what: chosen: flag: value:
    let
      wanted = lib.escapeShellArgs [
        flag
        value
      ];
    in
    if lib.hasInfix wanted (command chosen) then
      [ ]
    else
      [ "${what}: no ${wanted} in ${command chosen}" ];

  # And one per set of ports other than the set said.
  opens =
    what: chosen: want:
    let
      got = ports chosen;
    in
    if got == want then [ ] else [ "${what}: opens [${toString got}], expected [${toString want}]" ];

  complaints =
    # The peer listener's address, defaulted and chosen. Every interface by
    # default, because the device dialling this one is on neither the loopback
    # nor anywhere this host can name ahead of time.
    (passes "the peer listener's default" { } "--peer-listen" "0.0.0.0:8423")
    ++ (passes "a peer address of somebody's own" {
      peerListen = "127.0.0.1:9423";
    } "--peer-listen" "127.0.0.1:9423")

    # And the workbench's, which this task left alone: an option that changed
    # how the listener already there is passed would be a packaged install
    # answering somewhere else than it used to.
    ++ (passes "the workbench listener's default" { } "--listen" "127.0.0.1:8422")
    ++ (passes "a workbench address of somebody's own" {
      listen = "0.0.0.0:8422";
    } "--listen" "0.0.0.0:8422")

    # The firewall, which is on unless somebody turns it off: a peer listener
    # nothing can reach is a linking that cannot happen.
    ++ (opens "by default" { } [ 8423 ])
    ++ (opens "for a listener somebody moved" { peerListen = "0.0.0.0:9423"; } [ 9423 ])
    ++ (opens "for one written as IPv6" { peerListen = "[::]:9423"; } [ 9423 ])
    ++ (opens "on a host that turned it off" { openFirewall = false; } [ ]);
in

runCommand "verkstead-module-peer" { } (
  if complaints == [ ] then
    "touch $out"
  else
    ''
      ${lib.concatMapStringsSep "\n" (complaint: "echo ${lib.escapeShellArg complaint} >&2") complaints}
      exit 1
    ''
)
