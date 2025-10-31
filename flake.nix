{
  description = "File Clipper Nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        fileClipperPkg = pkgs.rustPlatform.buildRustPackage {
          pname = "file_clipper";
          version = "0.1.3";

          src = ./.;
          binaries = [ "clp" ];

          cargoHash = "sha256-MlLT1FEXCGi5+FST8Xlzu+EmeTWcsEUI2t2O2kd1owA=";
        };
      in
      {
        devShell = pkgs.mkShell {
          packages = [
            pkgs.rust-bin.stable.latest.default
          ];
        };

        packages.default = fileClipperPkg;

        apps.default = {
          type = "app";
          program = "${fileClipperPkg}/bin/clp";
        };
      }
    );
}
