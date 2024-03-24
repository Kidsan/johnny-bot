{
  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, rust-overlay }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
      in
      {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
          buildInputs = [ pkgs.openssl ];
          nativeBuildInputs = [ pkgs.pkg-config ];
        };

        devShell = with pkgs; mkShell {
          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath [ pkgs.openssl ];
          buildInputs = [
            pkg-config
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
            })
            bashInteractive
            cargo-watch
          ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          shellHook = ''
            export OPENSSL_DIR="${openssl.dev}"
            export OPENSSL_LIB_DIR="${openssl.out}/lib"
          '';
        };
      });
}