{
  description = "bongo-modulator dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";

  outputs = { self, nixpkgs, rust-overlay, cargo2nix }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
          cargo2nix.overlays.default
        ];
      };
      rustToolchain = pkgs.rust-bin.stable.latest.default;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
      rustPkgs = pkgs.rustBuilder.makePackageSet {
        rustToolchain = rustToolchain;
        packageFun = import ./Cargo.nix;
        packageOverrides = pkgs: pkgs.rustBuilder.overrides.all;
      };
    in {
      packages.${system}.default = rustPkgs.workspace.bongo-modulator {
        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.protobuf
          pkgs.llvmPackages.libclang
          pkgs.linuxHeaders
        ];
        buildInputs = [ pkgs.llvmPackages.libclang pkgs.libv4l ];
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linuxHeaders}/include -I${pkgs.glibc.dev}/include";
        postInstall = ''
          mkdir -p $out/lib/systemd/system
          cat > $out/lib/systemd/system/bongo-modulator.service <<EOF
          [Unit]
          Description=Bongo Modulator daemon
          After=network.target

          [Service]
          ExecStart=$out/bin/bongo-modulator daemon
          Restart=on-failure

          [Install]
          WantedBy=multi-user.target
          EOF
        '';
      };

      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          rustToolchain
          pkgs.cargo-nextest
          pkgs.pkg-config
          pkgs.protobuf
          pkgs.llvmPackages.libclang
          pkgs.linuxHeaders
          pkgs.libv4l
        ];
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linuxHeaders}/include -I${pkgs.glibc.dev}/include";
      };

      checks.${system} = let
        devInputs = [
          rustToolchain
          pkgs.cargo-nextest
          pkgs.pkg-config
          pkgs.protobuf
          pkgs.llvmPackages.libclang
          pkgs.linuxHeaders
          pkgs.libv4l
        ];
        commonEnv = {
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linuxHeaders}/include -I${pkgs.glibc.dev}/include";
        };
        cargoArtifacts = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
      in {
        clippyCheck = rustPlatform.buildRustPackage {
          pname = "bongo-modulator-clippy";
          version = "0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = devInputs;
          buildInputs = [ pkgs.llvmPackages.libclang pkgs.libv4l ];
          inherit (commonEnv) LIBCLANG_PATH BINDGEN_EXTRA_CLANG_ARGS;
          CARGO_HOME = cargoArtifacts;
          doCheck = false;
          buildPhase = ''
            cargo clippy --offline -- -D warnings
          '';
          installPhase = ''
            touch $out
          '';
        };

        nextestCheck = rustPlatform.buildRustPackage {
          pname = "bongo-modulator-nextest";
          version = "0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = devInputs;
          buildInputs = [ pkgs.llvmPackages.libclang pkgs.libv4l ];
          inherit (commonEnv) LIBCLANG_PATH BINDGEN_EXTRA_CLANG_ARGS;
          CARGO_HOME = cargoArtifacts;
          doCheck = false;
          buildPhase = ''
            cargo nextest run --offline
          '';
          installPhase = ''
            touch $out
          '';
        };
      };

      clippyCheck = self.checks.${system}.clippyCheck;
      nextestCheck = self.checks.${system}.nextestCheck;
    };
}
