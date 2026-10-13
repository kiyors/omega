{
  description = "A high-performance terminal session and workspace navigator";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          }
        );
    in
    {
      overlays.default = final: prev: {
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine (
            with (toolchainOf {
              channel = "1.99.0";
              sha256 = "sha256-zm3dyIY2T414ZRR3EhLOvptzG6gta4WZUcawzMUWtqI=";
            });
            [
              clippy
              rustc
              cargo
              rustfmt
              rust-src
            ]
          );
      };

      # Delegates derivation (buildRustPackage with pname = "omega") to ./nix/package.nix
      packages = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.callPackage ./nix/package.nix { };
          omega = pkgs.callPackage ./nix/package.nix { };
        }
      );

      # Delegates devShell to ./nix/devshell.nix
      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = import ./nix/devshell.nix {
            inherit pkgs;
            rustToolchain = pkgs.rustToolchain;
          };
        }
      );
    };
}
