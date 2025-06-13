{
  description = "bongo-modulator dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "bongo-modulator";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [ pkgs.pkg-config pkgs.protobuf ];
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
          pkgs.rustc
          pkgs.cargo
          pkgs.clippy
          pkgs.rustfmt
          pkgs.cargo-nextest
          pkgs.pkg-config
          pkgs.protobuf
        ];
      };
    };
}
