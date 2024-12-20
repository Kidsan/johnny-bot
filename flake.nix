{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, rust-overlay }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.stable.latest.default;
          rustc = pkgs.rust-bin.stable.latest.default;
        };
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
      in
      rec
      {
        packages = {
          bot = rustPlatform.buildRustPackage {
            pname = manifest.name;
            version = if (self ? rev) then self.shortRev else "dirty";
            cargoLock.lockFile = ./Cargo.lock;
            src = pkgs.lib.cleanSource ./.;
            buildInputs = [
              pkgs.openssl
              pkgs.libopus
            ];
            nativeBuildInputs = [
              pkgs.pkg-config
            ];
            doCheck = false;
          };
          docker = pkgs.dockerTools.buildLayeredImage {
            name = "registry.digitalocean.com/johnnybot/bot";
            tag = if (self ? rev) then self.shortRev else "dirty";
            config.Cmd = [ "${packages.bot}/bin/bot" ];
            contents = [ packages.bot ];
          };
        };

        defaultPackage = packages.bot;


        devShell = with pkgs; mkShell {
          # LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath [ pkgs.openssl ];
          buildInputs = [
            python3
            doctl
            pkg-config
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
            })
            bashInteractive
            cargo-watch
            sqlx-cli
            opentofu
            libopus
          ];
          # RUST_SRC_PATH = rustPlatform.rustLibSrc;
          shellHook = ''
            export OPENSSL_DIR="${openssl.dev}"
            export OPENSSL_LIB_DIR="${openssl.out}/lib"
          '';
        };
      });
}
