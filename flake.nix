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
        build-bot = (pkgs: pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
          buildInputs = [ pkgs.openssl ];
          nativeBuildInputs = [
            pkgs.pkg-config
            # pkgs.gcc
          ];
          doCheck = false;
          # RUSTFLAGS = "-C target-feature=+crt-static";
        });
      in
      rec
      {
        packages = {
          bot = build-bot pkgs;
          bot-cross-aarch64-linux = build-bot pkgs.pkgsCross.aarch64-multiplatform;
          docker = pkgs.dockerTools.buildLayeredImage {
            name = "registry.digitalocean.com/johnnybot/bot";
            tag = if (self ? rev) then self.shortRev else "dirty";
            config.Cmd = [ "${packages.bot}/bin/bot" ];
            contents = [ packages.bot ];
          };
        };

        defaultPackage = packages.bot;


        devShell = with pkgs; mkShell {
          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath [ pkgs.openssl ];
          buildInputs = [
            doctl
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
