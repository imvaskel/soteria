{pkgs ? import <nixpkgs> {}}: rec {
  soteria = pkgs.callPackage ./package.nix {};
  default = soteria;
}
