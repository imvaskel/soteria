{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    supportedSystems = ["x86_64-linux"];
    eachSystem = nixpkgs.lib.genAttrs supportedSystems;
  in {
    packages = eachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      soteria = pkgs.callPackage ./default.nix {};
      default = self.packages.${system}.soteria;
    });

    devShell = eachSystem (system: let pkgs = nixpkgs.legacyPackages.${system}; in pkgs.mkShell {
      packages = with pkgs; [
        gtk4
        glib
        polkit
        pkg-config
        cargo
      ];
    });

    formatter = eachSystem (system: nixpkgs.legacyPackages.${system}.alejandra);
  };
}
