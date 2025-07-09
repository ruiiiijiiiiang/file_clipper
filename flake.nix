{
  description = "File Clipper Nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }:
    let
      system = pkgs.system;
      pkgs = import nixpkgs {
        inherit system;
      };
    in {
      packages.${system}.default = pkgs.stdenv.mkDerivation {
        pname = "file_clipper";
        version = "0.1.1";
        src = self;

        nativeBuildInputs = with pkgs; [
          pkgs.rustPlatform.buildRustPackage
        ];
        cargoBuildFlags = "--release";
        cargoInstallFlags = "--root $out --path .";

        meta = with pkgs.lib; {
          description = "Command Line File Clipboard";
          homepage = "https://github.com/ruiiiijiiiiang/file_clipper";
          license = licenses.mit;
          platforms = platforms.linux;
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
        ] ++ self.packages.${system}.default.buildInputs;
      };
    };
}
