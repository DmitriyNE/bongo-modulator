{
  description = "bongo-modulator dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";

  outputs = { self, nixpkgs }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux; in {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = [ pkgs.rustc pkgs.cargo pkgs.pkg-config ];
      };
    };
}
