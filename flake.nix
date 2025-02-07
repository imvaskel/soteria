{
  description = "A gtk-based polkit authentication agent.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-compat.url = "github:edolstra/flake-compat";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    ...
  }: let
    inherit (nixpkgs) lib;
    systems = ["x86_64-linux" "aarch64-linux"];
    forEachSystem = lib.genAttrs systems;
  in {
    packages = forEachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = crane.mkLib pkgs;
    in {
      soteria = pkgs.callPackage ./package.nix {inherit craneLib;};
      default = self.packages.${system}.soteria;
    });

    devShells = forEachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = crane.mkLib pkgs;
      soteria = self.packages.${system}.default;
      buildDeps = soteria.nativeBuildInputs ++ soteria.buildInputs;
    in {
      default = craneLib.devShell {
        packages =
          [
          ]
          ++ buildDeps;
      };
    });
  };
}
