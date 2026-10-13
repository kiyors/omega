use std::fs;
use std::path::Path;

#[test]
fn test_homebrew_formula_structure_and_validity() {
    let formula_path = Path::new("Formula/omega.rb");
    assert!(formula_path.exists(), "Formula/omega.rb must exist");

    let content = fs::read_to_string(formula_path).expect("failed to read Formula/omega.rb");
    assert!(
        content.contains("class Omega < Formula"),
        "formula must declare class Omega < Formula"
    );
    assert!(
        content.contains("https://github.com/kiyors/omega"),
        "formula must point to omega repository"
    );
    assert!(content.contains("cargo"), "formula must build using cargo");
    assert!(
        content.contains("omega doctor"),
        "formula test block should verify omega doctor"
    );
}

#[test]
fn test_nix_flake_declares_package_build() {
    let flake_path = Path::new("flake.nix");
    assert!(flake_path.exists(), "flake.nix must exist");

    let content = fs::read_to_string(flake_path).expect("failed to read flake.nix");
    assert!(
        content.contains("buildRustPackage"),
        "flake.nix should declare a buildRustPackage derivation for omega"
    );
    assert!(
        content.contains("packages ="),
        "flake.nix should expose package outputs"
    );
    assert!(
        content.contains("pname = \"omega\""),
        "flake.nix package should be named omega"
    );
}

#[test]
fn test_rust_toolchain_and_modular_nix_structure() {
    let toolchain_path = Path::new("rust-toolchain.toml");
    assert!(toolchain_path.exists(), "rust-toolchain.toml must exist");
    let toolchain_content = fs::read_to_string(toolchain_path).unwrap();
    assert!(
        toolchain_content.contains("channel = \"1.99.0\""),
        "rust toolchain channel must be locked to 1.99.0"
    );

    let pkg_path = Path::new("nix/package.nix");
    assert!(pkg_path.exists(), "nix/package.nix must exist");
    let pkg_content = fs::read_to_string(pkg_path).unwrap();
    assert!(pkg_content.contains("buildRustPackage"));
    assert!(pkg_content.contains("pname = \"omega\""));

    let devshell_path = Path::new("nix/devshell.nix");
    assert!(devshell_path.exists(), "nix/devshell.nix must exist");
}
