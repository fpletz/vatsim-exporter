{
  description = "VATSIM Prometheus Exporter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    nix-fast-build = {
      url = "github:Mic92/nix-fast-build";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
      };
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.pre-commit-hooks.flakeModule
      ];

      perSystem =
        {
          self',
          inputs',
          pkgs,
          system,
          config,
          ...
        }:
        let
          craneLib = inputs.crane.mkLib inputs.nixpkgs.legacyPackages.${system};

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          commonArgs = {
            inherit src;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
          };
          cargoArtifacts = craneLib.buildDepsOnly (commonArgs // { pname = "vatsim-exporter"; });
          vatsim-exporter = craneLib.buildPackage (
            commonArgs
            // {
              inherit src cargoArtifacts;
              meta.mainProgram = "vatsim-exporter";
            }
          );
        in
        {
          packages = {
            default = vatsim-exporter;

            dockerImage = pkgs.dockerTools.buildImage {
              name = "vatsim-exporter";
              tag = "latest";
              copyToRoot = [ vatsim-exporter ];

              config = {
                Cmd = [ "/bin/vatsim-exporter" ];
                ExposedPorts."9185/tcp" = { };
              };
            };
          };

          apps = {
            ci-check.program = pkgs.writers.writeBashBin "ci-check" ''
              ${pkgs.lib.getExe inputs'.nix-fast-build.packages.default} --no-nom --skip-cached
            '';
          };

          checks = {
            package = self'.packages.default;
            devShell = self'.devShells.default;

            clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            cargo-fmt = craneLib.cargoFmt { inherit src; };

            nixosTest = pkgs.nixosTest {
              name = "vatsim-exporter-test";
              nodes.machine =
                { ... }:
                {
                  nixpkgs.system = system;
                  imports = [ inputs.self.nixosModules.default ];
                  services.vatsim-exporter.enable = true;
                };
              testScript = ''
                machine.wait_for_unit("default.target")
                machine.wait_for_open_port(9185)
              '';
            };
          };

          formatter = pkgs.nixfmt-rfc-style;

          pre-commit = {
            check.enable = true;
            settings.hooks.treefmt = {
              enable = true;
              package = config.treefmt.build.wrapper;
            };
          };
          treefmt = {
            projectRootFile = "flake.lock";

            settings.formatter = {
              nix = {
                command = pkgs.nixfmt-rfc-style;
                includes = [ "*.nix" ];
              };
              rustfmt = {
                command = pkgs.rustfmt;
                options = [
                  "--edition"
                  "2021"
                ];
                includes = [ "*.rs" ];
              };
            };
          };

          devShells.default = craneLib.devShell {
            packages = [
              pkgs.cargo-watch
              pkgs.rust-analyzer
              inputs.nix-fast-build.packages.${system}.default
              config.treefmt.build.wrapper
              pkgs.pkg-config
              pkgs.openssl
            ];
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            shellHook = ''
              ${config.pre-commit.installationScript}
            '';
          };
        };

      flake = {
        nixosModules.default =
          { config, lib, ... }:
          let
            cfg = config.services.vatsim-exporter;
          in
          {
            options = {
              services.vatsim-exporter = {
                enable = lib.mkEnableOption "VATSIM Prometheus Exporter";
                package = lib.mkOption {
                  type = lib.types.package;
                  default = inputs.self.packages.${config.nixpkgs.system}.default;
                };
              };
            };

            config = lib.mkIf cfg.enable {
              systemd.services.vatsim-exporter = {
                wantedBy = [ "multi-user.target" ];
                serviceConfig = {
                  ExecStart = "${lib.getExe cfg.package}";
                  DynamicUser = true;
                  Restart = "always";
                  RestartSec = "2s";
                };
              };
            };
          };
      };
    };
}
