{
  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    cargo2nix.url = "github:cargo2nix/cargo2nix/unstable";
  };

  outputs = { self, nixpkgs, utils, rust-overlay, cargo2nix }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) cargo2nix.overlays.default ];
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
        rustPkgs = pkgs.rustBuilder.makePackageSet
          {
            rustVersion = "1.75.0";
            packageFun = import ./Cargo.nix;
          };
      in
      rec
      {
        packages = {
          bot = build-bot pkgs;
          bot2 = (rustPkgs.workspace.bot { });
          bot-cross-aarch64-linux = build-bot pkgs.pkgsCross.aarch64-multiplatform;
          docker = pkgs.dockerTools.buildLayeredImage {
            name = "registry.digitalocean.com/johnnybot/bot";
            tag = if (self ? rev) then self.shortRev else "dirty";
            config.Cmd = [ "${packages.bot2}/bin/bot" ];
            contents = [ packages.bot2 ];
          };
        };

        defaultPackage = packages.bot2;


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
            sqlx-cli
          ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          shellHook = ''
            export OPENSSL_DIR="${openssl.dev}"
            export OPENSSL_LIB_DIR="${openssl.out}/lib"
          '';
        };
      });
}
