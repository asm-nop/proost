{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-23.11";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, crane, fenix, flake-utils, nixpkgs, }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ fenix.overlays.default ];
        pkgs = import nixpkgs { inherit overlays system; };

        toolchain = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-6lRcCTSUmWOh0GheLMTZkY7JC273pWLp2s98Bb2REJQ=";
        };

        rust = (crane.mkLib pkgs).overrideToolchain toolchain;

        proost = {
          args = {
            src = ./.;
            pname = "proost";
            version = "0.3.0";

            env = {
              CARGO_PROFILE = "test";
            };
          };

          cargoArtifacts = rust.buildDepsOnly proost.args;
        };
      in rec {
        packages = {
          default = packages.proost;

          doc = rust.cargoDoc (proost.args // {
            cargoDenyChecks = "all";
            cargoDocExtraArgs = "--document-private-items --no-deps";
          });

          proost = rust.buildPackage (proost.args // {
            cargoArtifacts = proost.cargoArtifacts;
            meta = with pkgs.lib; {
              description = "A simple proof assistant written in Rust";
              homepage = "https://gitlab.crans.org/loutr/proost";
              license = licenses.gpl3;
            };
          });
        };

        checks = {
          proost = packages.proost;

          clippy = rust.cargoClippy (proost.args // {
            cargoArtifacts = proost.cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets --all-features --no-deps";
          });

          deny = rust.cargoDeny (proost.args // {
            cargoDenyChecks = "all";
          });

          fmt = rust.cargoFmt proost.args;
        } // pkgs.lib.optionalAttrs (pkgs.stdenv.isLinux) {
          coverage = let
            llvmCovArgs = "--ignore-filename-regex nix/store --locked --frozen --offline --profile test";
          in rust.buildPackage (proost.args // {
            pnameSuffix = "-coverage";
            cargoArtifacts = proost.cargoArtifacts;

            nativeBuildInputs = [ pkgs.cargo-llvm-cov ];

            buildPhaseCargoCommand = ''
              cargo llvm-cov test --workspace --no-report ${llvmCovArgs}
              cargo llvm-cov report --html --output-dir $out ${llvmCovArgs}
              cargo llvm-cov report --cobertura --output-path $out/cobertura.xml ${llvmCovArgs}
              cargo llvm-cov report --json --output-path $out/coverage.json ${llvmCovArgs}
            '';
            installPhaseCommand = "true";

            doCheck = false;
          });
        };

        devShells.default = rust.devShell { checks = self.checks.${system}; };

        formatter = pkgs.nixfmt;
      });
}
