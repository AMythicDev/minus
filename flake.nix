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

        rust-toolchain = pkgs.rust-bin.stable."1.67.0".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

      in
      with pkgs;
      {
        devShells = {
          default = mkShell {
            packages = [ just ] ++ [ rust-toolchain ];
          };
        };
      }
    );
}
