{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      in
      {
        devShells.default = pkgs.callPackage (
          {
            mkShellNoCC,
            stdenv,
            rust-bin,
            wasmtime,
          }:
          mkShellNoCC {
            nativeBuildInputs = [
              (rust-bin.stable.latest.default.override {
                targets = [ "wasm32-unknown-unknown" ];
              })
            ];

            # Set linker flags for WASM
            CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS = "-C target-feature=+bulk-memory";
          }
        ) { };
      }
    );
}
