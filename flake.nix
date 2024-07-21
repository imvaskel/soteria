{
  description = "A gtk-based polkit authentication agent.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      lib = pkgs.lib;
      inherit
        (lib.fileset)
        toSource
        gitTracked
        difference
        unions
        fileFilter
        ;

      craneLib = crane.mkLib pkgs;

      # Common arguments can be set here to avoid repeating them later
      # Note: changes here will rebuild all dependency crates
      commonArgs = {
        src = lib.fileset.toSource {
          root = ./.;
          fileset =
            difference (gitTracked ./.)
            (unions [
              ./README.md
              (fileFilter (file: file.hasExt "nix") ./.)
            ]);
        };
        strictDeps = true;

        preBuild = ''
          export POLKIT_AGENT_HELPER_PATH="$(strings ${pkgs.polkit.out}/lib/libpolkit-agent-1.so | grep "polkit-agent-helper-1")"
        '';

        buildInputs =
          [
            # Add additional build inputs here
            pkgs.gtk4
            pkgs.glib
            pkgs.polkit
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.binutils
        ];
      };

      soteria = craneLib.buildPackage (commonArgs
        // {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          meta.mainProgram = "soteria";

          # Additional environment variables or build phases/hooks can be set
          # here *without* rebuilding all dependency crates
          # MY_CUSTOM_VAR = "some value";
        });
    in {
      checks = {
        inherit soteria;
      };

      packages.default = soteria;

      apps.default = flake-utils.lib.mkApp {
        drv = soteria;
      };

      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [
          # pkgs.ripgrep
        ];
      };
    });
}
