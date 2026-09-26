{
  description = "Verkstead — a service and CLI through which coding agents put questions to a human";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # What the viewer under `web/` is built and tested with. Named here because
      # the dev shell and the build both take it, and a pnpm in one that is not
      # the pnpm in the other is a lockfile argument waiting to happen.
      webTools =
        pkgs: with pkgs; [
          nodejs
          pnpm
        ];
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = verkstead;
        # The released binary, downloaded — see nix/verkstead.nix for why that is
        # what `nix run github:tobico/verkstead` should get. The build from this
        # tree is one attribute away, under its own name.
        #
        # The manifest names the last Release, one entry per system it built
        # for, and `release.yml`'s last job is what writes it — so what this
        # resolves to moves with the Releases and nothing here is edited by
        # hand.
        #
        # **A system the manifest has no entry for falls back to the source
        # build**, which is what the condition is for: a package whose `src`
        # cannot be named is one `nix flake check` refuses to evaluate, so an
        # entry that is not there has to be something rather than an error. It
        # covered every system before the first Release, when `systems` shipped
        # empty, and it covers a platform a later Release did not build for.
        verkstead =
          if (nixpkgs.lib.importJSON ./nix/release.json).systems ? ${pkgs.stdenv.hostPlatform.system} then
            pkgs.callPackage ./nix/verkstead.nix { }
          else
            verkstead-source;
        verkstead-source = pkgs.callPackage ./nix/verkstead-source.nix { inherit viewer; };
        # The viewer's static files on their own. Nothing serves them from here —
        # `verkstead` embeds them — but they are worth building alone when what is
        # being looked at is the vite output.
        viewer = pkgs.callPackage ./nix/web.nix { };
      });

      # The module runs the package above, so it closes over this flake rather
      # than looking for `pkgs.verkstead`, which is nowhere to be found.
      nixosModules = rec {
        default = verkstead;
        verkstead = import ./nix/module.nix self;
      };

      # `nix flake check` builds whatever is in here. The viewer's suite runs
      # anywhere node does; the VM test is offered only where a NixOS VM can be
      # booted at all, because it needs a Linux host to run the guest kernel on,
      # and on Darwin that check is simply absent rather than a failure.
      checks = forAllSystems (
        pkgs:
        {
          web = pkgs.callPackage ./nix/web.nix { runTests = true; };
        }
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          module = pkgs.callPackage ./nix/vm-test.nix {
            module = self.nixosModules.verkstead;
            package = self.packages.${pkgs.stdenv.hostPlatform.system}.verkstead-source;
          };
          # And the one option of the module's whose whole effect is a line in
          # passwd — the shell a Conversation's Terminal comes up in — which is
          # an evaluation's question rather than a boot's.
          module-shell = pkgs.callPackage ./nix/module-shell.nix {
            module = self.nixosModules.verkstead;
            nixosSystem = nixpkgs.lib.nixosSystem;
            system = pkgs.stdenv.hostPlatform.system;
          };
          # And the cluster's three options, whose other half is the same kind
          # of question: a machine is booted with its firewall open or shut and
          # never both, so what an `openFirewall = false` host is left with — or
          # one that says nothing about itself on the LAN — is asked of the
          # evaluation rather than of a second VM.
          module-peer = pkgs.callPackage ./nix/module-peer.nix {
            module = self.nixosModules.verkstead;
            nixosSystem = nixpkgs.lib.nixosSystem;
            system = pkgs.stdenv.hostPlatform.system;
          };
        }
      );

      # `nix run` is the server, UI and all; the CLI is the same binary without
      # the `serve` verb and has to be asked for by name.
      apps = forAllSystems (
        pkgs:
        let
          verkstead = self.packages.${pkgs.stdenv.hostPlatform.system}.verkstead;
        in
        {
          default = {
            type = "app";
            # An app is a program and no arguments, and the server is a verb of
            # the one binary now — so what `nix run` runs is a script that
            # supplies the verb and passes the caller's own flags on through.
            program = "${pkgs.writeShellScript "verkstead-serve" ''
              exec ${verkstead}/bin/verkstead serve "$@"
            ''}";
            meta.description = "The Verkstead server, agent API and UI both";
          };
          verkstead = {
            type = "app";
            program = "${verkstead}/bin/verkstead";
            meta.description = "The Verkstead CLI, through which an agent asks";
          };
        }
      );

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages =
            (webTools pkgs)
            ++ (with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer
              sqlite
              # The CLI derives `project`, `branch` and the Diff by shelling out
              # to git, so git is a runtime dependency and not just a habit.
              git
              # What a session runs inside. Verkstead is Linux-and-bwrap only by
              # design, and the sandbox's own tests prove the surface by running
              # a probe in one rather than by reading the flags.
              bubblewrap
              # What the shared Rust build cache compiles through. The server
              # resolves one off its own `PATH` at startup and binds it into
              # every sandbox, so a checkout run gets the whole feature rather
              # than the half of it that only shares the downloads — the
              # packaged unit puts it on the service's path for the same reason.
              sccache
              # The probe's one tool: it proves the sandbox is on the host's
              # network by reaching a listener the test itself is holding open,
              # which is the sharing proved without touching the internet.
              curl
              # What a ticked `gh` row unpacks on an Intel Mac, which has no
              # Homebrew to install one with — see
              # `crates/server/src/onboarding/install.rs`'s `GH_RELEASE`. Both
              # halves: `unzip` is what that line runs, and `zip` is what
              # `crates/server/tests/installing.rs` builds the release its stub
              # `curl` answers with. Every Mac carries `unzip` already, and the
              # runner images carry both.
              zip
              unzip
              # The PWA icons are one PNG downscaled to the sizes the favicon,
              # the manifest and iOS need — see tools/generate-icons.sh. The
              # same tool downscales the same artwork into the sizes a desktop's
              # launcher draws — see tools/generate-packaging.sh.
              imagemagick
              # `desktop-file-validate`, which that script runs over the entry it
              # writes: an entry a desktop will not parse is one that never
              # appears in a menu, and nothing else here would notice.
              desktop-file-utils
              # `mksquashfs`, which is an AppImage's filesystem and so most of
              # what tools/build-appimage.sh needs to make one — see there for
              # why the format's own tool is not what makes it.
              squashfsTools
              # What the desktop crate's C dependencies below are found with.
              # Nothing else here needs one: `crates/desktop` is the first thing
              # in this repository to link a system library at all.
              pkg-config
              # `dbus-run-session`, which the tray's suite is run under: the
              # icon is published onto the session bus, and what that suite puts
              # on the bus is the panel's own name — so it wants a session of
              # its own rather than the desktop's. Nothing links this and the
              # AppImage carries none of it; a developer running
              # `crates/desktop/tests/tray.rs` is the whole of what it is for.
              dbus
            ]);

          # The desktop app's toolkit (ADR-0012). A build input rather than a
          # package so that pkg-config is pointed at its development files: the
          # app's dialogs compile against GTK3 headers, and a shell without them
          # cannot build `crates/desktop` at all. It is what the AppImage
          # carries, and it is now the only such library — the tray used to be
          # drawn over an appindicator beside it, and is spoken onto the session
          # bus in Rust instead. See `crates/desktop/Cargo.toml` for the swap,
          # and note that nothing is needed at *run* time either: there is no
          # longer a library opened by name with no rpath to find it by.
          buildInputs = with pkgs; [
            gtk3
          ];

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}
