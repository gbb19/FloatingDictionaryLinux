{
  description = "Rust dev environment with OpenSSL";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    {
      devShells.x86_64-linux.default =
        let
          pkgs = import nixpkgs { system = "x86_64-linux"; };
        in
        pkgs.mkShell {
          buildInputs = [
            pkgs.rustc
            pkgs.cargo
            pkgs.pkg-config
            pkgs.leptonica
            pkgs.clang
            pkgs.llvmPackages.libclang
            pkgs.openssl
            pkgs.gcc
          ];
        };
    };
}
