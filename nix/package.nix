{
  lib,
  rustPlatform,
  stdenv,
  pkg-config,
  libiconv,
}:

rustPlatform.buildRustPackage {
  pname = "omega";
  version = "0.1.0";
  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = lib.optionals stdenv.hostPlatform.isDarwin [
    libiconv
  ];

  meta = with lib; {
    description = "A high-performance terminal session and workspace navigator";
    homepage = "https://github.com/kiyors/omega";
    license = licenses.mit;
    mainProgram = "omega";
  };
}
