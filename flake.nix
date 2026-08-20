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
        # Until the first Release there is nothing to download: the manifest
        # ships with `systems` empty, and a package whose `src` cannot be named
        # is one `nix flake check` refuses to evaluate — so `verkstead` *is* the
        # source build for exactly as long as that is true. `release.yml`
        # writing a real manifest is what switches it over, which means nothing
        # here has to be remembered and undone.
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
              # The PWA icons are one SVG rasterized to the PNG sizes the manifest
              # and iOS need — see tools/generate-icons.sh.
              resvg
            ]);

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}
