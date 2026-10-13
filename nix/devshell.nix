{
  pkgs,
  rustToolchain,
}:

pkgs.mkShell {
  packages =
    with pkgs;
    [
      rustToolchain
      openssl
      pkg-config
      rust-analyzer
      taplo
    ]
    ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
      pkgs.libiconv
    ];

  env = {
    RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
  };

  shellHook = ''
    unset SDKROOT
    unset DEVELOPER_DIR
    export PATH="/usr/bin:/usr/sbin:/usr/local/bin:$PATH"
  '';
}
