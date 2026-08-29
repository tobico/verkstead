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

  # systemd creates and owns this, and it is what the server is given as its
  # Data Directory: the database and everything else Verkstead makes live in it.
  # Named once because the sandbox, the working directory and `--data-dir` all
  # say it.
  stateDir = "/var/lib/verkstead";

  # And what the server is given as its shared Rust build cache: one directory
  # every sandboxed session downloads its crates and compiles its dependencies
  # into. systemd creates and owns it, and it survives a restart with everything
  # in it; the server would otherwise put it under the service's home, which is
  # not where a cache belongs on a packaged install.
  #
  # Named once because `CacheDirectory`, `BindPaths` and the flag all say it.
  cacheDir = "/var/cache/verkstead";

  # The directory half of a `sandboxBinds` entry: a plain path is the whole of
  # it, and `name=path` is the part after the `=` — the same rule the server
  # reads them by, so what the unit binds in is what the server hands out.
  bindPath = bind: if lib.hasPrefix "/" bind then bind else lib.last (lib.splitString "=" bind);

  # Whether the home is Verkstead's own to make. Under the state directory it
  # is: systemd creates it and hands it over. Anywhere else it is the human's,
  # and something that already exists.
  homeIsOurs = lib.hasPrefix "${stateDir}/" "${cfg.home}";
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

    home = lib.mkOption {
      type = lib.types.path;
      default = "${stateDir}/home";
      defaultText = lib.literalExpression ''"${stateDir}/home"'';
      description = ''
        The home directory the service runs with, as `HOME`, and what `~` means
        inside a session's sandbox.

        Nothing is read out of it and nothing of it is mounted into a sandbox.
        Credentials and identity are said rather than found: the GitHub token is
        `github_token` in `secrets.yaml` in the service's data directory, and who
        a commit is by is `git_author` in `config.yaml` beside it. Each reaches a
        session in its environment — `GH_TOKEN`, and git's own `GIT_CONFIG_*` —
        so there is nothing to provision here, and a credential left in this
        directory stays outside every sandbox.

        Said outright because systemd would otherwise derive it from the
        `verkstead` user's passwd entry, which is `/var/empty`: a home that is
        not writable is one a tool with a cache of its own trips over.

        The default is a directory under the state directory, which systemd
        creates and hands over. Pointing this at a human's own home works too, as
        long as the `verkstead` user can read it; it is bound in read-only, so it
        has to exist.
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
          "verkstead=/var/cache/verkstead-node"
        ]
      '';
      description = ''
        Extra read-write directories every sandboxed session gets, as
        `--sandbox-bind`. A plain path is given to every session; `name=path`
        is given only to sessions working in the Repo registered under that
        name, so a repository that needs a cache of its own can have one
        without every repository getting it.

        A Rust build cache is not one of them any more: the server gives every
        sandbox one of its own at `/var/cache/verkstead`, with sccache on this
        service's path to compile through, and the switch that turns it off is
        in the workbench settings. Nothing here has to be set for it.

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

    # The binary lands on `PATH`, for a human at a terminal: `verkstead serve
    # --help` is how they find out what this unit is passing it, and `verkstead
    # ask` is there for an agent working outside Verkstead. A session inside a
    # Sandbox asks with the running server's own image instead, bound in ahead of
    # this one, so the two halves of an ask are always the same build.
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

      # What the server itself has to be able to run. bwrap is every session's
      # sandbox, and it is here rather than in the package's own wrapper because
      # it is the server that spawns one — the CLI half of the same binary has
      # no use for it, and there are systems the package builds for that have no
      # bwrap to offer.
      #
      # `gh` is how Verkstead reaches GitHub itself: the pull request a finish
      # step opened, and what is on it. It runs as whoever the service's home is
      # logged in as — there is no token here and no GitHub App — so a unit whose
      # home has never run `gh auth login` will say so on the Timeline rather
      # than move a Conversation into Wrapping.
      #
      # `sccache` is what the shared Rust build cache compiles through. It is on
      # the service's path rather than named by a flag because that is where the
      # server looks for it: it resolves one at startup and binds it into every
      # sandbox read-only. Without it the cache is still a cache — the crate
      # downloads are shared — so the server says so in the log and carries on,
      # but on a module install it never has to.
      path = [
        pkgs.bubblewrap
        pkgs.gh
        pkgs.sccache
      ];

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
            "--data-dir"
            stateDir
            "--build-cache-dir"
            cacheDir
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

        # Said rather than left to systemd, which would take it from the
        # `verkstead` user's passwd entry and arrive at `/var/empty` — see the
        # `home` option for what a session reads out of it.
        Environment = [ "HOME=${cfg.home}" ];

        # systemd makes the directory and hands it over already owned; the
        # service never creates it, and it survives a restart with the database
        # in it. Relative paths the server is given resolve here too.
        #
        # The home joins it when it is under here, which is the default: a
        # directory systemd creates is one that is there on a fresh install
        # without anybody being told to make it.
        StateDirectory = [
          "verkstead"
        ]
        ++ lib.optional homeIsOurs (lib.removePrefix "/var/lib/" "${cfg.home}");
        StateDirectoryMode = "0750";

        # The shared build cache, made and owned the same way — and separately,
        # because it is the one directory here whose contents nobody would mind
        # losing: what is in it is rebuildable, and `systemctl clean --what=cache
        # verkstead` is how somebody reclaims the disk without touching the
        # database.
        CacheDirectory = "verkstead";
        CacheDirectoryMode = "0750";

        WorkingDirectory = stateDir;

        # An agent is blocked on an answer whenever the server is down, so come
        # back rather than sit in a failed state.
        Restart = "always";
        RestartSec = "5s";

        # Hardening. Three things it must not break: SQLite in WAL mode, which
        # creates `-wal` and `-shm` beside the database and so needs a
        # read-write directory rather than just a writable file; outbound
        # HTTPS — to the browser vendors' push services, whose addresses cannot
        # be enumerated ahead of time, and to GitHub for the update check, which
        # is why there is no `IPAddressAllow` here and why stopping the update
        # check is `updateCheck = false` rather than a firewall rule; and bwrap,
        # which every session runs inside and which needs namespaces, mounts and
        # a `/proc` of its own.
        #
        # Every relaxation below is one a sandbox started under this unit was
        # seen to need — the subtest in nix/vm-test.nix is that sandbox, and it
        # is what would catch one of them being quietly put back. Everything not
        # commented was tried alongside a working sandbox and left alone:
        # `PrivateUsers` and an empty `CapabilityBoundingSet` in particular look
        # like they would stop bwrap and do not, because a new user namespace
        # comes with a bounding set of its own.
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
        # `ProtectHostname` is gone: a sandbox gets a hostname of its own —
        # `bwrap --hostname verkstead` — and that setting makes `sethostname`
        # fail even inside a UTS namespace the process just created. What is
        # given up is a service that could rename the host, which it cannot do
        # anyway: it runs unprivileged with an empty capability bounding set,
        # and renaming the host needs CAP_SYS_ADMIN in the host's own namespace.
        #
        # `ProtectKernelLogs` and `ProtectKernelTunables` are gone for one
        # reason between them: each covers part of `/proc` — `/proc/kmsg` made
        # inaccessible, `/proc/sys` remounted read-only — and the kernel refuses
        # to mount a fresh procfs inside a user namespace when the procfs it
        # would be mounted over is only partly visible. bwrap mounts its own
        # `/proc`, which is what makes the sandbox's pid namespace mean
        # anything. What is given up is that the service can read the kernel
        # ring buffer; writing `/proc/sys` stays out of reach for the reason
        # above.
        ProtectKernelModules = true;
        ProtectProc = "invisible";
        # `ProcSubset = "pid"` is gone, and `ProtectProc` above is not: the
        # first hides everything in `/proc` that is not a process, and bwrap
        # reads `/proc/sys/kernel/overflowuid` before it can map a uid. The
        # second was tried alongside a working sandbox and stays.
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
        # Not `true`, which stops bwrap dead: an allow-list of exactly the
        # namespaces `bwrap --unshare-all --share-net` creates. `net` is not
        # among them and stays denied — the sandbox shares the host's network
        # rather than making one of its own, which is the design's own decision
        # and the one namespace whose absence a session would notice.
        RestrictNamespaces = [
          "cgroup"
          "ipc"
          "mnt"
          "pid"
          "user"
          "uts"
        ];
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        # `MemoryDenyWriteExecute` is gone, and this one is not about bwrap: a
        # sandbox starts perfectly well under it, and then everything inside
        # inherits the seccomp filter. What a session runs is `claude`, which is
        # node, and V8 aborts the moment it cannot make a page it wrote
        # executable. A JIT is what a coding agent is made of, so this is a
        # setting the product cannot have rather than one it has not got round
        # to.
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          # bwrap builds the sandbox's filesystem, and none of `mount`,
          # `umount2` or `pivot_root` is in `@system-service`.
          "@mount"
          "~@privileged"
          "~@resources"
          # The three calls out of `@privileged` a sandbox needs, named one at a
          # time rather than by letting the group back in: `capset` is bwrap
          # dropping capabilities, `pivot_root` is it swapping the root, and
          # `sethostname` is `--hostname`. Order matters — a name after a `~`
          # group puts that one call back.
          "capset"
          "pivot_root"
          "sethostname"
        ];
        UMask = "0077";

        # `ProtectSystem = "strict"` leaves the state directory writable and
        # nothing else, which is the whole of what the server writes: the Data
        # Directory is that directory, so there is nothing else to permit.

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
        #
        # The build cache is first, and it is here by the same rule rather than
        # because something is currently hiding it: `CacheDirectory` above
        # already exempts it from `ProtectSystem = "strict"`. This list is what
        # says which directories a *session* has to be able to reach, and a
        # cache missing from it is one a later tightening could take away
        # silently — from every Rust build in every sandbox, at once.
        BindPaths = [
          cacheDir
        ]
        ++ map (path: "${path}") cfg.watchedPaths
        ++ map bindPath cfg.sandboxBinds;

        # A home the human named somewhere of their own, bound in for the reason
        # a Watched Path is: it is usually under `/home`, which `ProtectHome`
        # replaces with an empty tmpfs, and a HOME that is not there is what a
        # tool reaching for one fails obscurely on. Read-only, because nothing is
        # read out of it any more and a service writing into somebody's own home
        # is not what naming one here asks for. Under the state directory there
        # is nothing to bind — systemd made it, and it is the service's own.
        BindReadOnlyPaths = lib.optional (!homeIsOurs) "${cfg.home}";
      };
    };
  };
}
