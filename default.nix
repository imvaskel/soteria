{
  rustPlatform,
  gtk4,
  glib,
  polkit,
  pkg-config,
}: let
  polkitAgentPath = "${polkit}/lib/polkit-1/polkit-agent-helper-1";
  pname = "soteria";
  version = "0.1.0";
in
  rustPlatform.buildRustPackage rec {
    inherit pname version;
    POLKIT_AGENT_HELPER_PATH = polkitAgentPath;

    nativeBuildInputs = [
      pkg-config
    ];

    buildInputs = [
      gtk4
      glib
      polkit
    ];
    inherit glib;

    src = ./.;

    cargoLock = {
      lockFile = ./Cargo.lock;
    };
  }
