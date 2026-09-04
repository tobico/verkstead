# The module's `shell` option, evaluated rather than booted.
#
# What it settles is a single line of the passwd file, and that line is the whole
# of what a Conversation's Terminal comes up in: the server reads its own user's
# login shell and runs it, so a service user left with `nologin` is a workbench
# where every terminal falls back to `/bin/sh`.
#
# An evaluation and no machine. The VM test next door is what proves a unit that
# runs, and it costs minutes; this asks what the module *says* — the default, the
# user it lands on, and that a shell of somebody's own still evaluates — which is
# an evaluation's question rather than a boot's.
{
  lib,
  runCommand,
  bash,
  fish,
  # The flake's own module, which closes over the flake's package, and the
  # evaluator to put a configuration through — neither is anything `pkgs` holds.
  module,
  nixosSystem,
  system,
}:

let
  # The `verkstead` user's login shell, in a configuration holding `chosen` and
  # otherwise the least the module will evaluate with: it refuses a build with no
  # Watched Path, and nothing else here is asked for.
  #
  # The package, which is what the option holds; NixOS writes the path inside it
  # into passwd, `shellPath` being the attribute that says which file that is —
  # and a package without one is refused by the option's own type, so a shell
  # that could not end up in passwd never evaluates this far.
  passwd =
    chosen:
    (nixosSystem {
      inherit system;
      modules = [
        module
        {
          services.verkstead = {
            enable = true;
            watchedPaths = [ "/srv/repos" ];
          }
          // chosen;
        }
      ];
    }).config.users.users.verkstead.shell;

  # One complaint per thing that came out other than as said, or nothing at all.
  said =
    what: got: want:
    if "${got}" == "${want}" then [ ] else [ "${what}: said ${got}, expected ${want}" ];

  complaints =
    (said "the default" (passwd { }) bash)
    ++ (said "fish, where somebody chose it" (passwd { shell = fish; }) fish);
in

runCommand "verkstead-module-shell" { } (
  if complaints == [ ] then
    "touch $out"
  else
    ''
      ${lib.concatMapStringsSep "\n" (complaint: "echo ${lib.escapeShellArg complaint} >&2") complaints}
      exit 1
    ''
)
