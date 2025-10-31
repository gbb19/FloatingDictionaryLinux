{
  description = "Rust dev environment with OpenSSL and Leptonica";

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
            pkgs.openssl
            pkgs.gcc
            pkgs.leptonica
            pkgs.clang
            pkgs.llvmPackages.libclang
          ];

          # ช่วยให้ bindgen หา libclang เจอแน่นอน
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          # บางโปรเจกต์ต้องการ headers ด้วย
          BINDGEN_EXTRA_CLANG_ARGS = ''
            -I${pkgs.llvmPackages.libclang.dev}/include
            -I${pkgs.libclang.dev}/include
          '';
        };
    };
}
