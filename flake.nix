{
  description = "bongo-modulator dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rustToolchain = pkgs.rust-bin.stable.latest.default;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
    in {
      packages.${system}.default = rustPlatform.buildRustPackage {
        pname = "bongo-modulator";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
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
    };
}
