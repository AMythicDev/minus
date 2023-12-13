{
  inputs = {
    # Specify the source of Home Manager and Nixpkgs.
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rust-nightly-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain-nightly.toml;

        shared_packages = with pkgs; [
          just
        ];
      in
      with pkgs;
      {
        devShells = rec {
          default = msrv;

          nightly = mkShell {
            packages = shared_packages ++ [ rust-nightly-toolchain ];
          };

          msrv = mkShell {
            packages = shared_packages ++ [ rust-toolchain ];
          };
        };
      }
    );
}
