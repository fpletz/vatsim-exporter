{
  description = "VATSIM Prometheus Exporter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-fast-build = {
      url = "github:Mic92/nix-fast-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    crane,
    nix-fast-build,
    ...
  }:
    utils.lib.eachSystem ["x86_64-linux" "aarch64-linux"] (system: let
      pkgs = import nixpkgs {inherit system;};
      craneLib = crane.lib.${system};

      src = craneLib.cleanCargoSource ./.;
      cargoArtifacts = craneLib.buildDepsOnly {
        inherit src;
        pname = "vatsim-exporter";
      };
      vatsim-exporter = craneLib.buildPackage {
        inherit src cargoArtifacts;
        meta.mainProgram = "vatsim-exporter";
      };
    in {
      packages = {
        default = vatsim-exporter;

        dockerImage = pkgs.dockerTools.buildImage {
          name = "vatsim-exporter";
          tag = "latest";
          copyToRoot = [vatsim-exporter];

          config = {
            Cmd = ["/bin/vatsim-exporter"];
            ExposedPorts."9185/tcp" = {};
          };
        };
      };

      apps = {
        ci-check = utils.lib.mkApp {
          drv = pkgs.writers.writeBashBin "ci-check" ''
            ${pkgs.lib.getExe nix-fast-build.packages.${system}.default} --no-nom --skip-cached
          '';
        };
      };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          cargo
          cargo-watch
          rust-analyzer
          rustPackages.clippy
          rustc
          rustfmt
          nix-fast-build.packages.${system}.default
        ];
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
      };

      formatter = pkgs.alejandra;

      nixosTests.default = pkgs.nixosTest {
        name = "vatsim-exporter-test";
        nodes.machine = {...}: {
          nixpkgs.system = system;
          imports = [self.nixosModules.default];
          services.vatsim-exporter.enable = true;
        };
        testScript = ''
          machine.wait_for_unit("default.target")
        '';
      };

      checks = {
        package = self.packages.${system}.default;
        devShell = self.devShells.${system}.default;

        # only checks evaluation
        nixosTest = self.nixosTests.${system}.default.driver;
      };
    })
    // {
      nixosModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.services.vatsim-exporter;
      in {
        options = {
          services.vatsim-exporter = {
            enable = lib.mkEnableOption "VATSIM Prometheus Exporter";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${config.nixpkgs.system}.default;
            };
          };
        };

        config = lib.mkIf cfg.enable {
          systemd.services.vatsim-exporter = {
            wantedBy = ["multi-user.target"];
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
}
