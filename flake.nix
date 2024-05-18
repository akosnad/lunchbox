{
  description = "lunchbox dev environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    esp-dev.url = "github:mirrexagon/nixpkgs-esp-dev";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, esp-dev, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [
          (import rust-overlay)
        ];
        pkgs = import nixpkgs
          {
            inherit system overlays;
          }
        //
        esp-dev.packages.${system};

        esp-rust = pkgs.callPackage ./nix/esp-rust.nix { };
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            openssl
            bun
            esp-idf-esp32
            esp-rust
            rustup
            espflash
          ];

          shellHook = ''
            export RUST_TOOLCHAIN="${esp-rust}"
          '';
        };
      });
}
