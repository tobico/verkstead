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

  # The directory half of a `sandboxBinds` entry: a plain path is the whole of
  # it, and `name=path` is the part after the `=` — the same rule the server
  # reads them by, so what the unit binds in is what the server hands out.
  bindPath = bind: if lib.hasPrefix "/" bind then bind else lib.last (lib.splitString "=" bind);
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

    watchedPaths = lib.mkOption {
      type = lib.types.listOf lib.types.path;
      example = lib.literalExpression ''[ "/home/you/src" ]'';
      description = ''
        The directories Verkstead is permitted to operate inside, as
        `--watched-path`.

        A security boundary rather than a convenience: nothing outside these
        directories is ever touched, and a Repo is registered only from within
        one. There is no default and no scan — the server refuses to start until
        this says what it may have, because guessing at what a machine's owner
        meant to expose is not a guess worth making.

        Each is resolved at startup, so it has to exist; symlinks and `..` are
        taken out of every path checked against them, and a path that merely
        reads as inside one is refused.

        Each is bound into the service's sandbox, and nothing beside it is: the
        hardening otherwise leaves nothing but the state directory reachable,
        so a watched path under `/home` would be one the service cannot see at
        all. What the sandbox exposes is therefore exactly this list.
      '';
    };

    sandboxBinds = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = lib.literalExpression ''
        [
          "/var/cache/shared"
          "verkstead=/var/cache/verkstead-cargo"
        ]
      '';
      description = ''
        Extra read-write directories every sandboxed session gets, as
        `--sandbox-bind`. A plain path is given to every session; `name=path`
        is given only to sessions working in the Repo registered under that
        name, so a repository that needs a build cache can have one without
        every repository getting it.

        This is the Sandbox Configuration. A session otherwise sees its own
        worktree, its Repo's git directory and its Agent Profile's claude pair
        and nothing else of the machine, so each entry here is a hole in that
        boundary — which is why it is set at installation, beside the watched
        paths, rather than anywhere the web UI can reach.

        Each is bound into the service's own sandbox as well, for the reason
        the watched paths are: the hardening leaves nothing but the state
        directory reachable, and a directory the service cannot see is not one
        it can hand to a session. A path that is not there refuses startup.
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
    # Said here as well as by the server itself, because a boundary nobody drew
    # is worth refusing at build time rather than at the first start.
    assertions = [
      {
        assertion = cfg.watchedPaths != [ ];
        message = ''
          services.verkstead.watchedPaths must name at least one directory.
          Verkstead operates only inside the directories it is given, so with
          none of them it has nothing it may touch and the server refuses to
          start.
        '';
      }
    ];

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
          # One flag per directory rather than the `:`-separated form the
          # environment variable takes: a path with a colon in it would split
          # in two, and this is the list that says what may be touched.
          ++ lib.concatMap (path: [
            "--watched-path"
            "${path}"
          ]) cfg.watchedPaths
          ++ lib.concatMap (bind: [
            "--sandbox-bind"
            bind
          ]) cfg.sandboxBinds
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
        # `tmpfs` rather than `true`: both leave the home directories empty, and
        # only this one composes with the `BindPaths` below. systemd.exec(5) says
        # it outright — a bind mount cannot be nested under `/home` with
        # `ProtectHome=yes`, and `ProtectHome=tmpfs` is what to use instead.
        ProtectHome = "tmpfs";
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

        # The Watched Paths, and nothing else of the filesystem they sit in.
        #
        # Bind mounts rather than `ReadWritePaths`, because a Watched Path is
        # usually somewhere under `/home` and `ProtectHome` replaces that with
        # an empty tmpfs: a path merely permitted under one is a path that is
        # not there to permit. Bound in, it exists inside the namespace and
        # nothing beside it does — which is the sandbox saying exactly what the
        # server says, rather than something wider that the server then narrows.
        #
        # Not prefixed with `-`, so a directory that has gone missing fails the
        # unit. The server refuses to start on one too; both of them saying so
        # beats a service that comes up watching nothing.
        #
        # The Sandbox Configuration's binds come in the same way and for the
        # same reason: what the service cannot see it cannot hand to a session.
        BindPaths = map (path: "${path}") cfg.watchedPaths ++ map bindPath cfg.sandboxBinds;
      };
    };
  };
}
