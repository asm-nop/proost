{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "proost";
          version = "0.1.0";

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "A simple proof assistant written in Rust";
            homepage = "https://gitlab.crans.org/loutr/proost";
            license = licenses.gpl3;
          };
        };
      });
}

