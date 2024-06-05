{
  description = "A gtk-based polkit authentication agent.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    supportedSystems = ["x86_64-linux"];
    eachSystem = f:
      nixpkgs.lib.genAttrs supportedSystems
      (system: f nixpkgs.legacyPackages.${system});
  in {
    packages = eachSystem (pkgs: import ./. {inherit pkgs;});
    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell (let
        soteria = self.packages.${pkgs.system}.default;
      in {
        inputsFrom = [soteria];
        packages = builtins.attrValues {
          inherit (pkgs) rust-analyzer rustfmt;
        };
      });
    });
    formatter = eachSystem (pkgs: pkgs.alejandra);
  };
}
