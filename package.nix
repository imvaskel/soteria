{
  rustPlatform,
  gtk4,
  glib,
  polkit,
  lib,
  pkg-config,
  binutils,
}: let
  inherit
    (lib.fileset)
    toSource
    gitTracked
    difference
    unions
    fileFilter
    ;
in
  rustPlatform.buildRustPackage {
    pname = "soteria";
    version = "0.1.0";

    nativeBuildInputs = [
      pkg-config
      binutils
    ];

    buildInputs = [
      gtk4
      glib
      polkit
    ];

    # Takes advantage of nixpkgs manually editing PACKAGE_PREFIX by grabbing it from
    # the binary itself.
    # https://github.com/NixOS/nixpkgs/blob/9b5328b7f761a7bbdc0e332ac4cf076a3eedb89b/pkgs/development/libraries/polkit/default.nix#L142
    # https://github.com/polkit-org/polkit/blob/d89c3604e2a86f4904566896c89e1e6b037a6f50/src/polkitagent/polkitagentsession.c#L599
    preBuild = ''
      export POLKIT_AGENT_HELPER_PATH="$(strings ${polkit.out}/lib/libpolkit-agent-1.so | grep "polkit-agent-helper-1")"
    '';

    src = toSource {
      root = ./.;
      fileset =
        difference (gitTracked ./.)
        (unions [
          ./README.md
          (fileFilter (file: file.hasExt "nix") ./.)
        ]);
    };

    cargoLock.lockFile = ./Cargo.lock;
    meta = {
      mainProgram = "soteria";
      description = "A Polkit authentication agent written in GTK designed to be used with any desktop environment";
      homepage = "https://github.com/ImVaskel/soteria";
      platforms = lib.platforms.linux;
      license = lib.licenses.asl20;
    };
  }
