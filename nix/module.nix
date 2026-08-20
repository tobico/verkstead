# Verkstead as a systemd service: the server under its own user, with the CLI on
# every user's `PATH` so an agent working on the box can just call `verkstead`.
#
# The package is the flake's, so the module is a function of it rather than of
# `pkgs` — nothing here is in nixpkgs to be found by name.
self:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.verkstead;

  # systemd creates and owns this, and the database defaults inside it. Named
  # once because the sandbox, the working directory and that default all say it.
  stateDir = "/var/lib/verkstead";
in

{
  options.services.verkstead = {
    enable = lib.mkEnableOption "Verkstead, through which coding agents put questions to a human" // {
      description = ''
        Whether to run the Verkstead server as a system service, with the CLI on
        every user's `PATH`.

        The server binds the loopback interface and speaks plain HTTP.
        Reaching the web UI from a phone means HTTPS, which is
        `tailscale serve --bg 8422`'s job in front of it and stays host-level
        configuration rather than anything this module arranges.
      '';
    };

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.verkstead;
      defaultText = lib.literalExpression "verkstead.packages.\${system}.verkstead";
      description = ''
        The Verkstead package to run. One derivation carries both halves: the
        server this service starts and the CLI it puts on `PATH`.

        The default is the released binary, downloaded — so a host that imports
        this module needs no Rust toolchain and `nixos-rebuild` does not turn
        into a workspace compile. Building from the flake's own tree instead is
        `verkstead.packages.''${system}.verkstead-source`, which is what
        `nix flake check` proves.

        Until Verkstead's first release the two are the same thing: with the
        release manifest still empty there is nothing to download, so the
        flake hands out the source build under both names.
      '';
    };

    listen = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1:8422";
      example = "0.0.0.0:8422";
      description = ''
        Address and port the server binds, as `VERKSTEAD_LISTEN`.

        The default is the server's own: loopback, which is what
        `tailscale serve` proxies to. Binding a tailnet address instead reaches
        other devices directly, but over plain HTTP — which rules out the push
        notifications, since a service worker needs a secure context. That is
        what the `tailscale serve` proxy is for.

        The CLI's own default is `http://127.0.0.1:8422`, so a host that changes
        the port here has to set `VERKSTEAD_SERVER` for the agents alongside it.
      '';
    };

    database = lib.mkOption {
      type = lib.types.path;
      default = "${stateDir}/verkstead.db";
      defaultText = lib.literalExpression ''"${stateDir}/verkstead.db"'';
      description = ''
        SQLite file, as `VERKSTEAD_DATABASE`. Created, with its parent directory,
        on first run; it holds the Question Sets, the Archive, the push
        subscriptions and the VAPID keypair, so it is the whole of the service's
        state.

        The default is the server's own filename inside the service's state
        directory. Pointing it elsewhere means the sandbox has to be opened up
        for that path, which this module does by directory — so the directory
        has to exist, even though the file need not.
      '';
    };

    updateCheck = lib.mkOption {
      type = lib.types.bool;
      default = true;
      example = false;
      description = ''
        Whether the server asks GitHub, once a day, whether a newer Verkstead has
        been released — and shows the Update Notice in the web UI when one has.

        Nothing is ever installed on anyone's behalf either way: the Notice is a
        banner linking the updating instructions. Turning it off passes
        `--no-update-check`, and then no task runs and no request is made — the
        one thing this service says to anywhere but the push services.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    # The binary lands on `PATH`; an agent asks with it, and `verkstead serve
    # --help` is how a human finds out what this unit is passing it.
    environment.systemPackages = [ cfg.package ];

    users.users.verkstead = {
      isSystemUser = true;
      group = "verkstead";
      description = "Verkstead server";
    };
    users.groups.verkstead = { };

    systemd.services.verkstead = {
      description = "Verkstead — questions from coding agents to a human";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        # The flags rather than the environment variables behind them: what the
        # unit passes is then readable in `systemctl cat verkstead`, which is
        # where a human goes to find out what this service is actually running.
        ExecStart = lib.escapeShellArgs (
          [
            "${cfg.package}/bin/verkstead"
            "serve"
            "--listen"
            cfg.listen
            "--database"
            "${cfg.database}"
          ]
          ++ lib.optional (!cfg.updateCheck) "--no-update-check"
        );

        User = "verkstead";
        Group = "verkstead";

        # systemd makes the directory and hands it over already owned; the
        # service never creates it, and it survives a restart with the database
        # in it. Relative paths the server is given resolve here too.
        StateDirectory = "verkstead";
        StateDirectoryMode = "0750";
        WorkingDirectory = stateDir;

        # An agent is blocked on an answer whenever the server is down, so come
        # back rather than sit in a failed state.
        Restart = "always";
        RestartSec = "5s";

        # Hardening. Two things it must not break: SQLite in WAL mode, which
        # creates `-wal` and `-shm` beside the database and so needs a
        # read-write directory rather than just a writable file; and outbound
        # HTTPS — to the browser vendors' push services, whose addresses cannot
        # be enumerated ahead of time, and to GitHub for the update check. That
        # is why there is no `IPAddressAllow` here, and why stopping the update
        # check is `updateCheck = false` rather than a firewall rule.
        CapabilityBoundingSet = [ "" ];
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        PrivateUsers = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        ProcSubset = "pid";
        ProtectSystem = "strict";
        # AF_UNIX is the journal's socket and AF_NETLINK is what glibc's
        # resolver asks which interfaces exist over — neither is the server
        # reaching anywhere.
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
          "AF_UNIX"
          "AF_NETLINK"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];
        UMask = "0077";

        # `ProtectSystem = "strict"` leaves the state directory writable and
        # nothing else, so a database put elsewhere needs its directory saying
        # so. Under the state directory this would be redundant.
        ReadWritePaths = lib.optional (!lib.hasPrefix "${stateDir}/" "${cfg.database}") (
          builtins.dirOf "${cfg.database}"
        );
      };
    };
  };
}
