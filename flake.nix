{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay }@inputs:
  let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
        ];
      };

      rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
      toolchain = rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      };
    in f system pkgs toolchain);
  in {
    devShell = forAllSystems (system: pkgs: toolchain: pkgs.mkShell {
      packages = with pkgs; [
        toolchain
        mpv
        grcov
        cargo-nextest
        cargo-edit
      ];

      env.RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
    });
  };
}
